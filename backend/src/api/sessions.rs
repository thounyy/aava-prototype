use axum::{http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

use crate::models::session::*;

pub fn create_router() -> Router {
    Router::new()
        .route("/api/sessions/open", post(open_session))
        .route("/api/sessions/close", post(close_session))
}

/// Response from the enclave for session creation
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveOpenSessionResponse {
    session_id: String,
    viewer_id: String,
    stream_id: String,
    status: String,
}

/// Response from the enclave for session termination
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveCloseSessionResponse {
    session_id: String,
    status: String,
}

async fn open_session(
    Json(request): Json<OpenSessionRequest>,
) -> Result<Json<OpenSessionResponse>, (StatusCode, String)> {
    info!(
        "Opening session for viewer {} on stream {}",
        request.viewer_id, request.stream_id
    );

    // Get enclave URL from environment, default to localhost
    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create request payload
    let request_body = serde_json::json!({
        "viewer_id": request.viewer_id,
        "stream_id": request.stream_id,
    });

    // Make HTTP request to enclave
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/open_session", enclave_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave at {}: {}", enclave_url, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE connection error: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE error: HTTP {} - {}", status, error_text),
        ));
    }

    // Parse response
    let enclave_response: EnclaveOpenSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE response parsing error: {}", e),
        )
    })?;

    info!(
        "Session {} opened successfully",
        enclave_response.session_id
    );

    Ok(Json(OpenSessionResponse {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
        status: SessionStatus::Open,
        created_at: chrono::Utc::now(),
    }))
}

async fn close_session(
    Json(request): Json<CloseSessionRequest>,
) -> Result<Json<CloseSessionResponse>, (StatusCode, String)> {
    info!("Closing session {}", request.session_id);

    // Get enclave URL from environment, default to localhost
    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create request payload
    let request_body = serde_json::json!({
        "session_id": request.session_id,
    });

    // Make HTTP request to enclave
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/close_session", enclave_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave at {}: {}", enclave_url, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("TEE connection error: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE error: HTTP {} - {}", status, error_text),
        ));
    }

    // Parse response
    let enclave_response: EnclaveCloseSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TEE response parsing error: {}", e),
        )
    })?;

    info!(
        "Session {} closed successfully",
        enclave_response.session_id
    );

    Ok(Json(CloseSessionResponse {
        session_id: enclave_response.session_id,
        status: SessionStatus::Closed,
    }))
}
