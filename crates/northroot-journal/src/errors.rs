use thiserror::Error;

/// Errors that can occur during journal operations.
#[derive(Error, Debug)]
pub enum JournalError {
    /// I/O error during read or write.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid file header (magic, version, or flags).
    #[error("invalid journal header: {0}")]
    InvalidHeader(String),
    /// Invalid frame structure (kind, reserved bytes, or length).
    #[error("invalid frame at offset {offset}: {reason}")]
    InvalidFrame {
        /// Byte offset where the frame starts.
        offset: u64,
        /// Reason for invalidity.
        reason: String,
    },
    /// Payload exceeds maximum size limit.
    #[error("payload size {size} exceeds maximum {max}")]
    PayloadTooLarge {
        /// Actual payload size.
        size: u32,
        /// Maximum allowed size.
        max: u32,
    },
    /// Invalid UTF-8 in EventJson payload.
    #[error("invalid UTF-8 in event payload: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    /// Invalid JSON in EventJson payload (from serde_json).
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    /// Invalid JSON in EventJson payload.
    #[error("invalid JSON in event payload: {0}")]
    InvalidJson(String),
    /// Attempted to write to a non-empty file without proper initialization.
    #[error("file is not empty; cannot initialize header")]
    FileNotEmpty,
    /// Truncated frame detected in strict mode.
    #[error("truncated frame at offset {offset}")]
    TruncatedFrame {
        /// Byte offset where truncation occurred.
        offset: u64,
    },
}
