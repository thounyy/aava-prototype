use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::enclave::error::EnclaveError;

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveOpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveCloseSessionResponse {
    pub session_id: String,
    pub status: String,
}

fn enclave_url() -> String {
    std::env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

fn enclave_internal_token() -> Result<String, EnclaveError> {
    std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| EnclaveError::ParseError("Missing ENCLAVE_INTERNAL_TOKEN".into()))
}

/// Create a session via the enclave.
pub async fn open_session(
    viewer_id: &str,
    stream_id: &str,
) -> Result<EnclaveOpenSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{url}/internal/sessions/open"))
        .header("X-Internal-Token", token)
        .json(&serde_json::json!({
            "viewer_id": viewer_id,
            "stream_id": stream_id,
        }))
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

    let parsed: EnclaveOpenSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    info!("Session {} opened successfully", parsed.session_id);
    Ok(parsed)
}

/// Close a session via the enclave.
pub async fn close_session(session_id: &str) -> Result<EnclaveCloseSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{url}/internal/sessions/close"))
        .header("X-Internal-Token", token)
        .json(&serde_json::json!({ "session_id": session_id }))
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

    let parsed: EnclaveCloseSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    info!("Session {} closed successfully", parsed.session_id);
    Ok(parsed)
}
