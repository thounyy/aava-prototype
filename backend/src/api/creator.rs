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
pub struct AccountHandleRequest {
    pub account_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountResponse {
    pub tx_digest: String,
    pub account_id: String,
}

async fn create_creator_account(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AccountHandleRequest>,
) -> Result<Json<CreateCreatorAccountResponse>, AppError> {
    info!(
        "Creating creator account for account_handle {}",
        req.account_handle
    );
    let account_id = sui::read::derive_account_id(&req.account_handle)?;

    let tx = sui::creator::build_create_account_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        req.account_handle,
    )
    .await?;
    let tx_digest = tx.digest().to_string();

    let _result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(CreateCreatorAccountResponse {
        tx_digest,
        account_id: account_id.to_string(),
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
    Json(req): Json<AccountHandleRequest>,
) -> Result<Json<GetAccountResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&req.account_handle)?;
    let account = sui::creator::get_account(state.sui_client.clone(), account_id).await?;

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
    Json(req): Json<AccountHandleRequest>,
) -> Result<Json<AccountExistsResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&req.account_handle)?;
    let exists = sui::creator::account_exists(state.sui_client.clone(), account_id).await?;

    Ok(Json(AccountExistsResponse { exists }))
}
