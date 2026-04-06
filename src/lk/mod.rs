pub mod board_gen;
pub mod builder;
pub mod io;
pub mod schema;
pub mod store;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LkError {
    #[error("World not found: {0}")]
    WorldNotFound(String),
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    #[error("Calendar not found: {0}")]
    CalendarNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Draft resource not found: {0}")]
    DraftResourceNotFound(String),
    #[error("Draft document not found: {0}")]
    DraftDocumentNotFound(String),
}
