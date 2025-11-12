//! Receipt Python bindings
//!
//! This module exposes receipt generation and validation to Python:
//! - Receipt creation from JSON
//! - Receipt validation
//! - Receipt hash computation
//! - Receipt serialization (CBOR/JSON)

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pymodule;

use northroot_receipts::{
    adapters::json::{receipt_from_json, receipt_to_json},
    Receipt,
};

/// Python wrapper for Receipt
#[pyclass]
pub struct PyReceipt {
    receipt: Receipt,
}

#[pymethods]
impl PyReceipt {
    /// Validate this receipt.
    ///
    /// Performs hash integrity, payload rules, and format checks.
    ///
    /// Returns:
    ///     None if valid
    ///
    /// Raises:
    ///     ValueError: If validation fails
    fn validate(&self) -> PyResult<()> {
        self.receipt
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Receipt validation failed: {}", e)))
    }

    /// Compute hash from canonical body.
    ///
    /// Computes the SHA-256 hash of the canonical JSON representation
    /// (without `sig` and `hash` fields).
    ///
    /// Returns:
    ///     Hash in format `sha256:<64hex>`
    ///
    /// Raises:
    ///     ValueError: If hash computation fails
    fn compute_hash(&self) -> PyResult<String> {
        self.receipt
            .compute_hash()
            .map_err(|e| PyValueError::new_err(format!("Hash computation failed: {}", e)))
    }

    /// Serialize receipt to JSON string.
    ///
    /// Returns:
    ///     JSON string representation of the receipt
    ///
    /// Raises:
    ///     ValueError: If serialization fails
    fn to_json(&self) -> PyResult<String> {
        receipt_to_json(&self.receipt)
            .map_err(|e| PyValueError::new_err(format!("JSON serialization failed: {}", e)))
    }

    /// Get receipt ID (RID) as string.
    fn get_rid(&self) -> String {
        self.receipt.rid.to_string()
    }

    /// Get receipt kind as string.
    fn get_kind(&self) -> String {
        format!("{:?}", self.receipt.kind)
    }

    /// Get receipt version.
    fn get_version(&self) -> String {
        self.receipt.version.clone()
    }

    /// Get receipt hash.
    fn get_hash(&self) -> String {
        self.receipt.hash.clone()
    }
}

/// Create a receipt from JSON string.
///
/// Args:
///     json_str: JSON string representation of the receipt
///
/// Returns:
///     PyReceipt object
///
/// Raises:
///     ValueError: If JSON parsing or receipt creation fails
#[pyfunction]
fn receipt_from_json_py(json_str: String) -> PyResult<PyReceipt> {
    let receipt = receipt_from_json(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to create receipt from JSON: {}", e)))?;
    Ok(PyReceipt { receipt })
}

/// Python bindings for receipt operations
#[pymodule]
fn receipts(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyReceipt>()?;
    m.add_function(wrap_pyfunction!(receipt_from_json_py, m)?)?;
    Ok(())
}

/// Register receipts module
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_wrapped(wrap_pymodule!(receipts))?;
    Ok(())
}

