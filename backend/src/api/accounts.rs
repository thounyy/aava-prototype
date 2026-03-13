use std::sync::Arc;

use axum::{extract::State, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::AppError;
use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/accounts/creator/create", post(create_creator_account))
        .route("/api/accounts/viewer/create", post(create_viewer_account))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountRequest {
    pub user_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountResponse {
    pub tx_digest: String,
}

async fn create_creator_account(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateCreatorAccountRequest>,
) -> Result<Json<CreateCreatorAccountResponse>, AppError> {
    info!("Creating creator account for user {}", request.user_handle);

    let tx = sui::creator::build_create_account_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        request.user_handle,
    )
    .await?;

    let result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(CreateCreatorAccountResponse {
        tx_digest: result.digest,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountRequest {
    pub user_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountResponse {
    pub user_handle: String,
    pub account_id: String,
}

async fn create_viewer_account(
    Json(request): Json<CreateViewerAccountRequest>,
) -> Result<Json<CreateViewerAccountResponse>, AppError> {
    info!("Creating account for user {}", request.user_handle);
    // TODO: call viewer::new_account, get the account object id from the tx effects
    todo!()
}
