//! Error types for store operations.

use thiserror::Error;

/// Errors that can occur during store operations.
#[derive(Error, Debug)]
pub enum StoreError {
    /// I/O error during read or write.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Journal backend error.
    #[error("journal error: {0}")]
    Journal(#[from] northroot_journal::JournalError),
    /// Payload exceeds 16 MiB limit.
    #[error("payload exceeds 16 MiB limit")]
    PayloadTooLarge,
    /// Other error.
    #[error("{0}")]
    Other(String),
}

