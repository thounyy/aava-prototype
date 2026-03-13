// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use thiserror::Error;

/// Enclave errors enum.
#[derive(Debug, Error)]
pub enum EnclaveError {
    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Walrus error: {0}")]
    WalrusError(String),

    #[error("Attestation error: {0}")]
    AttestationError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for EnclaveError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            EnclaveError::RedisError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            EnclaveError::NotFound(e) => (StatusCode::NOT_FOUND, e.clone()),
            EnclaveError::InvalidInput(e) => (StatusCode::BAD_REQUEST, e.clone()),
            EnclaveError::ParseError(e) => (StatusCode::BAD_REQUEST, e.clone()),
            EnclaveError::WalrusError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            EnclaveError::AttestationError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            EnclaveError::HttpError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            EnclaveError::Unauthorized(e) => (StatusCode::UNAUTHORIZED, e.clone()),
            EnclaveError::InternalError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
