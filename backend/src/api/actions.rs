use std::sync::Arc;

use axum::extract::State;
use axum::{response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;
use tracing::info;

use crate::enclave;
use crate::error::AppError;
use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/actions/warn", post(warn_session))
        .route("/api/actions/revoke", post(revoke_session))
        .route("/api/actions/status", post(session_status))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusResponse {
    pub status: String,
}

async fn session_status(
    Json(req): Json<SessionStatusRequest>,
) -> Result<Json<SessionStatusResponse>, AppError> {
    info!("Getting session status {}", req.session_id);

    let enclave_response = enclave::session::get_session(&req.session_id).await?;

    Ok(Json(SessionStatusResponse {
        status: enclave_response.status,
    }))
}

/// Body for warn / revoke: ties the session (from enclave) to the creator account that signs the chain tx.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSanctionRequest {
    pub session_id: String,
    pub creator_handle: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarnSessionResponse {
    pub session_id: String,
    pub status: String,
    pub tx_digest: String,
}

async fn warn_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SessionSanctionRequest>,
) -> Result<Json<WarnSessionResponse>, AppError> {
    info!("Warn session {}", req.session_id);

    let tx_digest =
        execute_creator_flag_session(&state, &req, sui::creator::SANCTION_KIND_WARN).await?;

    let enclave_response = enclave::session::warn_session(&req.session_id).await?;

    Ok(Json(WarnSessionResponse {
        session_id: enclave_response.session_id,
        status: enclave_response.status,
        tx_digest,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSessionResponse {
    pub session_id: String,
    pub status: String,
    pub tx_digest: String,
}

async fn revoke_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SessionSanctionRequest>,
) -> Result<Json<RevokeSessionResponse>, AppError> {
    info!("Revoke session {}", req.session_id);

    let tx_digest =
        execute_creator_flag_session(&state, &req, sui::creator::SANCTION_KIND_REVOKE).await?;

    let enclave_response = enclave::session::revoke_session(&req.session_id).await?;

    Ok(Json(RevokeSessionResponse {
        session_id: enclave_response.session_id,
        status: enclave_response.status,
        tx_digest,
    }))
}

/// Load session metadata from enclave, build `creator::flag_session`, sign & execute, return tx digest.
async fn execute_creator_flag_session(
    state: &AppState,
    req: &SessionSanctionRequest,
    kind: u8,
) -> Result<String, AppError> {
    let session = enclave::session::get_session(&req.session_id).await?;

    let viewer_id: Address = session
        .viewer_id
        .parse()
        .map_err(|e| AppError::BadRequest(format!("Invalid viewer_id from enclave: {e}")))?;

    let stream_id: Address = session
        .stream_id
        .parse()
        .map_err(|e| AppError::BadRequest(format!("Invalid stream_id from enclave: {e}")))?;

    let creator_id = sui::read::derive_account_id(&req.creator_handle)?;

    let reason = if req.reason.trim().is_empty() {
        match kind {
            sui::creator::SANCTION_KIND_WARN => "warned by creator",
            sui::creator::SANCTION_KIND_REVOKE => "revoked by creator",
            _ => "session sanction",
        }
        .to_string()
    } else {
        req.reason.clone()
    };

    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;

    let tx = sui::creator::build_flag_session_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        creator_id,
        viewer_id,
        req.session_id.clone(),
        stream_id,
        kind,
        reason,
        timestamp_ms,
    )
    .await?;

    let digest = tx.digest().to_string();
    sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(digest)
}
