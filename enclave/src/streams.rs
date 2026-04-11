// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::sessions::SessionRecord;
use crate::bcs_utils::{to_signed_response, IntentMessage, IntentScope, ProcessedDataResponse};
use crate::require_internal_auth;
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::num::NonZeroU16;
use std::sync::Arc;
use tracing::{error, info, warn};
use walrus_core::{
    encoding::{EncodingConfig, EncodingFactory},
    metadata::BlobMetadataApi,
    EncodingType,
};

/// Session data structure for batch processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: String,
}

/// Request payload for ending a stream
#[derive(Debug, Serialize, Deserialize)]
pub struct EndStreamRequest {
    pub stream_id: String,
    pub n_shards: u16,
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
    pub n_shards: NonZeroU16,
    pub unencoded_size: u64,
    pub encoding_type: EncodingType,
    pub encoded_size: u64,
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
    headers: HeaderMap,
    Json(request): Json<EndStreamRequest>,
) -> Result<Json<ProcessedDataResponse<IntentMessage<EndStreamResponse>>>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!(
        "Ending stream {} - querying sessions from Redis",
        request.stream_id
    );

    let mut redis = state.redis.clone();

    let session_ids: Vec<String> = redis
        .smembers(&format!("stream:{}:sessions", request.stream_id))
        .await
        .map_err(|e| {
            error!("Redis error querying stream sessions: {}", e);
            EnclaveError::RedisError(format!("Failed to query sessions: {}", e))
        })?;

    let sessions_count = session_ids.len() as u64;
    info!(
        "Found {} sessions for stream {}",
        sessions_count, request.stream_id
    );

    if session_ids.is_empty() {
        return Err(EnclaveError::NotFound(format!(
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
            EnclaveError::RedisError(format!("Failed to read session {}: {}", session_id, e))
        })?;

        if let Some(json) = session_json {
            let session_value: SessionRecord = serde_json::from_str(&json).map_err(|e| {
                EnclaveError::ParseError(format!(
                    "Failed to parse session data for {}: {}",
                    session_id, e
                ))
            })?;

            sessions.push(SessionData {
                session_id: session_value.session_id,
                viewer_id: session_value.viewer_id,
                stream_id: session_value.stream_id,
                status: session_value.status.as_str().to_string(),
                created_at: session_value.created_at,
            });
        }
    }

    // Sort by created_at for consistency
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    // Serialize session data and compute Walrus blob metadata for verification.
    let data_json = serde_json::to_string(&sessions)
        .map_err(|e| EnclaveError::ParseError(format!("Failed to serialize sessions: {}", e)))?;
    let walrus_metadata =
        compute_walrus_metadata(request.n_shards, data_json.as_bytes()).map_err(|e| {
            EnclaveError::WalrusError(format!("Failed to compute Walrus blob id: {}", e))
        })?;

    let response_data = EndStreamResponse {
        stream_id: request.stream_id.clone(),
        sessions,
        sessions_count,
        blob_id: ByteBuf::from(walrus_metadata.blob_id),
        root_hash: ByteBuf::from(walrus_metadata.root_hash),
        n_shards: walrus_metadata.n_shards,
        unencoded_size: walrus_metadata.unencoded_size,
        encoding_type: walrus_metadata.encoding_type,
        encoded_size: walrus_metadata.encoded_size,
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
        IntentScope::EndStream,
    );

    info!(
        "Stream {} ended: {} sessions attested and signed",
        request.stream_id, sessions_count
    );

    Ok(Json(signed_response))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupStreamRequest {
    pub stream_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupStreamResponse {
    pub deleted_count: u64,
}

/// Cleanup stream data after successful Walrus upload
/// Deletes all sessions for a stream from Redis
pub async fn cleanup_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CleanupStreamRequest>,
) -> Result<Json<CleanupStreamResponse>, EnclaveError> {
    require_internal_auth(&headers)?;
    info!("Cleaning up sessions for stream {}", request.stream_id);

    let mut redis = state.redis.clone();

    let session_ids: Vec<String> = redis
        .smembers(&format!("stream:{}:sessions", request.stream_id))
        .await
        .map_err(|e| {
            error!("Redis error querying stream sessions: {}", e);
            EnclaveError::RedisError(format!("Failed to query sessions: {}", e))
        })?;

    if session_ids.is_empty() {
        warn!(
            "No sessions found to cleanup for stream {}",
            request.stream_id
        );
        return Ok(Json(CleanupStreamResponse { deleted_count: 0 }));
    }

    // Delete all session keys and the stream set
    let mut deleted_count = 0u64;
    for session_id in &session_ids {
        let deleted: bool = redis
            .del(&format!("session:{}", session_id))
            .await
            .map_err(|e| {
                error!("Redis error deleting session {}: {}", session_id, e);
                EnclaveError::RedisError(format!("Failed to delete session {}: {}", session_id, e))
            })?;
        if deleted {
            deleted_count += 1;
        }
    }

    // Delete the stream sessions set
    let _: () = redis
        .del(&format!("stream:{}:sessions", request.stream_id))
        .await
        .map_err(|e| {
            error!("Redis error deleting stream sessions set: {}", e);
            EnclaveError::RedisError(format!("Failed to delete stream sessions set: {}", e))
        })?;

    info!(
        "Cleaned up {} sessions for stream {}",
        deleted_count, request.stream_id
    );

    Ok(Json(CleanupStreamResponse { deleted_count }))
}

struct WalrusMetadata {
    blob_id: Vec<u8>,
    root_hash: Vec<u8>,
    n_shards: NonZeroU16,
    unencoded_size: u64,
    encoding_type: EncodingType,
    encoded_size: u64,
}

fn compute_walrus_metadata(n_shards: u16, blob: &[u8]) -> Result<WalrusMetadata, anyhow::Error> {
    let n_shards = NonZeroU16::new(n_shards).unwrap();
    let config = EncodingConfig::new(n_shards).get_for_type(EncodingType::RS2);
    let metadata = config.compute_metadata(blob)?;

    Ok(WalrusMetadata {
        blob_id: metadata.blob_id().as_ref().to_vec(),
        root_hash: metadata.metadata().compute_root_hash().bytes().to_vec(),
        n_shards: metadata.n_shards(),
        unencoded_size: metadata.metadata().unencoded_length(),
        encoding_type: metadata.metadata().encoding_type(),
        encoded_size: metadata
            .metadata()
            .encoded_size()
            .ok_or_else(|| anyhow::anyhow!("Encoded size is None"))?,
    })
}
