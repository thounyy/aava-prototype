use std::sync::Arc;

use axum::{
    extract::Path,
    response::Json,
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};
use uuid::Uuid;

use crate::enclave::error::EnclaveError;
use crate::error::AppError;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/viewers/{viewer_identifier}/streams/{stream_id}/sessions",
            post(open_session),
        )
        .route(
            "/api/viewers/{viewer_identifier}/sessions/{session_id}/close",
            post(close_session),
        )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionResponse {
    pub session_id: String,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Open,
    Active,
    Closed,
    Error(String),
}

/// Response from the enclave for session creation
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveOpenSessionResponse {
    session_id: String,
    viewer_id: String,
    stream_id: String,
    status: String,
}

/// Response from the enclave for session termination
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveCloseSessionResponse {
    session_id: String,
    status: String,
}

fn enclave_internal_token() -> Result<String, EnclaveError> {
    env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| EnclaveError::ParseError("Missing ENCLAVE_INTERNAL_TOKEN".into()))
}

async fn open_session(
    Path((viewer_identifier, stream_id)): Path<(String, String)>,
) -> Result<Json<OpenSessionResponse>, AppError> {
    info!(
        "Opening session for viewer {} on stream {}",
        viewer_identifier, stream_id
    );

    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let token = enclave_internal_token()?;

    let request_body = serde_json::json!({
        "viewer_id": viewer_identifier,
        "stream_id": stream_id,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/internal/sessions/open", enclave_url))
        .header("X-Internal-Token", token)
        .json(&request_body)
        .send()
        .await
        .map_err(EnclaveError::ConnectionFailed)?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err(EnclaveError::ApiError {
            status: status.as_u16(),
            body: error_text,
        }
        .into());
    }

    let enclave_response: EnclaveOpenSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        EnclaveError::ParseError(e.to_string())
    })?;

    info!(
        "Session {} opened successfully",
        enclave_response.session_id
    );

    Ok(Json(OpenSessionResponse {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
        status: SessionStatus::Open,
        created_at: chrono::Utc::now(),
    }))
}

async fn close_session(
    Path((viewer_identifier, session_id)): Path<(String, String)>,
) -> Result<Json<CloseSessionResponse>, AppError> {
    info!(
        "Closing session {} for viewer {}",
        session_id, viewer_identifier
    );

    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let token = enclave_internal_token()?;

    let request_body = serde_json::json!({
        "session_id": session_id,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/internal/sessions/close", enclave_url))
        .header("X-Internal-Token", token)
        .json(&request_body)
        .send()
        .await
        .map_err(EnclaveError::ConnectionFailed)?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err(EnclaveError::ApiError {
            status: status.as_u16(),
            body: error_text,
        }
        .into());
    }

    let enclave_response: EnclaveCloseSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        EnclaveError::ParseError(e.to_string())
    })?;

    info!(
        "Session {} closed successfully",
        enclave_response.session_id
    );

    Ok(Json(CloseSessionResponse {
        session_id: enclave_response.session_id,
        status: SessionStatus::Closed,
    }))
}
