use std::num::NonZeroU16;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use tracing::{error, info};
use walrus_core::EncodingType;

use crate::enclave::error::EnclaveError;

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
    pub blob_id: ByteBuf,
    pub root_hash: ByteBuf,
    pub n_shards: NonZeroU16,
    pub unencoded_size: u64,
    pub encoding_type: EncodingType,
    pub encoded_size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveSessionData {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
}

fn enclave_url() -> String {
    std::env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// Fetch attested session data from the enclave.
///
/// Returns (data, signature, timestamp_ms).
pub async fn fetch_signed_dataset(
    stream_id: &str,
    n_shards: u16,
) -> Result<(EnclaveStreamData, String, u64), EnclaveError> {
    let url = enclave_url();
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{url}/end_stream"))
        .json(&serde_json::json!({ "stream_id": stream_id, "n_shards": n_shards }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        error!("Enclave returned error {status}: {body}");
        return Err(EnclaveError::ApiError { status, body });
    }

    let parsed: EnclaveEndStreamResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    info!(
        "Received {} sessions for stream {} from enclave (blob_id: {}, sig: {}...)",
        parsed.response.data.sessions_count,
        parsed.response.data.stream_id,
        URL_SAFE_NO_PAD.encode(parsed.response.data.blob_id.as_ref()),
        &parsed.signature[..16.min(parsed.signature.len())],
    );

    Ok((
        parsed.response.data,
        parsed.signature,
        parsed.response.timestamp_ms,
    ))
}

/// Cleanup stream data from Redis after successful Walrus upload.
pub async fn cleanup_dataset(stream_id: &str) -> Result<(), EnclaveError> {
    let url = enclave_url();
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{url}/cleanup_stream"))
        .json(&serde_json::json!({ "stream_id": stream_id }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".into());
        error!("Enclave cleanup returned error {status}: {body}");
        return Err(EnclaveError::ApiError { status, body });
    }

    let cleanup_response: serde_json::Value = response.json().await.map_err(|e| {
        error!("Failed to parse cleanup response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    let deleted_count = cleanup_response["deleted_count"].as_u64().unwrap_or(0);
    info!("Cleaned up {deleted_count} Redis entries for stream {stream_id}");

    Ok(())
}
