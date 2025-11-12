//! Helper functions for method shape hash computation.
//!
//! This module provides convenience functions for computing method shape hashes
//! from code, signatures, or method shape payloads for PAC key computation.
//!
//! This is part of ADR-0009-P08: Helper Functions for Shape Hash Computation.

use northroot_receipts::MethodShapePayload;
use serde_json::Value;

use crate::commitments::{jcs, sha256_prefixed};

/// Error type for method shape operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodShapeError {
    /// Invalid hash format
    InvalidHashFormat(String),
    /// Serialization error
    Serialization(String),
    /// Invalid input
    InvalidInput(String),
}

impl std::fmt::Display for MethodShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodShapeError::InvalidHashFormat(msg) => {
                write!(f, "Invalid hash format: {}", msg)
            }
            MethodShapeError::Serialization(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            MethodShapeError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
        }
    }
}

impl std::error::Error for MethodShapeError {}

/// Compute method shape hash from code hash and parameters.
///
/// This function creates a method shape hash from a code hash (e.g., SHA-256 of
/// the method implementation) and optional parameters. The hash represents the
/// method's computational contract.
///
/// # Arguments
///
/// * `code_hash` - SHA-256 hash of the method code (format: "sha256:<64hex>" or just hex)
/// * `params` - Optional parameters as JSON value
///
/// # Returns
///
/// Method shape hash in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if serialization fails or hash format is invalid
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_method_shape_hash_from_code;
/// use serde_json::json;
///
/// let code_hash = "sha256:abc123...";
/// let params = json!({ "batch_size": 1000 });
/// let hash = compute_method_shape_hash_from_code(code_hash, Some(&params)).unwrap();
/// ```
pub fn compute_method_shape_hash_from_code(
    code_hash: &str,
    params: Option<&Value>,
) -> Result<String, MethodShapeError> {
    // Normalize code hash (remove "sha256:" prefix if present)
    let code_hash_hex = code_hash.strip_prefix("sha256:").unwrap_or(code_hash);

    // Validate hex format
    if code_hash_hex.len() != 64 || !code_hash_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(MethodShapeError::InvalidHashFormat(format!(
            "Invalid code hash format: {}",
            code_hash
        )));
    }

    // Build canonical representation
    let canonical = if let Some(params) = params {
        serde_json::json!({
            "code_hash": format!("sha256:{}", code_hash_hex),
            "params": params,
        })
    } else {
        serde_json::json!({
            "code_hash": format!("sha256:{}", code_hash_hex),
        })
    };

    // JCS canonicalization and hash
    let jcs_bytes = jcs(&canonical);
    Ok(sha256_prefixed(jcs_bytes.as_bytes()))
}

/// Compute method shape hash from function signature.
///
/// This function creates a method shape hash from a function signature,
/// including function name, input types, and output type. This is useful
/// for statically-typed methods where the signature uniquely identifies
/// the method contract.
///
/// # Arguments
///
/// * `function_name` - Name of the function/method
/// * `input_types` - Array of input type names
/// * `output_type` - Output type name
///
/// # Returns
///
/// Method shape hash in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if serialization fails
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_method_shape_hash_from_signature;
///
/// let hash = compute_method_shape_hash_from_signature(
///     "normalize_ledger",
///     &["Vec<Transaction>", "Config"],
///     "Ledger"
/// ).unwrap();
/// ```
pub fn compute_method_shape_hash_from_signature(
    function_name: &str,
    input_types: &[&str],
    output_type: &str,
) -> Result<String, MethodShapeError> {
    if function_name.is_empty() {
        return Err(MethodShapeError::InvalidInput(
            "Function name cannot be empty".to_string(),
        ));
    }

    // Build canonical representation
    let canonical = serde_json::json!({
        "function_name": function_name,
        "input_types": input_types,
        "output_type": output_type,
    });

    // JCS canonicalization and hash
    let jcs_bytes = jcs(&canonical);
    Ok(sha256_prefixed(jcs_bytes.as_bytes()))
}

/// Compute method shape root from MethodShapePayload.
///
/// This is a convenience function that extracts or computes the method shape root
/// from a MethodShapePayload. The root is typically the `root_multiset` field,
/// but this function provides a consistent interface.
///
/// # Arguments
///
/// * `payload` - MethodShapePayload to extract root from
///
/// # Returns
///
/// Method shape root (same as root_multiset) in format `sha256:<64hex>`
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_method_shape_root_from_payload;
/// use northroot_receipts::MethodShapePayload;
///
/// let payload = MethodShapePayload { ... };
/// let root = compute_method_shape_root_from_payload(&payload).unwrap();
/// ```
pub fn compute_method_shape_root_from_payload(
    payload: &MethodShapePayload,
) -> Result<String, MethodShapeError> {
    // The method shape root is the root_multiset
    // Validate format
    if !payload.root_multiset.starts_with("sha256:") || payload.root_multiset.len() != 71 {
        return Err(MethodShapeError::InvalidHashFormat(format!(
            "Invalid root_multiset format: {}",
            payload.root_multiset
        )));
    }

    Ok(payload.root_multiset.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compute_method_shape_hash_from_code() {
        let code_hash = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let hash = compute_method_shape_hash_from_code(code_hash, None).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_compute_method_shape_hash_from_code_with_params() {
        let code_hash = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let params = json!({ "batch_size": 1000 });
        let hash = compute_method_shape_hash_from_code(code_hash, Some(&params)).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_method_shape_hash_from_code_without_prefix() {
        let code_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let hash = compute_method_shape_hash_from_code(code_hash, None).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_method_shape_hash_from_signature() {
        let hash = compute_method_shape_hash_from_signature(
            "normalize_ledger",
            &["Vec<Transaction>"],
            "Ledger",
        )
        .unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_compute_method_shape_hash_from_signature_empty_name() {
        let result = compute_method_shape_hash_from_signature("", &["Vec<Transaction>"], "Ledger");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_method_shape_hash_deterministic() {
        let hash1 =
            compute_method_shape_hash_from_signature("test_func", &["i32", "String"], "bool")
                .unwrap();
        let hash2 =
            compute_method_shape_hash_from_signature("test_func", &["i32", "String"], "bool")
                .unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_method_shape_hash_different_signatures() {
        let hash1 = compute_method_shape_hash_from_signature("func1", &["i32"], "bool").unwrap();
        let hash2 = compute_method_shape_hash_from_signature("func2", &["i32"], "bool").unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_method_shape_root_from_payload() {
        let payload = MethodShapePayload {
            nodes: vec![],
            edges: None,
            root_multiset:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            dag_hash: None,
        };
        let root = compute_method_shape_root_from_payload(&payload).unwrap();
        assert_eq!(root, payload.root_multiset);
    }
}
