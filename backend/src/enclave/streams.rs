use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

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
pub struct EnclaveStreamData {
    pub stream_id: String,
    pub sessions: Vec<EnclaveSessionData>,
    pub sessions_count: u64,
    pub data_hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveSessionData {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
}

/// Fetch attested session data from the enclave.
///
/// Returns (data, signature).
pub async fn fetch_signed_sessions_from_enclave(
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
        &signature[..16]
    );

    Ok((enclave_response.response.data, signature))
}

/// Cleanup stream data from Redis after successful Walrus upload.
pub async fn cleanup_stream_from_enclave(stream_id: &str) -> Result<(), (StatusCode, String)> {
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

