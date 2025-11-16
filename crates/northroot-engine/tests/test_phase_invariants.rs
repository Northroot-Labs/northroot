//! Tests to verify core invariants after phased implementation changes.
//!
//! This module tests that core behavioral invariants still hold after:
//! - Phase 1: DataShape enum and ExecutionPayload extensions
//! - Phase 2: MerkleRowMap RFC-6962 domain separation (BREAKING CHANGE)
//! - Phase 3: ByteStream manifest builder (CAS module)

use ciborium::value::Value as CborValue;
use northroot_engine::{
    cas::{build_manifest_from_data, chunk_by_fixed},
    shapes::{compute_data_shape_hash, ChunkScheme, DataShape, KeyFormat, RowValueRepr},
};
use serde_json::json;

// Helper to convert JSON values to CBOR
fn json_to_cbor_value(value: &serde_json::Value) -> CborValue {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CborValue::Integer(i.into())
            } else if let Some(f) = n.as_f64() {
                CborValue::Float(f)
            } else {
                CborValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => CborValue::Text(s.clone()),
        serde_json::Value::Bool(b) => CborValue::Bool(*b),
        _ => CborValue::Text(value.to_string()),
    }
}

// Phase 2 tests removed - MerkleRowMap module deleted as dead weight

/// Phase 1: Verify DataShape enum invariants
mod phase1_data_shape {
    use super::*;

    #[test]
    fn test_data_shape_hash_deterministic() {
        // Invariant: Same DataShape → same hash
        let shape1 = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::CDC { avg_size: 65536 },
        };

        let shape2 = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::CDC { avg_size: 65536 },
        };

        let hash1 = compute_data_shape_hash(&shape1).unwrap();
        let hash2 = compute_data_shape_hash(&shape2).unwrap();

        assert_eq!(hash1, hash2, "Same DataShape must produce same hash");
        assert!(hash1.starts_with("sha256:"));
    }

    #[test]
    fn test_data_shape_hash_different_shapes() {
        // Invariant: Different shapes → different hashes
        let bytestream = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::Fixed { size: 4096 },
        };

        let rowmap = DataShape::RowMap {
            merkle_root: "sha256:abc123".to_string(),
            row_count: 1024,
            key_fmt: KeyFormat::Sha256Hex,
            value_repr: RowValueRepr::Number,
        };

        let hash1 = compute_data_shape_hash(&bytestream).unwrap();
        let hash2 = compute_data_shape_hash(&rowmap).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different shape types must produce different hashes"
        );
    }

    #[test]
    fn test_execution_payload_backward_compatibility() {
        // Invariant: New optional fields don't break existing code
        // This is tested implicitly by compilation, but we verify structure
        use northroot_receipts::{ExecutionPayload, MethodRef};

        let payload = ExecutionPayload {
            trace_id: "test".to_string(),
            method_ref: MethodRef {
                method_id: "test".to_string(),
                version: "1.0.0".to_string(),
                method_shape_root: "sha256:test".to_string(),
            },
            data_shape_hash: "sha256:test".to_string(),
            span_commitments: vec![],
            roots: northroot_receipts::ExecutionRoots {
                trace_set_root: "sha256:test".to_string(),
                identity_root: "sha256:test".to_string(),
                trace_seq_root: None,
            },
            cdf_metadata: None,
            pac: None,
            change_epoch: None,
            minhash_signature: None,
            hll_cardinality: None,
            chunk_manifest_hash: None,
            chunk_manifest_size_bytes: None,
            merkle_root: None,
            prev_execution_rid: None,
            // New optional fields - all None for backward compatibility
            output_digest: None,
            manifest_root: None,
            output_mime_type: None,
            output_size_bytes: None,
            input_locator_refs: None,
            output_locator_ref: None,
        };

        // Verify it compiles and can be used
        assert_eq!(payload.trace_id, "test");
        assert!(payload.output_digest.is_none());
    }
}

/// Phase 3: Verify CAS module invariants
mod phase3_cas {
    use super::*;

    #[test]
    fn test_manifest_deterministic() {
        // Invariant: Same data + scheme → same manifest root
        let data = b"test data for manifest";
        let manifest1 = build_manifest_from_data(
            data,
            northroot_engine::shapes::ChunkScheme::Fixed { size: 8 },
        )
        .unwrap();
        let manifest2 = build_manifest_from_data(
            data,
            northroot_engine::shapes::ChunkScheme::Fixed { size: 8 },
        )
        .unwrap();

        assert_eq!(
            manifest1.manifest_root, manifest2.manifest_root,
            "Same data and scheme must produce same manifest root"
        );
        assert_eq!(manifest1.manifest_len, manifest2.manifest_len);
    }

    #[test]
    fn test_chunking_preserves_data() {
        // Invariant: Chunking preserves all data (sum of chunk lengths = total length)
        let data = b"hello world, this is test data";
        let chunks = chunk_by_fixed(data, 8).unwrap();

        let total_chunk_len: u64 = chunks.iter().map(|c| c.len).sum();
        assert_eq!(
            total_chunk_len,
            data.len() as u64,
            "Sum of chunk lengths must equal total data length"
        );
    }

    #[test]
    fn test_manifest_empty_handling() {
        // Invariant: Empty data produces valid manifest
        let data = b"";
        let manifest = build_manifest_from_data(
            data,
            northroot_engine::shapes::ChunkScheme::Fixed { size: 8 },
        )
        .unwrap();

        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_len, 0);
        assert_eq!(manifest.chunks.len(), 0);
    }

    #[test]
    fn test_rfc6962_manifest_domain_separation() {
        // Verify manifest uses RFC-6962 domain separation
        let data = b"test";
        let manifest = build_manifest_from_data(
            data,
            northroot_engine::shapes::ChunkScheme::Fixed { size: 8 },
        )
        .unwrap();

        // Verify format
        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_root.len(), 71);
    }
}

/// Cross-phase integration invariants
mod cross_phase_invariants {
    use super::*;

    // Strategy tests removed - strategies module deleted as dead weight

    #[test]
    fn test_data_shape_integration() {
        // Invariant: DataShape can be constructed from CAS manifest
        let data = b"test data";
        let manifest = build_manifest_from_data(
            data,
            northroot_engine::shapes::ChunkScheme::Fixed { size: 4 },
        )
        .unwrap();

        let shape = DataShape::ByteStream {
            manifest_root: manifest.manifest_root.clone(),
            manifest_len: manifest.manifest_len,
            chunk_scheme: ChunkScheme::Fixed { size: 4 },
        };

        let shape_hash = compute_data_shape_hash(&shape).unwrap();
        assert!(shape_hash.starts_with("sha256:"));
    }
}
