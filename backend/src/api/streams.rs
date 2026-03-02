use std::{str::FromStr, sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::GetTransactionRequest;
use sui_rpc::Client;
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
        .route("/api/streams/end", post(end_stream))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartRequest {
    // fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartResponse {
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndRequest {
    pub stream_id: String,
    pub account_id: String,
    pub sender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndResponse {
    pub stream_id: String,
    pub sessions_count: u64,
    pub tx_digest: String,
    pub tx_bytes_b64: String,
}

/// Start a stream
///
/// Placeholder for Sui blockchain call to start a stream.
/// In production, this would:
/// - Call Sui to mark stream as active
/// - Update stream object on-chain
/// - Emit events for stream start
async fn start_stream(
    Json(_request): Json<StreamStartRequest>,
) -> Result<Json<StreamStartResponse>, (StatusCode, String)> {
    info!("Starting stream");

    // TODO: Real Sui implementation
    // - Call Sui Move function to start stream
    // - Update stream object status on-chain
    // - Emit stream_start event

    warn!("[PLACEHOLDER] Stream start - Sui call not implemented");

    Ok(Json(StreamStartResponse {
        stream_id: "stream_id".to_string(),
    }))
}

/// End a stream
///
/// This endpoint:
/// 1. Calls enclave to query Redis and generate cryptographic attestation
/// 2. Publishes attested session data to Walrus (decentralized storage)
/// 3. Publishes hash to Sui blockchain
/// 4. Cleans up sessions from Redis after successful upload
async fn end_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StreamEndRequest>,
) -> Result<Json<StreamEndResponse>, (StatusCode, String)> {
    info!(
        "Ending stream {} - calling enclave to batch sessions and generate attestation",
        request.stream_id
    );

    // Step 1: Fetch attested sessions from enclave
    let (data, signature, timestamp_ms) =
        enclave::stream::fetch_signed_dataset(&request.stream_id).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
            tx_digest: String::new(),
            tx_bytes_b64: String::new(),
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

    let price_per_unit_size = 1; // TODO: fetch price from system object

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

    let tx = sui::stream::build_end_stream_tx(
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
    let tx_bytes_b64 = URL_SAFE_NO_PAD.encode(bcs::to_bytes(&tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize tx bytes: {e}"),
        )
    })?);
    
    let watcher_tx_digest = tx_digest.clone();
    let watcher_stream_id = request.stream_id.clone();
    tokio::spawn(async move {
        watch_end_stream_tx(
            state.sui_client.clone(),
            watcher_tx_digest,
            watcher_stream_id,
            URL_SAFE_NO_PAD.encode(&data.blob_id),
            payload,
        )
        .await;
    });

    info!(
        "Prepared stream end tx for stream {}: tx_digest={}",
        request.stream_id.clone(), tx_digest.clone()
    );

    Ok(Json(StreamEndResponse {
        stream_id: request.stream_id,
        sessions_count: data.sessions_count,
        tx_digest,
        tx_bytes_b64,
    }))
}

async fn watch_end_stream_tx(
    client: Arc<Client>,
    tx_digest: String,
    stream_id: String,
    blob_id_b64: String,
    payload: Vec<u8>,
) {
    let mut client = client.as_ref().clone();
    info!(
        "Watching transaction {} for stream {}",
        tx_digest, stream_id
    );

    for _ in 0..120 {
        let request = GetTransactionRequest::default()
            .with_digest(&tx_digest)
            .with_read_mask(FieldMask::from_str("*"));
        match client.ledger_client().get_transaction(request).await {
            Ok(response) => {
                let tx = response.into_inner().transaction().clone();
                if tx.checkpoint_opt().is_none() {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                let status = tx.effects().status();
                if !status.success() {
                    warn!(
                        "Transaction {} failed for stream {}: {:?}",
                        tx_digest, stream_id, status
                    );
                    return; // TODO: handle failed transaction
                }

                let object_id = tx
                    .effects()
                    .changed_objects()
                    .iter()
                    .find(|obj| obj.object_type().contains("Blob"))
                    .map(|obj| obj.object_id().to_string());
                let Some(object_id) = object_id else {
                    warn!(
                        "Transaction {} succeeded but no Blob object found for stream {}",
                        tx_digest, stream_id
                    );
                    return; // TODO: handle object id not found
                };

                match walrus::blob::upload_dataset(&object_id, &blob_id_b64, payload).await {
                    Ok(confirmation_certificate) => {
                        let _ = sui::stream::certify_and_store_blob(
                            &object_id,
                            &blob_id_b64,
                            &confirmation_certificate,
                        )
                        .await;
                        let _ = enclave::stream::cleanup_dataset(&stream_id).await;
                        info!(
                            "Stream {} finalized (tx={}, object_id={})",
                            stream_id, tx_digest, object_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Walrus upload failed after successful tx {} for stream {}: {}",
                            tx_digest, stream_id, e
                        );
                        let _ = sui::stream::destroy_blob(&object_id).await;
                    }
                }
                return;
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }

    warn!(
        "Timed out waiting for transaction {} (stream {})",
        tx_digest, stream_id
    );
}
