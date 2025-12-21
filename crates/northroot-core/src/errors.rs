use thiserror::Error;

/// Core error types.
#[derive(Error, Debug)]
pub enum CoreError {
    /// Event ID computation failed.
    #[error("event ID computation failed: {0}")]
    EventId(#[from] crate::event_id::EventIdError),
    /// Verification failed.
    #[error("verification failed: {0}")]
    Verification(String),
    /// Invalid event structure or missing required fields.
    #[error("invalid event: {0}")]
    InvalidEvent(String),
    /// Canonicalization error.
    #[error("canonicalization error: {0}")]
    Canonicalization(#[from] northroot_canonical::CanonicalizationError),
}

