use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::AppError;
use crate::sui;
use crate::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/creators/:account_identifier",
            post(create_creator_account),
        )
        .route(
            "/api/viewers/:account_identifier",
            post(create_viewer_account),
        )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreatorAccountResponse {
    pub tx_digest: String,
    pub account_id: String,
}

async fn create_creator_account(
    State(state): State<Arc<AppState>>,
    Path(account_identifier): Path<String>,
) -> Result<Json<CreateCreatorAccountResponse>, AppError> {
    info!("Creating creator account for identifier {}", account_identifier);
    let account_id = sui::read::derive_account_id(&account_identifier)?;

    let tx = sui::creator::build_create_account_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        account_identifier,
    )
    .await?;

    let result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(CreateCreatorAccountResponse {
        tx_digest: result.digest,
        account_id: account_id.to_string(),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountResponse {
    pub account_identifier: String,
    pub account_id: String,
}

async fn create_viewer_account(
    Path(account_identifier): Path<String>,
) -> Result<Json<CreateViewerAccountResponse>, AppError> {
    info!("Creating viewer account for identifier {}", account_identifier);
    // TODO: call viewer::new_account, get the account object id from the tx effects
    let account_id = sui::read::derive_account_id(&account_identifier)?;
    Ok(Json(CreateViewerAccountResponse {
        account_identifier,
        account_id: account_id.to_string(),
    }))
}
