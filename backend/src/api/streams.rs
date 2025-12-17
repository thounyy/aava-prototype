use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::database::DbPool;
use crate::models::session::Session;

pub fn create_router() -> Router<DbPool> {
    Router::new()
        .route("/api/streams/start", post(stream_start))
        .route("/api/streams/end", post(stream_end))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartRequest {
    // fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartResponse {
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndRequest {
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndResponse {
    pub stream_id: String,
    pub sessions_count: u64,
    pub walrus_url: Option<String>,
    pub proof_hash: Option<String>,
    pub status: String,
}

/// Start a stream
///
/// Placeholder for Sui blockchain call to start a stream.
/// In production, this would:
/// - Call Sui to mark stream as active
/// - Update stream object on-chain
/// - Emit events for stream start
async fn stream_start(
    State(_db): State<DbPool>,
    Json(request): Json<StreamStartRequest>,
) -> Result<Json<StreamStartResponse>, (StatusCode, String)> {
    info!("Starting stream");

    // TODO: Real Sui implementation
    // - Call Sui Move function to start stream
    // - Update stream object status on-chain
    // - Emit stream_start event
    // Example:
    // let tx_result = sui::start_stream(&request.stream_id).await?;

    warn!("[PLACEHOLDER] Stream start - Sui call not implemented");

    Ok(Json(StreamStartResponse {
        stream_id: "stream_id".to_string(),
    }))
}

/// End a stream
///
/// This endpoint:
/// 1. Batches all sessions with this stream_id
/// 2. Publishes session data to Walrus (decentralized storage)
/// 3. Generates ZK proof via Nautilus
/// 4. Publishes proof to Sui blockchain
async fn stream_end(
    State(db): State<DbPool>,
    Json(request): Json<StreamEndRequest>,
) -> Result<Json<StreamEndResponse>, (StatusCode, String)> {
    info!(
        "Ending stream {} - batching sessions and generating proof",
        request.stream_id
    );

    // Step 1: Query all sessions for this stream
    let sessions = sqlx::query_as::<_, Session>(
        "SELECT id, viewer_id, stream_id, status, created_at
         FROM sessions
         WHERE stream_id = $1 AND status IN ('active', 'open', 'created')
         ORDER BY created_at",
    )
    .bind(&request.stream_id)
    .fetch_all(&db)
    .await
    .map_err(|e| {
        error!("Database error querying sessions: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to query sessions: {}", e),
        )
    })?;

    let sessions_count = sessions.len() as u64;
    info!(
        "Found {} sessions for stream {}",
        sessions_count, request.stream_id
    );

    if sessions.is_empty() {
        warn!("No active sessions found for stream {}", request.stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id: request.stream_id,
            sessions_count: 0,
            walrus_url: None,
            proof_hash: None,
            status: "completed".to_string(),
        }));
    }

    // Step 2: Prepare session data for batch processing
    let session_data = sessions
        .iter()
        .map(|s| {
            serde_json::json!({
                "session_id": s.id.to_string(),
                "viewer_id": s.viewer_id,
                "stream_id": s.stream_id,
                "status": s.status,
                "created_at": s.created_at.to_rfc3339(),
            })
        })
        .collect::<Vec<_>>();

    // Step 3: Publish to Walrus (placeholder)
    // TODO: Real Walrus implementation
    // - Upload session data to Walrus
    // - Get content hash and URL
    // Example:
    // let walrus_result = walrus::upload(&session_data).await?;
    // let walrus_url = walrus_result.url;

    warn!("[PLACEHOLDER] Publishing to Walrus - not implemented");
    let walrus_url = Some(format!(
        "walrus://placeholder/{}/sessions.json",
        request.stream_id
    ));

    // Step 4: Generate ZK proof via Nautilus (placeholder)
    // TODO: Real Nautilus implementation
    // - Send session data to Nautilus server
    // - Generate ZK proof of all sessions
    // - Get proof hash
    // Example:
    // let nautilus_url = env::var("NAUTILUS_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    // let proof = nautilus::generate_batch_proof(&nautilus_url, &session_data).await?;
    // let proof_hash = proof.hash;

    warn!("[PLACEHOLDER] Generating Nautilus proof - not implemented");
    let proof_hash = Some(format!("proof_hash_placeholder_{}", request.stream_id));

    // Step 5: Publish proof to Sui blockchain (placeholder)
    // TODO: Real Sui implementation
    // - Submit proof transaction to Sui
    // - Wait for transaction confirmation
    // Example:
    // let tx_result = sui::publish_stream_proof(&request.stream_id, &proof_hash, &walrus_url).await?;

    warn!("[PLACEHOLDER] Publishing proof to Sui - not implemented");

    // Step 6: Update all sessions to 'completed' status
    let updated = sqlx::query(
        "UPDATE sessions 
         SET status = 'completed', updated_at = NOW()
         WHERE stream_id = $1 AND status IN ('active', 'open', 'created')",
    )
    .bind(&request.stream_id)
    .execute(&db)
    .await
    .map_err(|e| {
        error!("Database error updating sessions: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update sessions: {}", e),
        )
    })?;

    info!(
        "Stream {} ended: {} sessions processed, proof published",
        request.stream_id,
        updated.rows_affected()
    );

    Ok(Json(StreamEndResponse {
        stream_id: request.stream_id,
        sessions_count,
        walrus_url,
        proof_hash,
        status: "completed".to_string(),
    }))
}
