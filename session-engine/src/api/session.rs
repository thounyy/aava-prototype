use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::session::*;
use crate::sui;
use crate::tee;

async fn open_session(
    State(db): State<DbPool>,
    Json(request): Json<OpenSessionRequest>,
) -> Result<Json<OpenSessionResponse>, (StatusCode, String)> {
    info!(
        "Opening session for viewer {} on stream {}",
        request.viewer_id, request.stream_id
    );

    // TODO: Check permissions
    // let permission_check = check_permissions(State(db.clone()), Json(request.clone())).await?;

    // if !permission_check.has_permission {
    //     return Err((
    //         StatusCode::FORBIDDEN,
    //         format!(
    //             "User {} does not have permission to access stream {}",
    //             request.user_id, request.stream_id
    //         ),
    //     ));
    // }

    // TODO: Create session in TEE (placeholder)
    let session_id = uuid::Uuid::new_v4();
    // let session_id = tee::create_session(&request.user_id, &request.stream_id)
    //     .await
    //     .map_err(|e| {
    //         error!("TEE error: {}", e);
    //         (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             format!("TEE error: {}", e),
    //         )
    //     })?;

    // Store session in database
    let session = sqlx::query_as::<_, Session>(
        "INSERT INTO sessions (id, viewer_id, stream_id, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, viewer_id, stream_id, status, created_at",
    )
    .bind(session_id)
    .bind(&request.viewer_id)
    .bind(&request.stream_id)
    .bind("created")
    .fetch_one(&db)
    .await
    .map_err(|e| {
        error!("Database error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create session".to_string(),
        )
    })?;

    info!("Session {} created successfully", session.id);

    Ok(Json(OpenSessionResponse {
        session_id: session.id.to_string(),
        viewer_id: session.viewer_id,
        stream_id: session.stream_id,
        status: SessionStatus::Created,
        created_at: session.created_at,
    }))
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
