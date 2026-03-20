// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::require_internal_auth;
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SessionStatus {
    Opened,
    Warned,
    Revoked,
    Closed,
}

impl SessionStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Warned => "warned",
            Self::Revoked => "revoked",
            Self::Closed => "closed",
        }
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Opened, Self::Warned)
                | (Self::Opened, Self::Revoked)
                | (Self::Opened, Self::Closed)
                | (Self::Warned, Self::Revoked)
                | (Self::Warned, Self::Closed)
                | (Self::Revoked, Self::Closed)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionRecord {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: SessionStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl SessionRecord {
    fn transition_to(&mut self, next: SessionStatus) -> Result<(), EnclaveError> {
        if !self.status.can_transition_to(next) {
            return Err(EnclaveError::InvalidInput(format!(
                "Invalid session status transition: {} -> {}",
                self.status.as_str(),
                next.as_str()
            )));
        }
        self.status = next;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub viewer_id: String,
    pub stream_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub viewer_id: String,
    pub stream_id: String,
    pub session_id: String,
    pub status: String,
}

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

    let session_data = SessionRecord {
        session_id: session_id.clone(),
        viewer_id: request.viewer_id.clone(),
        stream_id: request.stream_id.clone(),
        status: SessionStatus::Opened,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut redis = state.redis.clone();

    // Store session data in Redis
    // Key: session:{session_id}
    // Value: JSON string of session data
    // TTL: 24 hours (86400 seconds) as safety net
    let session_key = format!("session:{}", session_id);
    let session_json = serde_json::to_string(&session_data).map_err(|e| {
        error!("Failed to serialize session: {}", e);
        EnclaveError::ParseError(format!("Failed to serialize session: {}", e))
    })?;

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
        EnclaveError::RedisError(format!("Failed to create session: {}", e))
    })?;

    info!("Session {} created successfully in Redis", session_id);

    Ok(Json(OpenSessionResponse {
        viewer_id: request.viewer_id,
        stream_id: request.stream_id,
        session_id,
        status: SessionStatus::Opened.as_str().to_string(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WarnSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WarnSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn warn_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<WarnSessionRequest>,
) -> Result<Json<WarnSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!("Warning session {}", request.session_id);

    let mut redis = state.redis.clone();
    let mut session_data = get_session_record(&mut redis, &request.session_id).await?;
    session_data.transition_to(SessionStatus::Warned)?;

    let updated_json = serde_json::to_string(&session_data).map_err(|e| {
        error!("Failed to serialize updated session: {}", e);
        EnclaveError::ParseError(format!("Failed to serialize updated session: {}", e))
    })?;

    let _: () = redis
        .set(&format!("session:{}", &request.session_id), &updated_json)
        .await
        .map_err(|e| {
            error!("Redis error updating session: {}", e);
            EnclaveError::RedisError(format!("Failed to warn session: {}", e))
        })?;

    Ok(Json(WarnSessionResponse {
        session_id: request.session_id,
        status: session_data.status.as_str().to_string(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn revoke_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RevokeSessionRequest>,
) -> Result<Json<RevokeSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!("Revoking session {}", request.session_id);

    let mut redis = state.redis.clone();
    let mut session_data = get_session_record(&mut redis, &request.session_id).await?;
    session_data.transition_to(SessionStatus::Revoked)?;

    let updated_json = serde_json::to_string(&session_data).map_err(|e| {
        error!("Failed to serialize updated session: {}", e);
        EnclaveError::ParseError(format!("Failed to serialize updated session: {}", e))
    })?;

    let _: () = redis
        .set(&format!("session:{}", &request.session_id), &updated_json)
        .await
        .map_err(|e| {
            error!("Redis error updating session: {}", e);
            EnclaveError::RedisError(format!("Failed to revoke session: {}", e))
        })?;

    Ok(Json(RevokeSessionResponse {
        session_id: request.session_id,
        status: session_data.status.as_str().to_string(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseSessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn close_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CloseSessionRequest>,
) -> Result<Json<CloseSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!("Closing session {}", &request.session_id);

    let mut redis = state.redis.clone();

    let mut session_data = get_session_record(&mut redis, &request.session_id).await?;
    session_data.transition_to(SessionStatus::Closed)?;

    let updated_json = serde_json::to_string(&session_data).map_err(|e| {
        error!("Failed to serialize updated session: {}", e);
        EnclaveError::ParseError(format!("Failed to serialize updated session: {}", e))
    })?;

    let _: () = redis
        .set(&format!("session:{}", &request.session_id), &updated_json)
        .await
        .map_err(|e| {
            error!("Redis error updating session: {}", e);
            EnclaveError::RedisError(format!("Failed to close session: {}", e))
        })?;

    info!("Session {} closed successfully", request.session_id);

    Ok(Json(CloseSessionResponse {
        session_id: request.session_id,
        status: session_data.status.as_str().to_string(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<GetSessionRequest>,
) -> Result<Json<GetSessionResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!("Getting session status for {}", request.session_id);

    let mut redis = state.redis.clone();
    let session_data = get_session_record(&mut redis, &request.session_id).await?;

    Ok(Json(GetSessionResponse {
        session_id: session_data.session_id,
        status: session_data.status.as_str().to_string(),
        viewer_id: session_data.viewer_id,
        stream_id: session_data.stream_id,
        created_at: session_data.created_at,
        updated_at: session_data.updated_at,
    }))
}

async fn get_session_record(
    redis: &mut ConnectionManager,
    session_id: &str,
) -> Result<SessionRecord, EnclaveError> {
    let _ = Uuid::parse_str(session_id)
        .map_err(|e| EnclaveError::InvalidInput(format!("Invalid session ID: {}", e)))?;

    redis
        .get::<_, Option<String>>(&format!("session:{}", session_id))
        .await
        .map_err(|e| {
            error!("Redis error reading session: {}", e);
            EnclaveError::RedisError(format!("Failed to read session: {}", e))
        })?
        .ok_or_else(|| EnclaveError::NotFound(format!("Session {} not found", session_id)))
        .and_then(|json| {
            serde_json::from_str(&json).map_err(|e| {
                EnclaveError::ParseError(format!("Failed to parse session data: {}", e))
            })
        })
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
