use std::{sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use serde::{Deserialize, Serialize};
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::GetTransactionRequest;
use sui_sdk_types::Address;
use tracing::{info, warn};
use walrus_core::encoding;

use crate::enclave;
use crate::sui;
use crate::walrus;
use crate::AppState;

// From walrus-sui
// Keep in sync with the same constant in `contracts/walrus/sources/system/system_state_inner.move`.
// The storage unit is used in doc comments for CLI arguments in the files
// `crates/walrus-service/bin/deploy.rs` and `crates/walrus-service/bin/node.rs`.
// Change the unit there if it changes.
/// The number of bytes per storage unit.
pub const BYTES_PER_UNIT_SIZE: u64 = 1_024 * 1_024; // 1 MiB

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/streams/start", post(start_stream))
        .route("/api/streams/end/init", post(init_end_stream))
        .route("/api/streams/end/finalize", post(finalize_end_stream))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartRequest {
    pub account_id: String,
    pub sender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartResponse {
    pub tx_digest: String,
    pub tx_bytes_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndInitRequest {
    pub stream_id: String,
    pub account_id: String,
    pub sender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndInitResponse {
    pub stream_id: String,
    pub sessions_count: u64,
    pub tx_digest: String,
    pub tx_bytes_b64: String,
    pub blob_id_b64: String,
    pub payload_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndFinalizeRequest {
    pub stream_id: String,
    pub account_id: String,
    pub sender: String,
    pub end_tx_digest: String,
    pub blob_id_b64: String,
    pub payload_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndFinalizeResponse {
    pub stream_id: String,
    pub action: String,
    pub tx_digest: String,
    pub tx_bytes_b64: String,
}

async fn start_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StreamStartRequest>,
) -> Result<Json<StreamStartResponse>, (StatusCode, String)> {
    info!(
        "Building create_stream tx for account {}",
        request.account_id
    );

    let account_id: Address = request.account_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid account_id `{}`: {e}", request.account_id),
        )
    })?;
    let sender: Address = request.sender.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid sender `{}`: {e}", request.sender),
        )
    })?;

    let tx =
        sui::creator::build_create_stream_tx(state.sui_client.clone(), sender, account_id).await?;
    let tx_digest = tx.digest().to_string();
    let tx_bytes_b64 = STANDARD.encode(bcs::to_bytes(&tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize create_stream tx bytes: {e}"),
        )
    })?);

    Ok(Json(StreamStartResponse {
        tx_digest,
        tx_bytes_b64,
    }))
}

async fn init_end_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StreamEndInitRequest>,
) -> Result<Json<StreamEndInitResponse>, (StatusCode, String)> {
    info!(
        "Ending stream {} - calling enclave to batch sessions and generate attestation",
        request.stream_id
    );

    // Step 1: Fetch attested sessions from enclave
    let (data, signature, timestamp_ms) =
        enclave::stream::fetch_signed_dataset(&request.stream_id).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndInitResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
            tx_digest: String::new(),
            tx_bytes_b64: String::new(),
            blob_id_b64: String::new(),
            payload_b64: String::new(),
        }));
    }

    // Step 2: Verify signature + register blob on Sui (atomic tx)
    let payload = serde_json::to_vec(&data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize Walrus payload: {e}"),
        )
    })?;

    let encoded_size = encoding::encoded_blob_length_for_n_shards(
        data.n_shards,
        data.unencoded_size,
        data.encoding_type,
    )
    .ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Cannot compute Walrus encoded size".to_string(),
        )
    })?;

    let price_per_unit_size =
        sui::read::fetch_walrus_price_per_unit_size(state.sui_client.clone()).await?;

    let price_for_encoded_length =
        encoded_size.div_ceil(BYTES_PER_UNIT_SIZE) * price_per_unit_size * 53u64;
    let account_id: Address = request.account_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid account_id `{}`: {e}", request.account_id),
        )
    })?;
    let sender: Address = request.sender.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid sender `{}`: {e}", request.sender),
        )
    })?;

    let tx = sui::creator::build_verify_and_store_blob_tx(
        state.sui_client.clone(),
        sender,
        account_id,
        price_for_encoded_length,
        &request.stream_id,
        timestamp_ms,
        &signature,
        &data.blob_id,
        &data.root_hash,
        data.unencoded_size,
        data.encoding_type.into(),
        encoded_size,
        true,
    )
    .await?;
    let tx_digest = tx.digest().to_string().clone();
    let tx_bytes_b64 = STANDARD.encode(bcs::to_bytes(&tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize tx bytes: {e}"),
        )
    })?);

    info!(
        "Prepared stream end tx for stream {}: tx_digest={}",
        request.stream_id.clone(),
        tx_digest.clone()
    );

    Ok(Json(StreamEndInitResponse {
        stream_id: request.stream_id,
        sessions_count: data.sessions_count,
        tx_digest,
        tx_bytes_b64,
        blob_id_b64: URL_SAFE_NO_PAD.encode(&data.blob_id),
        payload_b64: URL_SAFE_NO_PAD.encode(payload),
    }))
}

