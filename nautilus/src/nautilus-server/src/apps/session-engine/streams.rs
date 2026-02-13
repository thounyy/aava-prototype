// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::{to_signed_response, IntentMessage, IntentScope, ProcessedDataResponse};
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::num::NonZeroU16;
use walrus_core::{
    EncodingType,
    encoding::{EncodingConfig, EncodingFactory},
    metadata::BlobMetadataApi,
};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Request payload for ending a stream
#[derive(Debug, Serialize, Deserialize)]
pub struct EndStreamRequest {
    pub stream_id: String,
}

/// Response payload for ending a stream
/// Contains the session data and cryptographic attestation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndStreamResponse {
    pub stream_id: String,
    pub sessions: Vec<SessionData>,
    pub sessions_count: u64,
    pub blob_id: ByteBuf,
    pub root_hash: ByteBuf,
    pub size: u64,
    pub encoding_type: u8,
}

/// Request payload for cleaning up sessions after Walrus upload
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupStreamRequest {
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

/// End a stream
///
/// This function:
/// 1. Queries Redis for all sessions with the given stream_id
/// 2. Generates cryptographic attestation of the data
/// 3. Returns the session data with attestation
///
/// Only the enclave interacts with Redis, ensuring the dataset is proven.
pub async fn end_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<EndStreamRequest>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<EndStreamResponse>>>, EnclaveError> {
    info!(
        "Ending stream {} - querying sessions from Redis",
        request.stream_id
    );

    let mut redis = state.redis.clone();
    let stream_sessions_key = format!("stream:{}:sessions", request.stream_id);

    // Get all session IDs for this stream
    let session_ids: Vec<String> = redis.smembers(&stream_sessions_key).await.map_err(|e| {
        error!("Redis error querying stream sessions: {}", e);
        EnclaveError::GenericError(format!("Failed to query sessions: {}", e))
    })?;

    let sessions_count = session_ids.len() as u64;
    info!(
        "Found {} sessions for stream {}",
        sessions_count, request.stream_id
    );

    if session_ids.is_empty() {
        return Err(EnclaveError::GenericError(format!(
            "No sessions found for stream {}",
            request.stream_id
        )));
    }

    // Batch fetch all session data
    let mut sessions: Vec<SessionData> = Vec::new();
    for session_id in &session_ids {
        let session_key = format!("session:{}", session_id);
        let session_json: Option<String> = redis.get(&session_key).await.map_err(|e| {
            error!("Redis error reading session {}: {}", session_id, e);
            EnclaveError::GenericError(format!("Failed to read session {}: {}", session_id, e))
        })?;

        if let Some(json) = session_json {
            let session_value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
                EnclaveError::GenericError(format!(
                    "Failed to parse session data for {}: {}",
                    session_id, e
                ))
            })?;

            sessions.push(SessionData {
                session_id: session_value["session_id"]
                    .as_str()
                    .unwrap_or(session_id)
                    .to_string(),
                viewer_id: session_value["viewer_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                stream_id: session_value["stream_id"]
                    .as_str()
                    .unwrap_or(&request.stream_id)
                    .to_string(),
                status: session_value["status"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                created_at: session_value["created_at"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    // Sort by created_at for consistency
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let sessions_count = sessions.len() as u64;

    // Serialize session data and compute Walrus blob metadata for verification.
    let data_json = serde_json::to_string(&sessions)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to serialize sessions: {}", e)))?;
    let walrus_metadata = compute_walrus_metadata(data_json.as_bytes()).map_err(|e| {
        EnclaveError::GenericError(format!("Failed to compute Walrus blob id: {}", e))
    })?;

    let response_data = EndStreamResponse {
        stream_id: request.stream_id.clone(),
        sessions,
        sessions_count,
        blob_id: ByteBuf::from(walrus_metadata.blob_id),
        root_hash: ByteBuf::from(walrus_metadata.root_hash),
        size: walrus_metadata.size,
        encoding_type: walrus_metadata.encoding_type,
    };

    // Generate cryptographic attestation using enclave keypair
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let signed_response = to_signed_response(
        &state.eph_kp,
        response_data,
        timestamp_ms,
        IntentScope::HashSessions,
    );

    info!(
        "Stream {} ended: {} sessions attested and signed",
        request.stream_id, sessions_count
    );

    Ok(Json(signed_response))
}

/// Cleanup stream data after successful Walrus upload
/// Deletes all sessions for a stream from Redis
pub async fn cleanup_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CleanupStreamRequest>,
) -> Result<Json<serde_json::Value>, EnclaveError> {
    info!("Cleaning up sessions for stream {}", request.stream_id);

    let mut redis = state.redis.clone();
    let stream_sessions_key = format!("stream:{}:sessions", request.stream_id);

    // Get all session IDs for this stream
    let session_ids: Vec<String> = redis.smembers(&stream_sessions_key).await.map_err(|e| {
        error!("Redis error querying stream sessions: {}", e);
        EnclaveError::GenericError(format!("Failed to query sessions: {}", e))
    })?;

    if session_ids.is_empty() {
        warn!(
            "No sessions found to cleanup for stream {}",
            request.stream_id
        );
        return Ok(Json(serde_json::json!({
            "stream_id": request.stream_id,
            "deleted_count": 0,
            "status": "completed"
        })));
    }

    // Delete all session keys and the stream set
    let mut deleted_count = 0u64;
    for session_id in &session_ids {
        let session_key = format!("session:{}", session_id);
        let deleted: bool = redis.del(&session_key).await.map_err(|e| {
            error!("Redis error deleting session {}: {}", session_id, e);
            EnclaveError::GenericError(format!("Failed to delete session {}: {}", session_id, e))
        })?;
        if deleted {
            deleted_count += 1;
        }
    }

    // Delete the stream sessions set
    let _: () = redis.del(&stream_sessions_key).await.map_err(|e| {
        error!("Redis error deleting stream sessions set: {}", e);
        EnclaveError::GenericError(format!("Failed to delete stream sessions set: {}", e))
    })?;

    info!(
        "Cleaned up {} sessions for stream {}",
        deleted_count, request.stream_id
    );

    Ok(Json(serde_json::json!({
        "stream_id": request.stream_id,
        "deleted_count": deleted_count,
        "status": "completed"
    })))
}

struct WalrusMetadataBytes {
    blob_id: Vec<u8>,
    root_hash: Vec<u8>,
    size: u64,
    encoding_type: u8,
}

fn compute_walrus_metadata(blob: &[u8]) -> Result<WalrusMetadataBytes, anyhow::Error> {
    let n_shards = std::env::var("WALRUS_N_SHARDS")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(31);
    let n_shards = NonZeroU16::new(n_shards)
        .ok_or_else(|| anyhow::anyhow!("WALRUS_N_SHARDS must be > 0"))?;

    let config = EncodingConfig::new(n_shards).get_for_type(EncodingType::RS2);
    let metadata = config.compute_metadata(blob)?;
    let root_hash = metadata.metadata().compute_root_hash();

    Ok(WalrusMetadataBytes {
        blob_id: metadata.blob_id().as_ref().to_vec(),
        root_hash: root_hash.bytes().to_vec(),
        size: metadata.metadata().unencoded_length(),
        encoding_type: metadata.metadata().encoding_type().into(),
    })
}