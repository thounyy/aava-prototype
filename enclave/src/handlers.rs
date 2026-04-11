use crate::gcp_attestation::fetch_jwt;
use crate::{AppState, EnclaveError};
use axum::{extract::State, Json};
use fastcrypto::traits::ToFromBytes;
use fastcrypto::{encoding::Encoding, traits::KeyPair as FcKeyPair};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Response for get attestation.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAttestationResponse {
    /// Google-signed OIDC JWT from Confidential Space attestation server.
    pub jwt: String,
    /// Base64-encoded enclave ephemeral public key.
    pub public_key: String,
    /// Hex-encoded enclave ephemeral public key.
    pub public_key_hex: String,
}

/// Endpoint that returns an attestation committed to the enclave's public key.
pub async fn get_attestation(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GetAttestationResponse>, EnclaveError> {
    info!("get attestation called");

    let pk = state.eph_kp.public();
    // Keep nonce derivation aligned with the on-chain contract:
    // eat_nonce[0] = UTF-8 bytes of hex(32-byte Ed25519 public key).
    let nonce = fastcrypto::encoding::Hex::encode(pk.as_bytes());
    let jwt = fetch_jwt("https://sts.googleapis.com", vec![nonce])
        .await
        .map_err(EnclaveError::AttestationError)?
        .ok_or_else(|| {
            EnclaveError::AttestationError(
                "TEE socket not found: this endpoint only works in GCP Confidential Space"
                    .to_string(),
            )
        })?;

    Ok(Json(GetAttestationResponse {
        jwt,
        public_key: fastcrypto::encoding::Base64::encode(pk.as_bytes()),
        public_key_hex: fastcrypto::encoding::Hex::encode(pk.as_bytes()),
    }))
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Simple service status.
    pub status: &'static str,
    /// Hex encoded public key booted on enclave.
    pub pk: String,
}

/// Minimal health endpoint returning running status and enclave public key.
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthCheckResponse>, EnclaveError> {
    let pk = state.eph_kp.public();

    Ok(Json(HealthCheckResponse {
        status: "running",
        pk: fastcrypto::encoding::Hex::encode(pk.as_bytes()),
    }))
}
