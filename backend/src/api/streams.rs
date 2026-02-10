use axum::{http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::enclave::streams::{cleanup_stream_from_enclave, fetch_signed_sessions_from_enclave};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use crate::sui::stream::{
    certify_and_store_blob_on_sui, destroy_blob_on_sui, flag_stream_as_invalid_on_sui,
    verify_and_register_blob_on_sui,
};
use crate::walrus::blob::upload_dataset;

pub fn create_router() -> Router {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndResponse {
    pub stream_id: String,
    pub sessions_count: u64,
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
    Json(request): Json<StreamEndRequest>,
) -> Result<Json<StreamEndResponse>, (StatusCode, String)> {
    info!(
        "Ending stream {} - calling enclave to batch sessions and generate attestation",
        request.stream_id
    );

    // Step 1: Fetch attested sessions from enclave
    let (data, signature, timestamp_ms) =
        fetch_signed_sessions_from_enclave(&request.stream_id).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
        }));
    }

    // Step 2: Verify signature + register blob on Sui (atomic tx)
    let object_id = match verify_and_register_blob_on_sui(
        &request.stream_id,
        &data.blob_id,
        timestamp_ms,
        &signature,
    )
    .await
    {
        Ok(result) => result,
        Err((status, message)) => {
            error!(
                "Sui verification/registration failed for stream {}: {}",
                request.stream_id, message
            );

            // TODO: Store flawed stream data in Postgres for quarantine/audit.
            let _ = flag_stream_as_invalid_on_sui(&request.stream_id).await;

            return Err((status, message));
        }
    };
    info!(
        "Sui tx submitted for stream {}: object_id={}, blob_id={}",
        request.stream_id, object_id, URL_SAFE_NO_PAD.encode(data.blob_id.as_ref())
    );

    // Step 3: Upload data to Walrus using the registered blob_id after Sui tx success
    let payload = serde_json::to_vec(&data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize Walrus payload: {e}"),
        )
    })?;

    let blob_id_b64 = URL_SAFE_NO_PAD.encode(data.blob_id.as_ref());
    let confirmation_certificate = match upload_dataset(&object_id, &blob_id_b64, payload).await {
        Ok(result) => result,
        Err(e) => {
            let _ = destroy_blob_on_sui(&object_id).await;
            return Err((
                StatusCode::BAD_GATEWAY,
                format!("Walrus publish error: {e}"),
            ));
        }
    };

    info!(
        "Walrus upload succeeded for stream {}: object_id={}, blob_id={}",
        request.stream_id, object_id, blob_id_b64
    );

    // Step 3b: Certify the blob on Sui using the confirmation certificate (placeholder).
    certify_and_store_blob_on_sui(&object_id, &blob_id_b64, &confirmation_certificate).await?;

    // Step 4: Cleanup stream data from Redis after successful Walrus upload
    cleanup_stream_from_enclave(&request.stream_id).await?;

    info!(
        "Stream {} ended: {} sessions processed, verified+registered on Sui, uploaded+certified on Walrus/Sui, and cleaned up",
        request.stream_id,
        data.sessions_count
    );

    Ok(Json(StreamEndResponse {
        stream_id: request.stream_id,
        sessions_count: data.sessions_count,
    }))
}
