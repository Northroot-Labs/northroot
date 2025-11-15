//! Thin client for Northroot SDK.
//!
//! Provides a simple, ergonomic interface for recording work and verifying receipts.
//! Storage is decoupled - the client can optionally use a storage backend, but
//! receipts can be created and verified without any storage dependency.
//!
//! ## Usage
//!
//! ```python
//! from northroot import Client
//! import northroot as nr
//!
//! # Simple usage (no storage)
//! receipt = nr.record_work(
//!     workload_id="normalize-prices",
//!     payload={"input": "data"},
//!     tags=["etl"]
//! )
//!
//! # With storage (optional)
//! client = Client(storage_path="./receipts")
//! receipt = client.record_work(...)
//! client.store(receipt)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

use crate::receipts::{self, PyReceipt};
use northroot_storage::{FilesystemStore, ReceiptQuery, ReceiptStore};

/// Thin client for Northroot SDK.
///
/// Provides a simple interface for recording work and verifying receipts.
/// Storage is optional and decoupled - receipts can be created without storage.
#[pyclass]
pub struct Client {
    // Optional storage backend for persisting and querying receipts
    store: Option<Arc<FilesystemStore>>,
}

#[pymethods]
impl Client {
    /// Create a new client.
    ///
    /// # Arguments
    ///
    /// * `storage_path` - Optional path for local storage (filesystem-based)
    ///   If None, receipts are not persisted (can still be created and verified)
    ///
    /// # Example
    ///
    /// ```python
    /// # Client without storage
    /// client = nr.Client()
    ///
    /// # Client with filesystem storage
    /// client = nr.Client(storage_path="./receipts")
    /// ```
    #[new]
    #[pyo3(signature = (storage_path = None))]
    fn new(storage_path: Option<String>) -> PyResult<Self> {
        let store = if let Some(path) = storage_path {
            Some(Arc::new(
                FilesystemStore::new(&path).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Failed to create filesystem store: {}",
                        e
                    ))
                })?,
            ))
        } else {
            None
        };
        Ok(Self { store })
    }

    /// Record a unit of work and produce a verifiable receipt.
    ///
    /// This is the primary method for creating receipts. It accepts
    /// a workload identifier, payload data, optional tags, and optional trace/parent
    /// IDs for causal composition.
    ///
    /// # Arguments
    ///
    /// * `workload_id` - Identifier for this unit of work (required)
    /// * `payload` - Work payload as a dictionary (required)
    /// * `tags` - Optional tags for categorization
    /// * `trace_id` - Optional trace ID for grouping related work units
    /// * `parent_id` - Optional parent receipt ID for DAG composition
    ///
    /// # Returns
    ///
    /// A `PyReceipt` containing the verifiable proof of work.
    ///
    /// # Example
    ///
    /// ```python
    /// receipt = client.record_work(
    ///     workload_id="normalize-prices",
    ///     payload={"input_hash": "...", "output_hash": "..."},
    ///     tags=["etl", "batch"],
    ///     trace_id="daily-etl-2025-01-17"
    /// )
    /// ```
    #[pyo3(signature = (workload_id, payload, tags = None, trace_id = None, parent_id = None))]
    fn record_work(
        &self,
        workload_id: String,
        payload: &Bound<'_, PyDict>,
        tags: Option<Vec<String>>,
        trace_id: Option<String>,
        parent_id: Option<String>,
    ) -> PyResult<PyReceipt> {
        // Delegate to module-level function
        record_work_py(workload_id, payload, tags, trace_id, parent_id)
    }

    /// Verify receipt integrity and hash correctness.
    ///
    /// # Arguments
    ///
    /// * `receipt` - The receipt to verify
    ///
    /// # Returns
    ///
    /// `True` if the receipt is valid, `False` if invalid.
    ///
    /// # Example
    ///
    /// ```python
    /// is_valid = client.verify_receipt(receipt)
    /// if is_valid:
    ///     print(f"Receipt {receipt.get_rid()} is valid")
    /// ```
    fn verify_receipt(&self, receipt: &PyReceipt) -> PyResult<bool> {
        // Delegate to module-level function
        verify_receipt_py(receipt)
    }

    /// Store a receipt in the configured storage backend.
    ///
    /// # Arguments
    ///
    /// * `receipt` - The receipt to store
    ///
    /// # Returns
    ///
    /// Nothing on success, raises an error if storage is not configured or storage fails.
    ///
    /// # Example
    ///
    /// ```python
    /// client = Client(storage_path="./receipts")
    /// receipt = client.record_work("workload-id", {"data": "value"})
    /// client.store_receipt(receipt)  # Persist to filesystem
    /// ```
    fn store_receipt(&self, receipt: &PyReceipt) -> PyResult<()> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Storage not configured. Create client with storage_path to enable storage."
            ))?;

        let receipt_inner = receipt.inner();
        store.store_receipt(receipt_inner).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to store receipt: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// List receipts matching the query criteria.
    ///
    /// # Arguments
    ///
    /// * `workload_id` - Optional filter by workload ID
    /// * `trace_id` - Optional filter by trace ID
    /// * `limit` - Optional maximum number of results
    ///
    /// # Returns
    ///
    /// A list of `PyReceipt` objects matching the query.
    ///
    /// # Example
    ///
    /// ```python
    /// client = Client(storage_path="./receipts")
    /// # List all receipts
    /// all_receipts = client.list_receipts()
    /// # Filter by workload
    /// etl_receipts = client.list_receipts(workload_id="normalize-prices")
    /// # Filter by trace
    /// trace_receipts = client.list_receipts(trace_id="daily-etl-2025-01-17")
    /// ```
    #[pyo3(signature = (workload_id = None, trace_id = None, limit = None))]
    fn list_receipts(
        &self,
        workload_id: Option<String>,
        trace_id: Option<String>,
        limit: Option<usize>,
    ) -> PyResult<Vec<PyReceipt>> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Storage not configured. Create client with storage_path to enable listing."
            ))?;

        let query = ReceiptQuery {
            workload_id,
            trace_id,
            limit,
            ..Default::default()
        };

        let receipts = store.query_receipts(query).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to query receipts: {}",
                e
            ))
        })?;

        Ok(receipts
            .into_iter()
            .map(|r| PyReceipt::from_inner(r))
            .collect())
    }
    
    // Note: Async methods (record_work_async, verify_receipt_async) are implemented
    // in Python (northroot/__init__.py) via monkey-patching. This keeps the Rust code
    // simple and avoids complex type conversions.
}

