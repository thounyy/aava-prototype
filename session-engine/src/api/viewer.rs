use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::viewer::*;
use crate::sui;
use crate::tee;

pub fn create_router() -> Router<DbPool> {
    Router::new()
        .route("/api/viewers/account/create", post(create_account))
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

// async fn check_permissions(
//     State(_db): State<DbPool>,
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
