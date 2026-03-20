use std::sync::Arc;

use axum::{response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::enclave;
use crate::error::AppError;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/actions/flag", post(flag_session))
        .route("/api/actions/revoke", post(revoke_session))
        .route("/api/actions/status", post(session_status))
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
pub struct FlagSessionResponse {
    pub session_id: String,
    pub status: String,
}

async fn flag_session(
    Json(req): Json<SessionIdRequest>,
) -> Result<Json<FlagSessionResponse>, AppError> {
    info!("Flagging session {}", req.session_id);

    let enclave_response = enclave::session::flag_session(&req.session_id).await?;
    Ok(Json(FlagSessionResponse {
        session_id: enclave_response.session_id,
        status: enclave_response.status,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSessionResponse {
    pub session_id: String,
    pub status: String,
}

async fn revoke_session(
    Json(req): Json<SessionIdRequest>,
) -> Result<Json<RevokeSessionResponse>, AppError> {
    info!("Revoking session {}", req.session_id);

    let enclave_response = enclave::session::revoke_session(&req.session_id).await?;
    Ok(Json(RevokeSessionResponse {
        session_id: enclave_response.session_id,
        status: enclave_response.status,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusResponse {
    pub status: String,
}

async fn session_status(
    Json(req): Json<SessionIdRequest>,
) -> Result<Json<SessionStatusResponse>, AppError> {
    info!("Getting session status {}", req.session_id);

    let enclave_response = enclave::session::get_session(&req.session_id).await?;

    Ok(Json(SessionStatusResponse {
        status: enclave_response.status,
    }))
}