/// Module-level function: Record a unit of work.
///
/// This is the direct API for recording work without creating a client.
/// Storage is completely optional - receipts can be created and verified
/// without any storage backend.
///
/// # Arguments
///
/// * `workload_id` - Identifier for this unit of work (required)
/// * `payload` - Work payload as a dictionary (required)
/// * `tags` - Optional tags for categorization
/// * `trace_id` - Optional trace ID for grouping related work units
/// * `parent_id` - Optional parent receipt ID for DAG composition
///
/// # Returns
///
/// A `PyReceipt` containing the verifiable proof of work.
///
/// # Example
///
/// ```python
/// import northroot as nr
///
/// receipt = nr.record_work(
///     workload_id="normalize-prices",
///     payload={"input": "data"},
///     tags=["etl"]
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (workload_id, payload, tags = None, trace_id = None, parent_id = None))]
pub fn record_work_py(
    workload_id: String,
    payload: &Bound<'_, PyDict>,
    tags: Option<Vec<String>>,
    trace_id: Option<String>,
    parent_id: Option<String>,
) -> PyResult<PyReceipt> {
    // Use the receipts module implementation (storage-agnostic)
    receipts::record_work_py(workload_id, payload, tags, trace_id, parent_id)
}

/// Module-level function: Verify receipt integrity.
///
/// This is the direct API for verifying receipts without creating a client.
/// Storage is not required - receipts can be verified independently.
///
/// # Arguments
///
/// * `receipt` - The receipt to verify
///
/// # Returns
///
/// `True` if the receipt is valid, `False` if invalid.
///
/// # Example
///
/// ```python
/// import northroot as nr
///
/// is_valid = nr.verify_receipt(receipt)
/// ```
#[pyfunction]
pub fn verify_receipt_py(receipt: &PyReceipt) -> PyResult<bool> {
    receipts::verify_receipt_py(receipt)
}


