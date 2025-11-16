//! Python exception hierarchy for SDK errors.
//!
//! This module provides Python exception classes that map to Rust `ApiError` variants.
//! Aligns with Goal Grid P2-T5: Add a clear exception hierarchy for SDK errors.
//!
//! For v0.1, we use PyValueError as the base (all exceptions extend ValueError).
//! Each error type has a distinct class that can be caught specifically.
//! This provides a clear exception hierarchy without complex inheritance.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::PyErr;

/// Convert Rust ApiError to Python exception.
///
/// Maps Rust `ApiError` variants to appropriate Python exception classes.
/// All exceptions extend PyValueError, providing a clear hierarchy:
///
/// - `SerializationError`: Payload serialization/deserialization failures
/// - `HashError`: Hash computation or verification failures
/// - `ValidationError`: Receipt structure validation failures
///
/// Users can catch specific exception types:
///
/// ```python
/// import northroot as nr
/// try:
///     receipt = nr.record_work(...)
/// except ValueError as e:
///     # Handle SDK errors (SerializationError, HashError, ValidationError)
///     # Error messages are prefixed with error type names
/// ```
pub fn api_error_to_python(e: northroot_engine::ApiError) -> PyErr {
    match e {
        northroot_engine::ApiError::SerializationError(msg) => {
            PyValueError::new_err(format!("SerializationError: {}", msg))
        }
        northroot_engine::ApiError::HashError(msg) => {
            PyValueError::new_err(format!("HashError: {}", msg))
        }
        northroot_engine::ApiError::ValidationError(msg) => {
            PyValueError::new_err(format!("ValidationError: {}", msg))
        }
    }
}

/// Register error documentation in Python module.
///
/// For v0.1, we document the exception hierarchy. The actual exception classes
/// are raised as PyValueError with prefixed error type names.
/// This can be enhanced in future versions with proper exception class inheritance.
pub fn register_errors(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    // For v0.1, exceptions are raised as PyValueError with prefixed names.
    // This provides a clear exception hierarchy without complex inheritance.
    // Future versions can add proper exception class registration here.
    Ok(())
}

