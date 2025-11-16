//! Minimal SDK API facade for v0.1.
//!
//! This module provides a narrow, stable API surface for SDK consumers.
//! It hides engine complexity and exposes only the essential operations:
//! - `record_work`: Create a verifiable receipt for a unit of work
//! - `verify_receipt`: Verify receipt integrity and hash correctness
//!
//! ## Design Principles
//!
//! - **Narrow slice**: Only exposes verifiable proofs of work, not advanced reuse economics
//! - **Simple semantics**: workload_id = span-level work unit, trace_id = correlation ID
//! - **Composable**: DAGs built from spans via parent_id and receipt references
//! - **Future-proof**: Engine can evolve internally without breaking SDK contract
//!
//! ## Receipt Population Strategy
//!
//! **Core Fields (Always Populated):**
//! - Envelope: `rid`, `version`, `kind`, `dom`, `cod`, `links`, `hash`
//! - Context: `timestamp` (audit trail)
//! - Execution: `trace_id`, `method_ref`, `data_shape_hash`, `span_commitments`, `roots`
//!
//! **Optional Fields (Policy/Feature-Driven):**
//! - Context: `policy_ref`, `identity_ref`, `nonce`, `determinism` (set when policy enabled)
//! - Execution: `pac`, `change_epoch`, `output_mime_type`, `output_size_bytes` (set when needed)
//! - Execution: `input_locator_refs`, `output_locator_ref` (set when resolver used)
//! - All other fields: Reserved for future features, always `None` in v0.1

use northroot_receipts::{
    Context, DeterminismClass, ExecutionPayload, MethodRef, Payload, Receipt, ReceiptKind,
};
use uuid::Uuid;

use crate::commitments::sha256_prefixed;
use crate::execution::state::{compute_execution_roots, generate_trace_id};

/// Record a unit of work and produce a verifiable receipt.
///
/// This is the primary SDK entry point for creating receipts. It accepts
/// a workload identifier, payload data, optional tags, and optional trace/parent
/// IDs for causal composition.
///
/// # Arguments
///
/// * `workload_id` - Identifier for this unit of work (e.g., "normalize-prices", "train-model")
/// * `payload` - Work payload as a dictionary (will be hashed to compute data shape)
/// * `tags` - Optional tags for categorization (e.g., ["etl", "batch"])
/// * `trace_id` - Optional trace ID for grouping related work units
/// * `parent_id` - Optional parent receipt ID for DAG composition
///
/// # Returns
///
/// A `Receipt` containing the verifiable proof of work, or an error if creation fails.
///
/// # Example
///
/// ```rust,ignore
/// use northroot_engine::api::record_work;
///
/// let receipt = record_work(
///     "normalize-prices",
///     serde_json::json!({"input_hash": "...", "output_hash": "..."}),
///     vec!["etl"],
///     Some("trace-123".to_string()),
///     None,
/// )?;
/// ```
pub fn record_work(
    workload_id: &str,
    payload: serde_json::Value,
    _tags: Vec<String>, // Reserved for future use
    trace_id: Option<String>,
    parent_id: Option<String>,
) -> Result<Receipt, ApiError> {
    // Generate RID
    // For v0.1, use a simple UUID generation approach
    // In production, prefer UUIDv7 for time-ordered IDs
    let rid = {
        use uuid::Uuid;
        // Generate random UUID v4
        let mut bytes = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        // Set version (4) and variant bits
        bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
        bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 10
        Uuid::from_bytes(bytes)
    };

    // Compute data shape hash from payload
    // For v0.1, we use a simple hash of the canonical JSON representation
    let payload_bytes = serde_json::to_string(&payload)
        .map_err(|e| ApiError::SerializationError(e.to_string()))?;
    let data_shape_hash = sha256_prefixed(payload_bytes.as_bytes());

    // Generate or use provided trace ID
    let trace_id = trace_id.unwrap_or_else(|| generate_trace_id(None));

    // Create method reference from workload_id
    // For v0.1, we use a simple method shape hash derived from workload_id
    let method_shape_hash = crate::commitments::sha256_prefixed(
        format!("workload:{}", workload_id).as_bytes(),
    );

    let method_ref = MethodRef {
        method_id: workload_id.to_string(),
        version: "0.1.0".to_string(),
        method_shape_root: method_shape_hash,
    };

    // Create a single span commitment from the payload hash
    // For v0.1 minimal API, we use the data shape hash as the span commitment
    let span_commitment = data_shape_hash.clone();

    // Compute execution roots
    let identity_root = sha256_prefixed(&[0x00u8]); // Default empty
    let roots = compute_execution_roots(&[span_commitment.clone()], identity_root);

    // Build execution payload
    // 
    // Core fields (always populated for verifiable compute):
    // - trace_id: For grouping related work
    // - method_ref: Workload identifier and method shape
    // - data_shape_hash: Input data commitment (also used as dom)
    // - span_commitments: Output commitments
    // - roots: Merkle roots for composition (trace_set_root used as cod)
    //
    // Optional fields (populated by policy/features when needed):
    // - pac: Proof-addressable cache key (for caching)
    // - change_epoch: Snapshot/version tracking
    // - output_mime_type, output_size_bytes: Metadata (when available)
    // - identity_ref, policy_ref: Set by policy when enabled
    // - input_locator_refs, output_locator_ref: Set when resolver is used
    // - All other fields: Reserved for future features
    let execution_payload = ExecutionPayload {
        trace_id: trace_id.clone(),
        method_ref,
        data_shape_hash: data_shape_hash.clone(),
        span_commitments: vec![span_commitment],
        roots: roots.clone(),
        // Optional fields - only populate when needed
        cdf_metadata: None,
        pac: None,
        change_epoch: None,
        minhash_signature: None,
        hll_cardinality: None,
        chunk_manifest_hash: None,
        chunk_manifest_size_bytes: None,
        merkle_root: None,
        prev_execution_rid: parent_id
            .as_ref()
            .and_then(|p| Uuid::parse_str(p).ok()),
        output_digest: None, // Removed: redundant with data_shape_hash (which is already in dom)
        manifest_root: None,
        output_mime_type: None,
        output_size_bytes: None,
        input_locator_refs: None,
        output_locator_ref: None,
    };

    // Build context
    let ctx = Context {
        policy_ref: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
        nonce: None,
        determinism: Some(DeterminismClass::Observational), // Default for v0.1
        identity_ref: None,
    };

    // Build receipt
    let receipt = Receipt {
        rid,
        version: "0.1.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: data_shape_hash.clone(),
        cod: roots.trace_set_root.clone(),
        links: parent_id
            .and_then(|p| Uuid::parse_str(&p).ok())
            .map(|uuid| vec![uuid])
            .unwrap_or_default(),
        ctx,
        payload: Payload::Execution(execution_payload),
        attest: None,
        sig: None,
        hash: String::new(), // Will be computed
    };

    // Compute hash
    let hash = receipt
        .compute_hash()
        .map_err(|e| ApiError::HashError(e.to_string()))?;

    Ok(Receipt { hash, ..receipt })
}