async fn finalize_end_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StreamEndFinalizeRequest>,
) -> Result<Json<StreamEndFinalizeResponse>, (StatusCode, String)> {
    let account_id: Address = request.account_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid account_id `{}`: {e}", request.account_id),
        )
    })?;
    let sender: Address = request.sender.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid sender `{}`: {e}", request.sender),
        )
    })?;
    let payload = URL_SAFE_NO_PAD.decode(&request.payload_b64).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid payload_b64, expected base64url: {e}"),
        )
    })?;

    info!(
        "Finalizing stream {} from end tx {}",
        request.end_tx_digest, request.stream_id
    );

    let mut tx_opt = None;
    for _ in 0..120 {
        let tx_request = GetTransactionRequest::default()
            .with_digest(&request.end_tx_digest)
            .with_read_mask(FieldMask::from_str("*"));
        match state
            .sui_client
            .as_ref()
            .clone()
            .ledger_client()
            .get_transaction(tx_request)
            .await
        {
            Ok(response) => {
                let tx = response.into_inner().transaction().clone();
                if tx.checkpoint_opt().is_none() {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }

                let status = tx.effects().status();
                if !status.success() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "Transaction {} failed for stream {}: {:?}",
                            request.end_tx_digest, request.stream_id, status
                        ),
                    ));
                }
                tx_opt = Some(tx);
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }
    let tx = tx_opt.ok_or_else(|| {
        (
            StatusCode::GATEWAY_TIMEOUT,
            format!(
                "Timed out waiting for transaction {} (stream {})",
                request.end_tx_digest, request.stream_id
            ),
        )
    })?;

    let object_id = tx
        .effects()
        .changed_objects()
        .iter()
        .find(|obj| obj.object_type().contains("Blob"))
        .map(|obj| obj.object_id().to_string())
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                format!(
                    "Transaction {} succeeded but no Blob object found for stream {}",
                    request.end_tx_digest, request.stream_id
                ),
            )
        })?;

    let (finalize_action, finalize_tx) =
        match walrus::blob::upload_dataset(&object_id, &request.blob_id_b64, payload).await {
            Ok(relay_response) => {
                let tx = sui::creator::build_certify_blob_tx(
                    state.sui_client.clone(),
                    sender,
                    account_id,
                    &request.stream_id,
                    &relay_response.confirmation_certificate,
                )
                .await?;
                ("certify_blob".to_string(), tx)
            }
            Err(e) => {
                // TODO: keep data in separate db
                warn!(
                    "Walrus upload failed after successful tx {} for stream {}: {}",
                    request.end_tx_digest, request.stream_id, e
                );
                let tx = sui::creator::build_destroy_blob_tx(
                    state.sui_client.clone(),
                    sender,
                    account_id,
                    &request.stream_id,
                )
                .await?;
                ("destroy_blob".to_string(), tx)
            }
        };

    // TODO: assess how to handle cleanup in case of failure
    enclave::stream::cleanup_dataset(&request.stream_id).await?;

    let finalize_tx_digest = finalize_tx.digest().to_string();
    let finalize_tx_bytes_b64 = STANDARD.encode(bcs::to_bytes(&finalize_tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize finalize tx bytes: {e}"),
        )
    })?);

    Ok(Json(StreamEndFinalizeResponse {
        stream_id: request.stream_id,
        action: finalize_action,
        tx_digest: finalize_tx_digest,
        tx_bytes_b64: finalize_tx_bytes_b64,
    }))
}
