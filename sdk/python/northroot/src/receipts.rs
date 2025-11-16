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

use northroot_engine::api::verify_receipt as verify_receipt_rust;
use northroot_receipts::{
    adapters::json::{receipt_from_json, receipt_to_json},
    Receipt,
};
use pyo3::types::PyDict;

use crate::errors::api_error_to_python;

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

    /// Get domain shape commitment (dom).
    fn get_dom(&self) -> String {
        self.receipt.dom.clone()
    }

    /// Get codomain shape commitment (cod).
    fn get_cod(&self) -> String {
        self.receipt.cod.clone()
    }

    /// Get workload ID from execution payload (if this is an execution receipt).
    ///
    /// Returns None if this is not an execution receipt or if workload_id is not available.
    fn get_workload_id(&self) -> Option<String> {
        // For now, execution receipts don't have a direct workload_id field
        // It's stored in the payload. We'll need to extract it from the payload.
        // For v0.1, we can return None and let the demo work around it
        None
    }

    /// Get trace ID from execution payload (if this is an execution receipt).
    fn get_trace_id(&self) -> Option<String> {
        match &self.receipt.payload {
            northroot_receipts::Payload::Execution(exec) => Some(exec.trace_id.clone()),
            _ => None,
        }
    }
}

impl PyReceipt {
    /// Get the inner Receipt (for internal use)
    pub fn inner(&self) -> &Receipt {
        &self.receipt
    }

    /// Create a PyReceipt from an inner Receipt (for internal use)
    pub fn from_inner(receipt: Receipt) -> Self {
        Self { receipt }
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

/// Record a unit of work and produce a verifiable receipt.
///
/// This is the primary SDK entry point for creating receipts. It accepts
/// a workload identifier, payload data, optional tags, and optional trace/parent
/// IDs for causal composition.
///
/// Args:
///     workload_id: Identifier for this unit of work (e.g., "normalize-prices", "train-model")
///     payload: Work payload as a dictionary (will be hashed to compute data shape)
///     tags: Optional tags for categorization (e.g., ["etl", "batch"])
///     trace_id: Optional trace ID for grouping related work units
///     parent_id: Optional parent receipt ID for DAG composition
///
/// Returns:
///     PyReceipt object containing the verifiable proof of work
///
/// Raises:
///     ValueError: If receipt creation fails
///
/// Example:
///     >>> import northroot as nr
///     >>> receipt = nr.record_work(
///     ...     "normalize-prices",
///     ...     {"input_hash": "...", "output_hash": "..."},
///     ...     tags=["etl"],
///     ...     trace_id="trace-123",
///     ...     parent_id=None
///     ... )
#[pyfunction]
#[pyo3(signature = (workload_id, payload, tags = None, trace_id = None, parent_id = None))]
pub fn record_work_py(
    workload_id: String,
    payload: &Bound<'_, PyDict>,
    tags: Option<Vec<String>>,
    trace_id: Option<String>,
    parent_id: Option<String>,
) -> PyResult<PyReceipt> {
    // Convert Python dict to serde_json::Value
    // Use Python's json module to serialize, then deserialize to serde_json::Value
    let py = payload.py();
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module
        .call_method1("dumps", (payload,))?
        .extract()?;
    let payload_json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse payload JSON: {}", e)))?;

    // Convert tags
    let tags_vec = tags.unwrap_or_default();

    // Call Rust API
    let receipt = northroot_engine::api::record_work(
        &workload_id,
        payload_json,
        tags_vec,
        trace_id,
        parent_id,
    )
    .map_err(api_error_to_python)?;

    Ok(PyReceipt { receipt })
}

/// Verify receipt integrity and hash correctness.
///
/// This performs basic verification:
/// - Hash integrity: receipt.hash == compute_hash(receipt)
/// - Syntactic validation: receipt structure is well-formed
///
/// Args:
///     receipt: PyReceipt object to verify
///
/// Returns:
///     True if receipt is valid, False if invalid
///
/// Raises:
///     ValueError: If verification fails due to error (not invalid receipt)
///
/// Example:
///     >>> import northroot as nr
///     >>> is_valid = nr.verify_receipt(receipt)
///     >>> if is_valid:
///     ...     print(f"Receipt {receipt.get_rid()} is valid")
#[pyfunction]
pub fn verify_receipt_py(receipt: &PyReceipt) -> PyResult<bool> {
    verify_receipt_rust(&receipt.receipt).map_err(api_error_to_python)
}

/// Python bindings for receipt operations
#[pymodule]
fn receipts(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyReceipt>()?;
    m.add_function(wrap_pyfunction!(receipt_from_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(record_work_py, m)?)?;
    m.add_function(wrap_pyfunction!(verify_receipt_py, m)?)?;
    Ok(())
}

/// Register receipts module
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_wrapped(wrap_pymodule!(receipts))?;
    Ok(())
}