/// Verify receipt integrity and hash correctness.
///
/// This performs basic verification:
/// - Hash integrity: `receipt.hash == compute_hash(receipt)`
/// - Syntactic validation: receipt structure is well-formed
///
/// # Arguments
///
/// * `receipt` - Receipt to verify
///
/// # Returns
///
/// `Ok(true)` if receipt is valid, `Ok(false)` if invalid, or error if verification fails.
///
/// # Example
///
/// ```rust,ignore
/// use northroot_engine::api::verify_receipt;
///
/// let is_valid = verify_receipt(&receipt)?;
/// if is_valid {
///     println!("Receipt {} is valid", receipt.rid);
/// }
/// ```
pub fn verify_receipt(receipt: &Receipt) -> Result<bool, ApiError> {
    // Validate hash integrity
    let computed_hash = receipt
        .compute_hash()
        .map_err(|e| ApiError::HashError(e.to_string()))?;

    if computed_hash != receipt.hash {
        return Ok(false);
    }

    // Validate receipt structure
    receipt
        .validate_fast()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    Ok(true)
}

/// API error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// Serialization error
    SerializationError(String),
    /// Hash computation error
    HashError(String),
    /// Receipt validation error
    ValidationError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ApiError::HashError(msg) => write!(f, "Hash error: {}", msg),
            ApiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_work_basic() {
        let receipt = record_work(
            "test-workload",
            serde_json::json!({"input": "data"}),
            vec!["test".to_string()],
            None,
            None,
        )
        .unwrap();

        assert_eq!(receipt.kind, ReceiptKind::Execution);
        assert!(!receipt.hash.is_empty());
        assert!(receipt.hash.starts_with("sha256:"));
    }

    #[test]
    fn test_record_work_with_trace_and_parent() {
        let parent_receipt = record_work(
            "parent-workload",
            serde_json::json!({"input": "parent"}),
            vec![],
            Some("trace-123".to_string()),
            None,
        )
        .unwrap();

        let child_receipt = record_work(
            "child-workload",
            serde_json::json!({"input": "child"}),
            vec![],
            Some("trace-123".to_string()),
            Some(parent_receipt.rid.to_string()),
        )
        .unwrap();

        assert_eq!(child_receipt.links.len(), 1);
        assert_eq!(child_receipt.links[0], parent_receipt.rid);
    }

    #[test]
    fn test_verify_receipt_valid() {
        let receipt = record_work(
            "test-workload",
            serde_json::json!({"input": "data"}),
            vec![],
            None,
            None,
        )
        .unwrap();

        let is_valid = verify_receipt(&receipt).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_receipt_invalid_hash() {
        let mut receipt = record_work(
            "test-workload",
            serde_json::json!({"input": "data"}),
            vec![],
            None,
            None,
        )
        .unwrap();

        // Corrupt the hash
        receipt.hash = "sha256:invalid".to_string();

        let is_valid = verify_receipt(&receipt).unwrap();
        assert!(!is_valid);
    }
}

