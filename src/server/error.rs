use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

/// The errors that can occur in the handlers.
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Invalid hex string: {0}")]
    InvalidHex(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Transaction receipt not found: {0}")]
    ReceiptNotFound(String),
    #[error("Blob data not found: {0}")]
    BlobDataNotFound(String),
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        let status = match self {
            HandlerError::InvalidHex(_) => StatusCode::BAD_REQUEST,
            HandlerError::ProviderError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::TransactionNotFound(_) => StatusCode::NOT_FOUND,
            HandlerError::ReceiptNotFound(_) => StatusCode::NOT_FOUND,
            HandlerError::BlobDataNotFound(_) => StatusCode::NOT_FOUND,
        };

        (status, self.to_string()).into_response()
    }
}
