//! Execution state management.
//!
//! This module provides utilities for managing trace IDs, span commitments,
//! and execution root computation.

use crate::commitments::{commit_seq_root, commit_set_root};
use northroot_receipts::{ExecutionRoots, MethodRef, ValidationError};

/// Validate method reference structure.
///
/// Checks that the method reference has valid format and non-empty fields.
///
/// # Arguments
///
/// * `method_ref` - Method reference to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `ValidationError` if invalid
pub fn validate_method_ref(method_ref: &MethodRef) -> Result<(), ValidationError> {
    if method_ref.method_id.is_empty() {
        return Err(ValidationError::InvalidValue(
            "method_id cannot be empty".to_string(),
        ));
    }

    if method_ref.version.is_empty() {
        return Err(ValidationError::InvalidValue(
            "version cannot be empty".to_string(),
        ));
    }

    if !method_ref.method_shape_root.starts_with("sha256:")
        || method_ref.method_shape_root.len() != 71
    {
        return Err(ValidationError::InvalidHashFormat(
            method_ref.method_shape_root.clone(),
        ));
    }

    Ok(())
}

/// Compute execution roots from span commitments.
///
/// Computes trace_set_root, trace_seq_root, and validates identity_root format.
/// Note: identity_root computation is handled by the receipts crate.
///
/// # Arguments
///
/// * `span_commitments` - Vector of span commitment hashes
/// * `identity_root` - Identity root (computed separately)
///
/// # Returns
///
/// Execution roots structure
pub fn compute_execution_roots(
    span_commitments: &[String],
    identity_root: String,
) -> ExecutionRoots {
    let trace_set_root = commit_set_root(span_commitments);
    let trace_seq_root = Some(commit_seq_root(span_commitments));

    ExecutionRoots {
        trace_set_root,
        identity_root,
        trace_seq_root,
    }
}

/// Generate a trace ID.
///
/// Creates a deterministic trace ID from a seed or generates a UUID-based one.
/// For now, this is a simple wrapper; in production, this might use UUIDv7
/// for time-ordered trace IDs.
///
/// # Arguments
///
/// * `seed` - Optional seed for deterministic trace ID generation
///
/// # Returns
///
/// Trace ID string
pub fn generate_trace_id(seed: Option<&str>) -> String {
    if let Some(seed) = seed {
        // Deterministic trace ID from seed
        crate::commitments::sha256_prefixed(seed.as_bytes())
    } else {
        // Generate UUID-based trace ID
        // Note: In production, use UUIDv7 for time-ordered IDs
        format!(
            "trace:{}",
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_receipts::MethodRef;

    #[test]
    fn test_validate_method_ref_valid() {
        let method_ref = MethodRef {
            method_id: "com.acme/normalize-ledger".to_string(),
            version: "1.0.0".to_string(),
            method_shape_root:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_string(),
        };

        assert!(validate_method_ref(&method_ref).is_ok());
    }

    #[test]
    fn test_validate_method_ref_empty_id() {
        let method_ref = MethodRef {
            method_id: String::new(),
            version: "1.0.0".to_string(),
            method_shape_root:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_string(),
        };

        assert!(validate_method_ref(&method_ref).is_err());
    }

    #[test]
    fn test_validate_method_ref_invalid_hash() {
        let method_ref = MethodRef {
            method_id: "com.acme/test".to_string(),
            version: "1.0.0".to_string(),
            method_shape_root: "invalid".to_string(),
        };

        assert!(validate_method_ref(&method_ref).is_err());
    }

    #[test]
    fn test_compute_execution_roots() {
        let span_commitments = vec![
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        ];

        let identity_root =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

        let roots = compute_execution_roots(&span_commitments, identity_root.clone());

        assert!(roots.trace_set_root.starts_with("sha256:"));
        assert_eq!(roots.identity_root, identity_root);
        assert!(roots.trace_seq_root.is_some());
    }

    #[test]
    fn test_generate_trace_id() {
        let trace_id1 = generate_trace_id(Some("test-seed"));
        let trace_id2 = generate_trace_id(Some("test-seed"));
        let trace_id3 = generate_trace_id(Some("different-seed"));

        assert_eq!(trace_id1, trace_id2); // Deterministic
        assert_ne!(trace_id1, trace_id3); // Different seeds produce different IDs
        assert!(trace_id1.starts_with("sha256:"));
    }
}
