use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use tracing::{error, info};

use crate::database::DbPool;
use crate::models::session::*;
use crate::sui;
use crate::tee;

pub fn create_router() -> Router<DbPool> {
    Router::new().route("/api/sessions/open", post(open_session))
    // .route("/api/account/get", get(get_account))
    // .route("/api/account/exists", get(account_exists))
    // .route("/api/sessions/open", post(open_session))
    // .route("/api/permissions/check", post(check_permissions))
}

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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use axum::{body::Body, http::{Request, StatusCode}, Router};
//     use tower::ServiceExt; // brings `oneshot` into scope
//     use sqlx::PgPool;

//     #[sqlx::test(migrator = "sqlx::migrate!()")]
//     async fn open_session_inserts_row(pool: PgPool) {
//         let app = Router::new()
//             .route("/api/session/open", post(open_session))
//             .with_state(pool.clone());

//         let payload = OpenSessionRequest {
//             viewer_id: "viewer-123".into(),
//             stream_id: "stream-456".into(),
//             // fill other fields if the struct requires them
//         };

//         let response = app
//             .oneshot(
//                 Request::post("/api/session/open")
//                     .header("content-type", "application/json")
//                     .body(Body::from(serde_json::to_vec(&payload).unwrap()))
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(response.status(), StatusCode::OK);

//         let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE viewer_id = $1")
//             .bind(&payload.viewer_id)
//             .fetch_one(&pool)
//             .await
//             .unwrap();

//         assert_eq!(count.0, 1, "expected exactly one session row");
//     }
// }