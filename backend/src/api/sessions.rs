use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::session::*;
use crate::tee;

pub fn create_router() -> Router<DbPool> {
    Router::new()
        .route("/api/sessions/open", post(open_session))
        .route("/api/sessions/terminate", post(terminate_session))
}

async fn open_session(
    State(_db): State<DbPool>,
    Json(request): Json<OpenSessionRequest>,
) -> Result<Json<OpenSessionResponse>, (StatusCode, String)> {
    info!(
        "Opening session for viewer {} on stream {}",
        request.viewer_id, request.stream_id
    );

    // Forward request to enclave - enclave writes directly to DB
    let enclave_response = tee::create_session(&request.viewer_id, &request.stream_id)
        .await
        .map_err(|e| {
            error!("TEE error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE error: {}", e),
            )
        })?;

    info!(
        "Session {} created successfully",
        enclave_response.session_id
    );

    Ok(Json(OpenSessionResponse {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
    }))
}

async fn terminate_session(
    State(_db): State<DbPool>,
    Json(request): Json<TerminateSessionRequest>,
) -> Result<Json<TerminateSessionResponse>, (StatusCode, String)> {
    info!("Terminating session {}", request.session_id);

    // Forward request to enclave - enclave updates DB directly
    let enclave_response = tee::terminate_session(&request.session_id)
        .await
        .map_err(|e| {
            error!("TEE error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE error: {}", e),
            )
        })?;

    info!(
        "Session {} terminated successfully",
        enclave_response.session_id
    );

    Ok(Json(TerminateSessionResponse {
        session_id: enclave_response.session_id,
        status: SessionStatus::Completed,
    }))
}
