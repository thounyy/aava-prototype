use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::viewer::*;
use crate::sui;
use crate::tee;

pub fn create_router() -> Router<DbPool> {
    Router::new()
        .route("/api/viewer/account/create", post(create_account))
        // .route("/api/account/get", get(get_account))
        // .route("/api/account/exists", get(account_exists))
        // .route("/api/sessions/open", post(open_session))
        // .route("/api/permissions/check", post(check_permissions))
}

async fn create_account(
    Json(request): Json<CreateViewerAccountRequest>,
) -> Result<Json<CreateViewerAccountResponse>, (StatusCode, String)> {
    info!("Creating account for user {}", request.user_handle);

    // call viewer::new_account

    // get the account object id from the tx effects

    todo!()
}