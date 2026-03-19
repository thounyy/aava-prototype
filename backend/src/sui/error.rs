use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SuiError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Failed to build transaction: {0}")]
    BuildFailed(String),

    #[error("Failed to sign transaction: {0}")]
    SignFailed(String),

    #[error("Transaction failed on-chain: {0}")]
    OnChainFailed(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Unexpected response: {0}")]
    ParseError(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl SuiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::BuildFailed(_) | Self::SignFailed(_) | Self::ParseError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::OnChainFailed(_) | Self::RpcError(_) => StatusCode::BAD_GATEWAY,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}
