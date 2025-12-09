use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

/// TEE client for communicating with Nautilus enclave
///
/// This module handles communication with the Nautilus enclave running
/// in a TEE (Trusted Execution Environment). The enclave creates sessions
/// and writes directly to the database.

/// Response from the enclave for session creation
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveSessionResponse {
    session_id: String,
    viewer_id: String,
    stream_id: String,
    status: String,
}

/// Response from the enclave for session termination
#[derive(Debug, Serialize, Deserialize)]
struct EnclaveTerminateResponse {
    session_id: String,
    status: String,
}

/// Result of creating a session in the TEE
#[derive(Debug, Clone)]
pub struct TEESessionResult {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
}

/// Result of terminating a session in the TEE
#[derive(Debug, Clone)]
pub struct TEETerminateResult {
    pub session_id: String,
}

/// Create a new session in the TEE
/// The enclave writes directly to the database
pub async fn create_session(viewer_id: &str, stream_id: &str) -> Result<TEESessionResult> {
    info!(
        "Creating session in TEE for viewer {} on stream {}",
        viewer_id, stream_id
    );

    // Get enclave URL from environment, default to localhost
    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create request payload
    let request = serde_json::json!({
        "viewer_id": viewer_id,
        "stream_id": stream_id,
    });

    // Make HTTP request to enclave
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/create_session", enclave_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave at {}: {}", enclave_url, e);
            anyhow::anyhow!("TEE connection error: {}", e)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err(anyhow::anyhow!(
            "TEE error: HTTP {} - {}",
            status,
            error_text
        ));
    }

    // Parse response
    let enclave_response: EnclaveSessionResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        anyhow::anyhow!("TEE response parsing error: {}", e)
    })?;

    info!(
        "Session {} created successfully in TEE",
        enclave_response.session_id
    );

    Ok(TEESessionResult {
        session_id: enclave_response.session_id,
        viewer_id: enclave_response.viewer_id,
        stream_id: enclave_response.stream_id,
    })
}

/// Terminate a session in the TEE
/// The enclave updates the database directly
pub async fn terminate_session(session_id: &str) -> Result<TEETerminateResult> {
    info!("Terminating session {} in TEE", session_id);

    // Get enclave URL from environment, default to localhost
    let enclave_url =
        env::var("ENCLAVE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create request payload
    let request = serde_json::json!({
        "session_id": session_id,
    });

    // Make HTTP request to enclave
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/terminate_session", enclave_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to connect to enclave at {}: {}", enclave_url, e);
            anyhow::anyhow!("TEE connection error: {}", e)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!("Enclave returned error status {}: {}", status, error_text);
        return Err(anyhow::anyhow!(
            "TEE error: HTTP {} - {}",
            status,
            error_text
        ));
    }

    // Parse response
    let enclave_response: EnclaveTerminateResponse = response.json().await.map_err(|e| {
        error!("Failed to parse enclave response: {}", e);
        anyhow::anyhow!("TEE response parsing error: {}", e)
    })?;

    info!(
        "Session {} terminated successfully in TEE",
        enclave_response.session_id
    );

    Ok(TEETerminateResult {
        session_id: enclave_response.session_id,
    })
}
