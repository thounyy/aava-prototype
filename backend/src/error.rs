use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

use crate::enclave::error::EnclaveError;
use crate::sui::error::SuiError;
use crate::walrus::error::WalrusError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Sui(#[from] SuiError),

    #[error(transparent)]
    Enclave(#[from] EnclaveError),

    #[error(transparent)]
    Walrus(#[from] WalrusError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Sui(e) => e.status_code(),
            Self::Enclave(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Walrus(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = json!({ "error": self.to_string() });
        (status, axum::Json(body)).into_response()
    }
}
