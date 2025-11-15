//! Storage error types.

use thiserror::Error;

/// Storage operation errors.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database operation error
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Compression/decompression error
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input parameter
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),
}
