//! Error types for receipt validation and processing.

use std::fmt;

/// Errors that can occur during receipt validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Hash mismatch: computed hash does not match stored hash
    HashMismatch {
        /// Expected hash value (from receipt)
        expected: String,
        /// Computed hash value (from canonical body)
        computed: String,
    },
    /// Invalid hash format: does not match `^sha256:[0-9a-f]{64}$`
    InvalidHashFormat(String),
    /// Schema validation failure
    SchemaViolation(String),
    /// Composition error: sequential chain has mismatched cod/dom
    CompositionError {
        /// Index of the receipt in the chain where the error occurred
        receipt_index: usize,
        /// Expected codomain hash from previous receipt
        expected_cod: String,
        /// Actual domain hash from next receipt
        actual_dom: String,
    },
    /// Missing required field
    MissingField(String),
    /// Invalid field value
    InvalidValue(String),
    /// Serialization/deserialization error
    SerializationError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::HashMismatch { expected, computed } => {
                write!(
                    f,
                    "Hash mismatch: expected {}, computed {}",
                    expected, computed
                )
            }
            ValidationError::InvalidHashFormat(hash) => {
                write!(
                    f,
                    "Invalid hash format: {} (must match ^sha256:[0-9a-f]{{64}}$)",
                    hash
                )
            }
            ValidationError::SchemaViolation(msg) => {
                write!(f, "Schema violation: {}", msg)
            }
            ValidationError::CompositionError {
                receipt_index,
                expected_cod,
                actual_dom,
            } => {
                write!(
                    f,
                    "Composition error at receipt {}: expected cod {}, got dom {}",
                    receipt_index, expected_cod, actual_dom
                )
            }
            ValidationError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ValidationError::InvalidValue(msg) => {
                write!(f, "Invalid value: {}", msg)
            }
            ValidationError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ValidationError {}
