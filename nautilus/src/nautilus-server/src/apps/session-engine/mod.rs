// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// ====
/// Session Engine Nautilus App
/// Handles session creation and termination with direct database writes
/// ====

/// Request payload for creating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRequest {
    pub viewer_id: String,
    pub stream_id: String,
}

/// Response for session creation
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
}

/// Request payload for terminating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct TerminateSessionRequest {
    pub session_id: String,
}

/// Response for session termination
#[derive(Debug, Serialize, Deserialize)]
pub struct TerminateSessionResponse {
    pub session_id: String,
    pub status: String,
}

/// Create a new session
/// Generates session ID and writes directly to database
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SessionRequest>,
) -> Result<Json<SessionResponse>, EnclaveError> {
    info!(
        "Creating session for viewer {} on stream {}",
        request.viewer_id, request.stream_id
    );

    // Generate unique session ID
    let session_id = Uuid::new_v4();

    // Write directly to database
    let result = sqlx::query(
        "INSERT INTO sessions (id, viewer_id, stream_id, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, viewer_id, stream_id, status",
    )
    .bind(session_id)
    .bind(&request.viewer_id)
    .bind(&request.stream_id)
    .bind("created")
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(row) => {
            let db_session_id: Uuid = row.get("id");
            let db_viewer_id: String = row.get("viewer_id");
            let db_stream_id: String = row.get("stream_id");
            let db_status: String = row.get("status");

            info!("Session {} created successfully in database", db_session_id);

            Ok(Json(SessionResponse {
                session_id: db_session_id.to_string(),
                viewer_id: db_viewer_id,
                stream_id: db_stream_id,
                status: db_status,
            }))
        }
        Err(e) => {
            error!("Database error creating session: {}", e);
            Err(EnclaveError::GenericError(format!(
                "Failed to create session: {}",
                e
            )))
        }
    }
}

/// Terminate a session
/// Updates session status to 'completed' in database
pub async fn terminate_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TerminateSessionRequest>,
) -> Result<Json<TerminateSessionResponse>, EnclaveError> {
    let session_id = Uuid::parse_str(&request.session_id).map_err(|e| {
        EnclaveError::GenericError(format!("Invalid session ID: {}", e))
    })?;

    info!("Terminating session {}", session_id);

    // Update session status to completed
    let result = sqlx::query(
        "UPDATE sessions 
        SET status = 'completed', updated_at = NOW()
        WHERE id = $1
        RETURNING id, status",
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(row)) => {
            let db_session_id: Uuid = row.get("id");
            let db_status: String = row.get("status");

            info!("Session {} terminated successfully", db_session_id);

            Ok(Json(TerminateSessionResponse {
                session_id: db_session_id.to_string(),
                status: db_status,
            }))
        }
        Ok(None) => {
            Err(EnclaveError::GenericError(format!(
                "Session {} not found",
                session_id
            )))
        }
        Err(e) => {
            error!("Database error terminating session: {}", e);
            Err(EnclaveError::GenericError(format!(
                "Failed to terminate session: {}",
                e
            )))
        }
    }
}

// Keep process_data for backward compatibility, but it now just calls create_session
use crate::common::ProcessDataRequest;
pub async fn process_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<SessionRequest>>,
) -> Result<Json<SessionResponse>, EnclaveError> {
    create_session(State(state), Json(request.payload)).await
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_session_id_generation() {
        let session_id = Uuid::new_v4();
        assert!(!session_id.to_string().is_empty());
    }
}
