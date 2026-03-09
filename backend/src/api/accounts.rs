use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;
use tracing::info;

use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/accounts/creator/create", post(create_creator_account))
        .route("/api/accounts/viewer/create", post(create_viewer_account))
    // .route("/api/account/get", get(get_account))
    // .route("/api/account/exists", get(account_exists))
    // .route("/api/sessions/open", post(open_session))
    // .route("/api/permissions/check", post(check_permissions))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountRequest {
    pub user_handle: String, // to define (could be platform user id)
    pub sender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountResponse {
    pub tx_digest: String,
    pub tx_bytes_b64: String,
}

async fn create_creator_account(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateCreatorAccountRequest>,
) -> Result<Json<CreateCreatorAccountResponse>, (StatusCode, String)> {
    info!("Creating creator account for user {}", request.user_handle);

    let sender: Address = request.sender.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid sender `{}`: {e}", request.sender),
        )
    })?;

    let tx = sui::creator::build_create_account_tx(
        state.sui_client.clone(),
        sender,
        request.user_handle,
    )
    .await?;
    let tx_digest = tx.digest().to_string();
    let tx_bytes_b64 = STANDARD.encode(bcs::to_bytes(&tx).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize create_account tx bytes: {e}"),
        )
    })?);

    Ok(Json(CreateCreatorAccountResponse {
        tx_digest,
        tx_bytes_b64,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountRequest {
    pub user_handle: String, // to define (could be platform user id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountResponse {
    pub user_handle: String,
    pub account_id: String,
}

async fn create_viewer_account(
    Json(request): Json<CreateViewerAccountRequest>,
) -> Result<Json<CreateViewerAccountResponse>, (StatusCode, String)> {
    info!("Creating account for user {}", request.user_handle);

    // call viewer::new_account

    // get the account object id from the tx effects

    todo!()
}

// async fn check_permissions(
//     Json(request): Json<SessionRequest>,
// ) -> Result<Json<PermissionCheck>, (StatusCode, String)> {
//     info!(
//         "Checking permissions for user {} on stream {}",
//         request.user_id, request.stream_id
//     );

//     // Check permissions from Sui blockchain (on-chain)
//     // Streams and subscriptions live on-chain, not in database
//     let permission_check = sui::check_stream_permission(&request.user_id, &request.stream_id)
//         .await
//         .map_err(|e| {
//             error!("Sui blockchain error: {}", e);
//             (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("Failed to check permissions: {}", e),
//             )
//         })?;

//     Ok(Json(permission_check))
// }
