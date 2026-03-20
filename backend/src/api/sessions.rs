use std::sync::Arc;

use axum::{response::Json, routing::post, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::enclave;
use crate::error::AppError;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/sessions/open", post(open_session))
        .route("/api/sessions/close", post(close_session))
        .route("/api/sessions/get", post(get_session))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIdRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Open,
    Active,
    Closed,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub viewer_identifier: String,
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
}

async fn open_session(
    Json(req): Json<OpenSessionRequest>,
) -> Result<Json<OpenSessionResponse>, AppError> {
    info!(
        "Opening session for viewer {} on stream {}",
        req.viewer_identifier, req.stream_id
    );

    let enclave_response =
        enclave::session::open_session(&req.viewer_identifier, &req.stream_id).await?;

    Ok(Json(OpenSessionResponse {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
        status: SessionStatus::Open,
        created_at: chrono::Utc::now(),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionResponse {
    pub session_id: String,
    pub status: SessionStatus,
}

async fn close_session(
    Json(req): Json<SessionIdRequest>,
) -> Result<Json<CloseSessionResponse>, AppError> {
    info!("Closing session {}", req.session_id);

    let enclave_response = enclave::session::close_session(&req.session_id).await?;

    Ok(Json(CloseSessionResponse {
        session_id: enclave_response.session_id,
        status: SessionStatus::Closed,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

async fn get_session(
    Json(req): Json<SessionIdRequest>,
) -> Result<Json<GetSessionResponse>, AppError> {
    info!("Getting session {}", req.session_id);

    let enclave_response = enclave::session::get_session(&req.session_id).await?;

    Ok(Json(GetSessionResponse {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
        status: enclave_response.status,
        created_at: enclave_response.created_at,
        updated_at: enclave_response.updated_at,
    }))
}