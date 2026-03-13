// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum::http::header::HeaderName;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use fastcrypto::ed25519::Ed25519KeyPair;
use serde_json::json;
use std::fmt;

mod apps {
    #[path = "session-engine/mod.rs"]
    pub mod session_engine;
}

pub mod app {
    pub use crate::apps::session_engine::*;
}

pub mod common;

use redis::aio::ConnectionManager;

/// App state, at minimum needs to maintain the ephemeral keypair.  
pub struct AppState {
    /// Ephemeral keypair on boot
    pub eph_kp: Ed25519KeyPair,
    /// Redis connection manager
    pub redis: ConnectionManager,
}

/// Enclave errors enum.
#[derive(Debug)]
pub enum EnclaveError {
    GenericError(String),
    Unauthorized(String),
    InternalError(String),
}

/// Implement IntoResponse for EnclaveError.
impl IntoResponse for EnclaveError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            EnclaveError::GenericError(e) => (StatusCode::BAD_REQUEST, e),
            EnclaveError::Unauthorized(e) => (StatusCode::UNAUTHORIZED, e),
            EnclaveError::InternalError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl fmt::Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnclaveError::GenericError(e) => write!(f, "{e}"),
            EnclaveError::Unauthorized(e) => write!(f, "{e}"),
            EnclaveError::InternalError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EnclaveError {}

pub fn require_internal_auth(headers: &axum::http::HeaderMap) -> Result<(), EnclaveError> {
    let token = std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| EnclaveError::InternalError("ENCLAVE_INTERNAL_TOKEN must be defined".to_string()))?;
    let header_name = HeaderName::from_static("x-internal-token");
    let provided = headers
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| EnclaveError::Unauthorized("Missing X-Internal-Token".to_string()))?;

    if provided != token {
        return Err(EnclaveError::Unauthorized(
            "Invalid internal auth token".to_string(),
        ));
    }

    Ok(())
}
