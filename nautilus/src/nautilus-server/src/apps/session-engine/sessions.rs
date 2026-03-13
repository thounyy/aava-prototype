// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::AppState;
use crate::EnclaveError;
use crate::require_internal_auth;
use axum::http::HeaderMap;
use axum::extract::State;
use axum::Json;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Request payload for creating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub viewer_id: String,
    pub stream_id: String,
}

/// Response for session creation
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
}

/// Request payload for terminating a session
#[derive(Debug, Serialize, Deserialize)]
pub struct CloseSessionRequest {
    pub session_id: String,
}

/// Response for session termination
#[derive(Debug, Serialize, Deserialize)]
pub struct CloseSessionResponse {
    pub session_id: String,
    pub status: String,
}

/// Create a new session
/// Generates session ID and writes to Redis
pub async fn open_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<OpenSessionRequest>,
) -> Result<Json<OpenSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!(
        "Creating session for viewer {} on stream {}",
        request.viewer_id, request.stream_id
    );

    // Generate unique session ID
    let session_id = Uuid::new_v4().to_string(); // TODO: should be 56-bit
    let status = "created";
    let created_at = chrono::Utc::now().to_rfc3339();

    // Create session data as JSON
    let session_data = serde_json::json!({
        "session_id": session_id,
        "viewer_id": request.viewer_id,
        "stream_id": request.stream_id,
        "status": status,
        "created_at": created_at,
    });

    let mut redis = state.redis.clone();

    // Store session data in Redis
    // Key: session:{session_id}
    // Value: JSON string of session data
    // TTL: 24 hours (86400 seconds) as safety net
    let session_key = format!("session:{}", session_id);
    let session_json = serde_json::to_string(&session_data)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to serialize session: {}", e)))?;

    // Add session to stream's session set
    let stream_sessions_key = format!("stream:{}:sessions", request.stream_id);

    // Store session and add to stream set atomically
    // Use MULTI/EXEC for atomicity
    let mut pipe = redis::pipe();
    pipe.atomic()
        .set(&session_key, &session_json)
        .expire(&session_key, 86400) // 24 hour TTL
        .sadd(&stream_sessions_key, &session_id)
        .expire(&stream_sessions_key, 86400); // Also expire the set

    let _: () = pipe.query_async(&mut redis).await.map_err(|e| {
        error!("Redis error creating session: {}", e);
        EnclaveError::GenericError(format!("Failed to create session: {}", e))
    })?;

    info!("Session {} created successfully in Redis", session_id);

    Ok(Json(OpenSessionResponse {
        session_id: session_id,
        viewer_id: request.viewer_id,
        stream_id: request.stream_id,
        status: status.to_string(),
    }))
}

/// Terminate a session
/// Updates session status to 'completed' in Redis
pub async fn close_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CloseSessionRequest>,
) -> Result<Json<CloseSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    let session_id_str = &request.session_id;
    let session_id = Uuid::parse_str(session_id_str)
        .map_err(|e| EnclaveError::GenericError(format!("Invalid session ID: {}", e)))?;

    info!("Closing session {}", session_id);

    let mut redis = state.redis.clone();
    let session_key = format!("session:{}", session_id_str);

    // Get existing session data
    let session_json: Option<String> = redis.get(&session_key).await.map_err(|e| {
        error!("Redis error reading session: {}", e);
        EnclaveError::GenericError(format!("Failed to read session: {}", e))
    })?;

    let session_data: serde_json::Value = match session_json {
        Some(json) => serde_json::from_str(&json).map_err(|e| {
            EnclaveError::GenericError(format!("Failed to parse session data: {}", e))
        })?,
        None => {
            return Err(EnclaveError::GenericError(format!(
                "Session {} not found",
                session_id
            )));
        }
    };

    // Update status to completed
    let mut updated_data = session_data.clone();
    updated_data["status"] = serde_json::Value::String("completed".to_string());
    updated_data["updated_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());

    let updated_json = serde_json::to_string(&updated_data).map_err(|e| {
        EnclaveError::GenericError(format!("Failed to serialize updated session: {}", e))
    })?;

    // Update session in Redis
    let _: () = redis.set(&session_key, &updated_json).await.map_err(|e| {
        error!("Redis error updating session: {}", e);
        EnclaveError::GenericError(format!("Failed to close session: {}", e))
    })?;

    info!("Session {} closed successfully", session_id);

    Ok(Json(CloseSessionResponse {
        session_id: session_id_str.clone(),
        status: "completed".to_string(),
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_session_id_generation() {
        let session_id = Uuid::new_v4();
        assert!(!session_id.to_string().is_empty());
    }
}
