// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use axum::http::header::HeaderName;
use fastcrypto::ed25519::Ed25519KeyPair;

pub mod sessions;
pub mod streams;
pub mod bcs_utils;
pub mod error;
pub mod gcp_attestation;
pub mod handlers;

pub use error::EnclaveError;

use redis::aio::ConnectionManager;

/// App state, at minimum needs to maintain the ephemeral keypair.  
pub struct AppState {
    /// Ephemeral keypair on boot
    pub eph_kp: Ed25519KeyPair,
    /// Redis connection manager
    pub redis: ConnectionManager,
}

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
