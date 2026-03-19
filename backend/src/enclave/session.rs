use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::enclave::error::EnclaveError;

fn enclave_url() -> String {
    std::env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

fn enclave_internal_token() -> Result<String, EnclaveError> {
    std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| EnclaveError::ParseError("Missing ENCLAVE_INTERNAL_TOKEN".into()))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveSessionIdRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveOpenSessionRequest {
    pub viewer_id: String,
    pub stream_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveOpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
}

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
        .json(&EnclaveOpenSessionRequest {
            viewer_id: viewer_id.to_string(),
            stream_id: stream_id.to_string(),
        })
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

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveFlagSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn flag_session(
    session_id: &str,
) -> Result<EnclaveFlagSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{url}/internal/sessions/flag"))
        .header("X-Internal-Token", token)
        .json(&EnclaveSessionIdRequest { session_id: session_id.to_string() })
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

    let parsed: EnclaveFlagSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    Ok(parsed)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveRevokeSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn revoke_session(
    session_id: &str,
) -> Result<EnclaveRevokeSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{url}/internal/sessions/revoke"))
        .header("X-Internal-Token", token)
        .json(&EnclaveSessionIdRequest { session_id: session_id.to_string() })
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

    let parsed: EnclaveRevokeSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    Ok(parsed)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveCloseSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn close_session(session_id: &str) -> Result<EnclaveCloseSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{url}/internal/sessions/close"))
        .header("X-Internal-Token", token)
        .json(&EnclaveSessionIdRequest { session_id: session_id.to_string() })
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

#[derive(Debug, Deserialize, Serialize)]
pub struct EnclaveGetSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn get_session(
    session_id: &str,
) -> Result<EnclaveGetSessionResponse, EnclaveError> {
    let url = enclave_url();
    let token = enclave_internal_token()?;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{url}/internal/sessions/get"))
        .header("X-Internal-Token", token)
        .json(&EnclaveSessionIdRequest { session_id: session_id.to_string() })
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

    let parsed: EnclaveGetSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {e}");
        EnclaveError::ParseError(e.to_string())
    })?;

    info!(
        "Session {} fetched successfully",
        parsed.session_id
    );
    Ok(parsed)
}
