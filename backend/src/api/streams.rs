use axum::{http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::sui::stream::{
    certify_blob_on_sui, delete_registered_blob, verify_and_register_dataset,
};
use crate::walrus::blobs::publish_dataset_to_walrus;

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

// Enclave response types for deserializing the signed session data
#[derive(Debug, Deserialize)]
struct EnclaveEndStreamResponse {
    response: EnclaveIntentMessage,
    signature: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct EnclaveIntentMessage {
    intent: u8,
    timestamp_ms: u64,
    data: EnclaveStreamData,
}

#[derive(Debug, Deserialize, Serialize)]
struct EnclaveStreamData {
    stream_id: String,
    sessions: Vec<EnclaveSessionData>,
    sessions_count: u64,
    data_hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct EnclaveSessionData {
    session_id: String,
    viewer_id: String,
    stream_id: String,
    status: String,
    created_at: String,
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
    let (data, signature) = fetch_signed_sessions_from_enclave(&request.stream_id).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
        }));
    }

    // Step 2: Verify signature + register blob on Sui (single tx placeholder)
    let (object_id, blob_id) =
        verify_and_register_dataset(&request.stream_id, &data.data_hash, &signature).await?;
    info!(
        "Sui tx submitted for stream {}: blob_id={}, object_id={}",
        request.stream_id, blob_id, object_id
    );

    // Step 3: Upload data to Walrus using the registered blob_id after Sui tx success
    let payload = serde_json::to_vec(&data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize Walrus payload: {e}"),
        )
    })?;

    let confirmation_certificate =
        match publish_dataset_to_walrus(&blob_id, &object_id, payload).await {
            Ok(result) => result,
            Err(e) => {
                let _ = delete_registered_blob(&object_id).await;
                return Err((
                    StatusCode::BAD_GATEWAY,
                    format!("Walrus publish error: {e}"),
                ));
            }
        };

    info!(
        "Walrus upload succeeded for stream {}: blob_id={}, blob_object_id={}",
        request.stream_id, blob_id, object_id
    );

    // Step 3b: Certify the blob on Sui using the confirmation certificate (placeholder).
    certify_blob_on_sui(&blob_id, &confirmation_certificate).await?;

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

// === Helper functions ===

/// Fetch attested session data from the enclave
///
/// Returns (sessions_count, data_hash, signature)
async fn fetch_signed_sessions_from_enclave(
    stream_id: &str,
) -> Result<(EnclaveStreamData, String), (StatusCode, String)> {
    let enclave_url =
        std::env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let request_body = serde_json::json!({
        "stream_id": stream_id,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/end_stream", enclave_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave at {}: {}", enclave_url, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE connection error: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE error: HTTP {} - {}", status, error_text),
        ));
    }

    let enclave_response: EnclaveEndStreamResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE response parsing error: {}", e),
        )
    })?;

    let signature = enclave_response.signature;

    info!(
        "Received {} sessions for stream {} from enclave with attestation (hash: {}, signature: {})",
        enclave_response.response.data.sessions_count,
        enclave_response.response.data.stream_id,
        enclave_response.response.data.data_hash,
        &signature[..16] // Show first 16 chars of signature
    );

    Ok((enclave_response.response.data, signature))
}

/// Cleanup stream data from Redis after successful Walrus upload
async fn cleanup_stream_from_enclave(stream_id: &str) -> Result<(), (StatusCode, String)> {
    let enclave_url =
        std::env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let request_body = serde_json::json!({
        "stream_id": stream_id,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/cleanup_stream", enclave_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave for cleanup: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE connection error: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!(
            "Enclave cleanup returned error status {}: {}",
            status, error_text
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE cleanup error: HTTP {} - {}", status, error_text),
        ));
    }

    let cleanup_response: serde_json::Value = response.json().await.map_err(|e| {
        error!("Failed to parse cleanup response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE response parsing error: {}", e),
        )
    })?;

    let deleted_count = cleanup_response["deleted_count"].as_u64().unwrap_or(0);
    info!(
        "Cleaned up {} stream data from Redis for stream {}",
        deleted_count, stream_id
    );

    Ok(())
}
