use std::{collections::HashMap, sync::Arc};

use axum::{extract::State, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::AppError;
use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/viewers/create", post(create_viewer_account))
        .route("/api/viewers/get", post(get_viewer_account))
        .route("/api/viewers/exists", post(viewer_account_exists))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerHandleRequest {
    pub viewer_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountResponse {
    pub tx_digest: String,
    pub viewer_id: String,
}

async fn create_viewer_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ViewerHandleRequest>,
) -> Result<Json<CreateViewerAccountResponse>, AppError> {
    info!(
        "Creating viewer account for viewer_handle {}",
        req.viewer_handle
    );
    let viewer_id = sui::read::derive_account_id(&req.viewer_handle)?;

    let tx = sui::viewer::build_create_account_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        req.viewer_handle,
    )
    .await?;
    let tx_digest = tx.digest().to_string();

    let _result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(CreateViewerAccountResponse {
        tx_digest,
        viewer_id: viewer_id.to_string(),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAccountResponse {
    pub object_id: String,
    pub handle: String,
    /// Claimed owner address (`viewer::Account.owner`), if any.
    pub addr: Option<String>,
    pub protocol: String,
    pub sanctions: Vec<sui::viewer::ViewerSanction>,
    pub metadata: HashMap<String, String>,
}

async fn get_viewer_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ViewerHandleRequest>,
) -> Result<Json<GetAccountResponse>, AppError> {
    let viewer_id = sui::read::derive_account_id(&req.viewer_handle)?;
    let account = sui::viewer::get_account(state.sui_client.clone(), viewer_id).await?;

    Ok(Json(GetAccountResponse {
        object_id: account.id,
        handle: account.handle,
        addr: account.owner,
        protocol: account.protocol,
        sanctions: account.sanctions,
        metadata: account.metadata,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountExistsResponse {
    pub exists: bool,
}

async fn viewer_account_exists(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ViewerHandleRequest>,
) -> Result<Json<AccountExistsResponse>, AppError> {
    let viewer_id = sui::read::derive_account_id(&req.viewer_handle)?;
    let exists = sui::viewer::account_exists(state.sui_client.clone(), viewer_id).await?;

    Ok(Json(AccountExistsResponse { exists }))
}
