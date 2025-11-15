//! Data shape Python bindings
//!
//! This module exposes data shape computation to Python:
//! - Data shape hash computation
//! - Method shape hash computation
//! - Shape hash from files/bytes

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pymodule;
use std::path::Path;

use northroot_engine::delta::{
    compute_data_shape_hash_from_bytes, compute_data_shape_hash_from_file,
    compute_method_shape_hash_from_code, compute_method_shape_hash_from_signature,
};
use northroot_engine::ChunkScheme;

/// Convert Python dict to ChunkScheme
///
/// Accepts:
/// - A dict with "type": "cdc", "avg_size": int
/// - A dict with "type": "fixed", "size": int
/// - None (defaults to CDC with avg_size=65536)
fn dict_to_chunk_scheme(dict: Option<&Bound<'_, PyDict>>) -> PyResult<Option<ChunkScheme>> {
    let dict = match dict {
        Some(d) => d,
        None => return Ok(None),
    };

    let type_str = dict
        .get_item("type")?
        .and_then(|t| t.extract::<String>().ok())
        .ok_or_else(|| PyValueError::new_err("Missing 'type' field in chunk_scheme"))?;

    match type_str.as_str() {
        "cdc" => {
            let avg_size = dict
                .get_item("avg_size")?
                .and_then(|v| v.extract::<u64>().ok())
                .ok_or_else(|| PyValueError::new_err("Missing 'avg_size' field in chunk_scheme"))?;
            Ok(Some(ChunkScheme::CDC { avg_size }))
        }
        "fixed" => {
            let size = dict
                .get_item("size")?
                .and_then(|v| v.extract::<u64>().ok())
                .ok_or_else(|| PyValueError::new_err("Missing 'size' field in chunk_scheme"))?;
            Ok(Some(ChunkScheme::Fixed { size }))
        }
        _ => Err(PyValueError::new_err(format!(
            "Invalid chunk_scheme type '{}', expected 'cdc' or 'fixed'",
            type_str
        ))),
    }
}

/// Compute data shape hash from a file.
///
/// Reads the file, chunks it, builds a manifest, and computes the data shape hash.
///
/// Args:
///     path: Path to the file (str or PathLike)
///     chunk_scheme: Optional chunking scheme dict:
///         - {"type": "cdc", "avg_size": int} for CDC
///         - {"type": "fixed", "size": int} for fixed-size
///         - None (defaults to CDC with avg_size=65536)
///
/// Returns:
///     Data shape hash in format `sha256:<64hex>`
///
/// Raises:
///     ValueError: If file cannot be read or shape computation fails
#[pyfunction]
#[pyo3(signature = (path, chunk_scheme = None))]
fn compute_data_shape_hash_from_file_py(
    path: String,
    chunk_scheme: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    let scheme = dict_to_chunk_scheme(chunk_scheme)?;
    compute_data_shape_hash_from_file(Path::new(&path), scheme)
        .map_err(|e| PyValueError::new_err(format!("Failed to compute data shape hash: {}", e)))
}

/// Compute data shape hash from bytes.
///
/// Chunks the bytes, builds a manifest, and computes the data shape hash.
///
/// Args:
///     data: Input data bytes (bytes or bytearray)
///     chunk_scheme: Optional chunking scheme dict:
///         - {"type": "cdc", "avg_size": int} for CDC
///         - {"type": "fixed", "size": int} for fixed-size
///         - None (defaults to CDC with avg_size=65536)
///
/// Returns:
///     Data shape hash in format `sha256:<64hex>`
///
/// Raises:
///     ValueError: If manifest building or shape computation fails
#[pyfunction]
#[pyo3(signature = (data, chunk_scheme = None))]
fn compute_data_shape_hash_from_bytes_py(
    data: &[u8],
    chunk_scheme: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    let scheme = dict_to_chunk_scheme(chunk_scheme)?;
    compute_data_shape_hash_from_bytes(data, scheme)
        .map_err(|e| PyValueError::new_err(format!("Failed to compute data shape hash: {}", e)))
}

/// Compute method shape hash from code hash and parameters.
///
/// Creates a method shape hash from a code hash (e.g., SHA-256 of the method
/// implementation) and optional parameters. The hash represents the method's
/// computational contract.
///
/// Args:
///     code_hash: SHA-256 hash of the method code (format: "sha256:<64hex>" or just hex)
///     params: Optional parameters as dict (will be serialized to JSON)
///
/// Returns:
///     Method shape hash in format `sha256:<64hex>`
///
/// Raises:
///     ValueError: If serialization fails or hash format is invalid
#[pyfunction]
#[pyo3(signature = (code_hash, params = None))]
fn compute_method_shape_hash_from_code_py(
    code_hash: String,
    params: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    let params_value = if let Some(params_dict) = params {
        // Convert Python dict to JSON string, then parse to serde_json::Value
        Python::with_gil(|py| {
            let json_module = py.import_bound("json")?;
            let json_str = json_module
                .call_method1("dumps", (params_dict,))?
                .extract::<String>()?;
            serde_json::from_str(&json_str)
                .map_err(|e| PyValueError::new_err(format!("Failed to parse params JSON: {}", e)))
        })?
    } else {
        None
    };

    compute_method_shape_hash_from_code(&code_hash, params_value.as_ref())
        .map_err(|e| PyValueError::new_err(format!("Failed to compute method shape hash: {}", e)))
}

/// Compute method shape hash from function signature.
///
/// Creates a method shape hash from a function signature, including function
/// name, input types, and output type. This is useful for statically-typed
/// methods where the signature uniquely identifies the method contract.
///
/// Args:
///     function_name: Name of the function/method
///     input_types: List of input type names
///     output_type: Output type name
///
/// Returns:
///     Method shape hash in format `sha256:<64hex>`
///
/// Raises:
///     ValueError: If serialization fails or function name is empty
#[pyfunction]
fn compute_method_shape_hash_from_signature_py(
    function_name: String,
    input_types: Vec<String>,
    output_type: String,
) -> PyResult<String> {
    let input_types_str: Vec<&str> = input_types.iter().map(|s| s.as_str()).collect();
    compute_method_shape_hash_from_signature(&function_name, &input_types_str, &output_type)
        .map_err(|e| PyValueError::new_err(format!("Failed to compute method shape hash: {}", e)))
}

/// Python bindings for shape operations
#[pymodule]
fn shapes(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_data_shape_hash_from_file_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_data_shape_hash_from_bytes_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_method_shape_hash_from_code_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_method_shape_hash_from_signature_py, m)?)?;
    Ok(())
}

/// Register shapes module
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_wrapped(wrap_pymodule!(shapes))?;
    Ok(())
}

