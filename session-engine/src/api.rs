use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::*;
use crate::sui;
use crate::tee;

pub fn create_router() -> Router<DbPool> {
    Router::new()
        .route("/api/account/create", post(create_account))
        // .route("/api/account/get", get(get_account))
        // .route("/api/account/exists", get(account_exists))
        // .route("/api/sessions/open", post(open_session))
        // .route("/api/permissions/check", post(check_permissions))
}

async fn create_account(
    State(db): State<DbPool>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, (StatusCode, String)> {
    info!("Creating account for user {}", request.user_id);
    todo!()
}

// async fn open_session(
//     State(db): State<DbPool>,
//     Json(request): Json<SessionRequest>,
// ) -> Result<Json<SessionResponse>, (StatusCode, String)> {
//     info!(
//         "Opening session for user {} on stream {}",
//         request.user_id, request.stream_id
//     );

//     // First check permissions
//     let permission_check = check_permissions(State(db.clone()), Json(request.clone())).await?;

//     if !permission_check.has_permission {
//         return Err((
//             StatusCode::FORBIDDEN,
//             format!(
//                 "User {} does not have permission to access stream {}",
//                 request.user_id, request.stream_id
//             ),
//         ));
//     }

//     // Create session in TEE (placeholder)
//     let session_id = tee::create_session(&request.user_id, &request.stream_id)
//         .await
//         .map_err(|e| {
//             error!("TEE error: {}", e);
//             (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("TEE error: {}", e),
//             )
//         })?;

//     // Store session in database
//     let session = sqlx::query_as::<_, Session>(
//         "INSERT INTO sessions (id, user_id, stream_id, status)
//          VALUES ($1, $2, $3, $4)
//          RETURNING id, user_id, stream_id, status, created_at",
//     )
//     .bind(session_id)
//     .bind(&request.user_id)
//     .bind(&request.stream_id)
//     .bind("created")
//     .fetch_one(&db)
//     .await
//     .map_err(|e| {
//         error!("Database error: {}", e);
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "Failed to create session".to_string(),
//         )
//     })?;

//     info!("Session {} created successfully", session.id);

//     Ok(Json(SessionResponse {
//         session_id: session.id.to_string(),
//         user_id: session.user_id,
//         stream_id: session.stream_id,
//         status: SessionStatus::Created,
//         created_at: session.created_at,
//     }))
// }

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
