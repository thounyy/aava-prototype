use std::{collections::HashMap, sync::Arc};

use axum::{extract::State, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::AppError;
use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/creators/create", post(create_creator_account))
        .route("/api/creators/get", post(get_creator_account))
        .route("/api/creators/exists", post(creator_account_exists))
}

// TODO: hardcode for prototype
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorHandleRequest {
    pub creator_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountResponse {
    pub tx_digest: String,
    pub creator_id: String,
}

async fn create_creator_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatorHandleRequest>,
) -> Result<Json<CreateCreatorAccountResponse>, AppError> {
    info!(
        "Creating creator account for creator_handle {}",
        req.creator_handle
    );
    let creator_id = sui::read::derive_account_id(&req.creator_handle)?;

    let tx = sui::creator::build_create_account_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        req.creator_handle,
    )
    .await?;
    let tx_digest = tx.digest().to_string();

    let _result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(CreateCreatorAccountResponse {
        tx_digest,
        creator_id: creator_id.to_string(),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAccountResponse {
    pub handle: String,
    pub members: Vec<String>,
    pub metadata: HashMap<String, String>,
}

async fn get_creator_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatorHandleRequest>,
) -> Result<Json<GetAccountResponse>, AppError> {
    let creator_id = sui::read::derive_account_id(&req.creator_handle)?;
    let account = sui::creator::get_account(state.sui_client.clone(), creator_id).await?;

    Ok(Json(GetAccountResponse {
        handle: account.handle,
        members: account.members,
        metadata: account.metadata,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountExistsResponse {
    pub exists: bool,
}

async fn creator_account_exists(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatorHandleRequest>,
) -> Result<Json<AccountExistsResponse>, AppError> {
    let creator_id = sui::read::derive_account_id(&req.creator_handle)?;
    let exists = sui::creator::account_exists(state.sui_client.clone(), creator_id).await?;

    Ok(Json(AccountExistsResponse { exists }))
}
