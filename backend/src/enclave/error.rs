use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnclaveError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(#[from] reqwest::Error),

    #[error("API error: HTTP {status} - {body}")]
    ApiError { status: u16, body: String },

    #[error("Parse error: {0}")]
    ParseError(String),
}
