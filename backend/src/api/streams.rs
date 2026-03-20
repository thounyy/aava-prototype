use std::sync::Arc;

use axum::{extract::State, response::Json, routing::post, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::enclave;
use crate::error::AppError;
use crate::sui;
use crate::sui::constants::AAVA_PACKAGE;
use crate::sui::constants::WALRUS_PACKAGE;
use crate::walrus;
use crate::AppState;

// From walrus-sui — 1 MiB per storage unit.
pub const BYTES_PER_UNIT_SIZE: u64 = 1_024 * 1_024;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/streams/start", post(start_stream))
        .route("/api/streams/end", post(end_stream))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartRequest {
    pub creator_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartResponse {
    pub stream_id: String,
    pub tx_digest: String,
}

async fn start_stream(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StreamStartRequest>,
) -> Result<Json<StreamStartResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&req.creator_handle)?;
    info!(
        "Creating stream for creator_handle {} (derived account {})",
        req.creator_handle, account_id
    );

    let tx = sui::creator::build_create_stream_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        account_id,
    )
    .await?;

    let tx_digest = tx.digest().to_string();
    let tx_results = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;
    let stream_id = sui::read::find_object_id_from_tx_results(
        tx_results,
        &format!("{}::creator::Stream", AAVA_PACKAGE),
    )?;

    Ok(Json(StreamStartResponse {
        stream_id,
        tx_digest,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndRequest {
    pub creator_handle: String,
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndResponse {
    pub stream_id: String,
    pub sessions_count: u64,
    pub init_tx_digest: String,
    pub finalize_tx_digest: String,
}

/// 1. Fetch attested session data from enclave
/// 2. Build, sign & execute the verify_and_store_blob tx
/// 3. Upload blob payload to Walrus relay
/// 4. Build, sign & execute the certify (or destroy) blob tx
/// 5. Cleanup enclave state
async fn end_stream(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StreamEndRequest>,
) -> Result<Json<StreamEndResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&req.creator_handle)?;
    info!(
        "Ending stream {} for creator_handle {} (derived account {})",
        req.stream_id, req.creator_handle, account_id
    );

    // ── 1. Fetch Walrus system params ───────────────────────────────
    let (price_per_unit_size, n_shards) =
        sui::read::fetch_walrus_system_params(state.sui_client.clone()).await?;

    // ── 2. Fetch attested sessions from enclave ─────────────────────
    let (data, signature, timestamp_ms) =
        enclave::stream::fetch_signed_dataset(&req.stream_id, n_shards).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", req.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: req.stream_id,
            sessions_count: 0,
            init_tx_digest: String::new(),
            finalize_tx_digest: String::new(),
        }));
    }

    // ── 3. Prepare payload & tip ────────────────────────────────────
    let payload = serde_json::to_vec(&data.sessions)
        .map_err(|e| AppError::Internal(format!("Failed to serialize Walrus payload: {e}")))?;

    let price_for_encoded_length =
        data.encoded_size.div_ceil(BYTES_PER_UNIT_SIZE) * price_per_unit_size * 53; // 53 epochs is 2y, maximum allowed

    let tip_config = walrus::tip::get_tip_config(payload.clone(), data.encoded_size).await?;

    // ── 4. Build & execute verify_and_store_blob tx ─────────────────
    let sender = sui::executor::wallet_address();

    let register_tx = sui::creator::build_verify_and_store_blob_tx(
        state.sui_client.clone(),
        sender,
        account_id,
        price_for_encoded_length,
        &req.stream_id,
        timestamp_ms,
        &signature,
        &data.blob_id,
        &data.root_hash,
        data.unencoded_size,
        data.encoding_type.into(),
        data.encoded_size,
        true,
        tip_config.auth_payload,
        tip_config.tip_payment,
    )
    .await?;

    let init_tx_digest = register_tx.digest().to_string();
    let init_results =
        sui::executor::sign_and_execute(state.sui_client.clone(), register_tx).await?;

    info!(
        "Register blob tx executed for stream {}: {}",
        req.stream_id, init_tx_digest
    );

    // ── 5. Find the Blob object from tx effects ─────────────────────
    let blob_object_id = sui::read::find_object_id_from_tx_results(
        init_results,
        &format!("{}::blob::Blob", WALRUS_PACKAGE),
    )?;

    let blob_id_b64 = URL_SAFE_NO_PAD.encode(&data.blob_id);

    // ── 6. Upload to Walrus relay, then certify or destroy ──────────
    let finalize_tx = match walrus::blob::upload_dataset(
        &blob_object_id,
        &blob_id_b64,
        payload,
        Some(&init_tx_digest),
        tip_config.nonce_b64.as_deref(),
    )
    .await
    {
        Ok(relay_response) => {
            sui::creator::build_certify_blob_tx(
                state.sui_client.clone(),
                sender,
                account_id,
                &req.stream_id,
                &relay_response.certificate,
            )
            .await?
        }
        Err(e) => {
            warn!(
                "Walrus upload failed after successful tx {} for stream {}: {e}",
                init_tx_digest, req.stream_id,
            );
            sui::creator::build_destroy_blob_tx(
                state.sui_client.clone(),
                sender,
                account_id,
                &req.stream_id,
            )
            .await?
        }
    };

    let finalize_tx_digest = finalize_tx.digest().to_string();
    sui::executor::sign_and_execute(state.sui_client.clone(), finalize_tx).await?;

    info!(
        "Finalize tx executed for stream {}: {}",
        req.stream_id, finalize_tx_digest
    );

    enclave::stream::cleanup_dataset(&req.stream_id).await?;

    Ok(Json(StreamEndResponse {
        stream_id: req.stream_id,
        sessions_count: data.sessions_count,
        init_tx_digest,
        finalize_tx_digest,
    }))
}

// ── Helpers ─────────────────────────────────────────────────────────
