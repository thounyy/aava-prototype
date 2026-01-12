use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::database::DbPool;

pub fn create_router() -> Router<DbPool> {
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
    pub walrus_url: Option<String>,
    pub status: String,
}

// Enclave response types for deserializing the signed session data
#[derive(Debug, Deserialize)]
struct EnclaveEndStreamResponse {
    response: EnclaveIntentMessage,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct EnclaveIntentMessage {
    #[allow(dead_code)]
    intent: u8,
    #[allow(dead_code)]
    timestamp_ms: u64,
    data: EnclaveStreamData,
}

#[derive(Debug, Deserialize)]
struct EnclaveStreamData {
    #[allow(dead_code)]
    stream_id: String,
    #[allow(dead_code)]
    sessions: Vec<EnclaveSessionData>,
    sessions_count: u64,
    data_hash: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
    State(_db): State<DbPool>,
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
    State(_db): State<DbPool>,
    Json(request): Json<StreamEndRequest>,
) -> Result<Json<StreamEndResponse>, (StatusCode, String)> {
    info!(
        "Ending stream {} - calling enclave to batch sessions and generate attestation",
        request.stream_id
    );

    // Step 1: Fetch attested sessions from enclave
    let (sessions_count, data_hash, signature) =
        fetch_signed_sessions_from_enclave(&request.stream_id).await?;

    if sessions_count == 0 {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
            walrus_url: None,
            status: "completed".to_string(),
        }));
    }

    // Step 2: Upload to Walrus
    let walrus_url =
        upload_dataset_to_walrus(&request.stream_id, sessions_count, &data_hash).await?;

    // Step 3: Publish hash to Sui
    publish_hash_to_sui(&request.stream_id, &walrus_url, &data_hash, &signature).await?;

    // Step 4: Cleanup stream data from Redis after successful Walrus upload
    cleanup_stream_from_enclave(&request.stream_id).await?;

    info!(
        "Stream {} ended: {} sessions processed, attested by enclave, uploaded to Walrus, and cleaned up",
        request.stream_id, sessions_count
    );

    Ok(Json(StreamEndResponse {
        stream_id: request.stream_id,
        sessions_count,
        walrus_url,
        status: "completed".to_string(),
    }))
}

// === Helper functions ===

/// Fetch attested session data from the enclave
///
/// Returns (sessions_count, data_hash, signature)
async fn fetch_signed_sessions_from_enclave(
    stream_id: &str,
) -> Result<(u64, String, String), (StatusCode, String)> {
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

    let sessions_count = enclave_response.response.data.sessions_count;
    let data_hash = enclave_response.response.data.data_hash;
    let signature = enclave_response.signature;

    info!(
        "Received {} sessions from enclave with attestation (hash: {}, signature: {})",
        sessions_count,
        data_hash,
        &signature[..16] // Show first 16 chars of signature
    );

    Ok((sessions_count, data_hash, signature))
}

/// Upload session data to Walrus (placeholder)
async fn upload_dataset_to_walrus(
    stream_id: &str,
    sessions_count: u64,
    data_hash: &str,
) -> Result<Option<String>, (StatusCode, String)> {
    info!(
        "Session data ready for Walrus upload: {} sessions, hash: {}",
        sessions_count, data_hash
    );

    // TODO: Real Walrus implementation
    // - Upload session data + attestation to Walrus
    // - Get content hash and URL
    // Example:
    // let walrus_result = walrus::upload(&session_data, &signature).await?;
    // let walrus_url = walrus_result.url;

    warn!("[PLACEHOLDER] Publishing to Walrus - not implemented");
    Ok(Some(format!(
        "walrus://placeholder/{}/sessions.json",
        stream_id
    )))
}

/// Publish proof to Sui blockchain (placeholder)
async fn publish_hash_to_sui(
    _stream_id: &str,
    _walrus_url: &Option<String>,
    _data_hash: &str,
    _signature: &str,
) -> Result<(), (StatusCode, String)> {
    // TODO: Real Sui implementation
    // - Submit proof transaction to Sui
    // - Include enclave signature and data hash for verification
    // - Wait for transaction confirmation
    // Example:
    // let tx_result = sui::publish_stream_proof(
    //     stream_id,
    //     proof_hash,
    //     walrus_url,
    //     data_hash,
    //     &signature
    // ).await?;

    warn!("[PLACEHOLDER] Publishing proof to Sui - not implemented");
    Ok(())
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