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

/// Request payload for ending a stream
#[derive(Debug, Serialize, Deserialize)]
pub struct EndStreamRequest {
    pub stream_id: String,
}

/// Session data structure for batch processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
}

/// Response payload for ending a stream
/// Contains the session data and cryptographic attestation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndStreamResponse {
    pub stream_id: String,
    pub sessions: Vec<SessionData>,
    pub sessions_count: u64,
    pub data_hash: String,
}

/// End a stream
///
/// This function:
/// 1. Queries the database for all sessions with the given stream_id
/// 2. Generates cryptographic attestation of the data
/// 3. Returns the session data with attestation
///
/// Only the enclave interacts with the database, ensuring the dataset is proven.
pub async fn end_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<EndStreamRequest>,
) -> Result<
    Json<crate::common::ProcessedDataResponse<crate::common::IntentMessage<EndStreamResponse>>>,
    EnclaveError,
> {
    info!(
        "Ending stream {} - querying sessions from database",
        request.stream_id
    );

    // Query all active sessions for this stream from database
    let result = sqlx::query(
        "SELECT id, viewer_id, stream_id, status, created_at
         FROM sessions
         WHERE stream_id = $1 AND status IN ('active', 'open', 'created')
         ORDER BY created_at",
    )
    .bind(&request.stream_id)
    .fetch_all(&state.db)
    .await;

    let rows = match result {
        Ok(rows) => rows,
        Err(e) => {
            error!("Database error querying sessions: {}", e);
            return Err(EnclaveError::GenericError(format!(
                "Failed to query sessions: {}",
                e
            )));
        }
    };

    let sessions_count = rows.len() as u64;
    info!(
        "Found {} sessions for stream {}",
        sessions_count, request.stream_id
    );

    // Convert database rows to SessionData
    let sessions: Vec<SessionData> = rows
        .into_iter()
        .map(|row| {
            let session_id: Uuid = row.get("id");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");

            SessionData {
                session_id: session_id.to_string(),
                viewer_id: row.get("viewer_id"),
                stream_id: row.get("stream_id"),
                status: row.get("status"),
                created_at: created_at.to_rfc3339(),
            }
        })
        .collect();

    // Calculate hash of the session data for verification
    use sha2::{Digest, Sha256};
    let data_json = serde_json::to_string(&sessions)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to serialize sessions: {}", e)))?;
    let mut hasher = Sha256::new();
    hasher.update(data_json.as_bytes());
    let data_hash = format!("{:x}", hasher.finalize());

    let stream_id = request.stream_id.clone();
    let response_data = EndStreamResponse {
        stream_id: stream_id.clone(),
        sessions,
        sessions_count,
        data_hash,
    };

    // Generate cryptographic attestation using enclave keypair
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let signed_response = crate::common::to_signed_response(
        &state.eph_kp,
        response_data,
        timestamp_ms,
        crate::common::IntentScope::ProcessData,
    );

    info!(
        "Stream {} ended: {} sessions attested and signed",
        stream_id, sessions_count
    );

    Ok(Json(signed_response))
}
