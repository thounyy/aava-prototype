use std::sync::Arc;

use axum::{
    extract::Path,
    response::Json,
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::enclave;
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

async fn open_session(
    Path((viewer_identifier, stream_id)): Path<(String, String)>,
) -> Result<Json<OpenSessionResponse>, AppError> {
    info!(
        "Opening session for viewer {} on stream {}",
        viewer_identifier, stream_id
    );

    let enclave_response = enclave::session::open_session(&viewer_identifier, &stream_id).await?;

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

    let enclave_response = enclave::session::close_session(&session_id).await?;

    Ok(Json(CloseSessionResponse {
        session_id: enclave_response.session_id,
        status: SessionStatus::Closed,
    }))
}
