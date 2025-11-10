//! Tests for execution tracking and state management.
//!
//! Note: Root computation baseline values for MerkleRowMap and compute_execution_roots
//! are locked in `test_drift_detection.rs`. If root computation algorithms change,
//! update `BASELINE_ROOTS` in that file.

use northroot_engine::execution::*;
use northroot_receipts::{Context, MethodRef};
use serde_json::json;

#[test]
fn test_validate_method_ref() {
    let method_ref = MethodRef {
        method_id: "com.acme/test".to_string(),
        version: "1.0.0".to_string(),
        method_shape_root:
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    };

    assert!(validate_method_ref(&method_ref).is_ok());
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
    assert!(roots.trace_seq_root.unwrap().starts_with("sha256:"));
}

#[test]
fn test_merkle_row_map() {
    let mut map = MerkleRowMap::new();
    map.insert("key1".to_string(), json!(42));
    map.insert("key2".to_string(), json!("value"));

    let root1 = map.compute_root();
    assert!(root1.starts_with("sha256:"));

    // Same entries should produce same root
    let mut map2 = MerkleRowMap::new();
    map2.insert("key1".to_string(), json!(42));
    map2.insert("key2".to_string(), json!("value"));
    let root2 = map2.compute_root();

    assert_eq!(root1, root2);
}

#[test]
fn test_merkle_row_map_deterministic() {
    let mut map1 = MerkleRowMap::new();
    map1.insert("a".to_string(), json!(1));
    map1.insert("b".to_string(), json!(2));

    let mut map2 = MerkleRowMap::new();
    map2.insert("b".to_string(), json!(2));
    map2.insert("a".to_string(), json!(1));

    // BTreeMap maintains sorted order, so roots should be same
    assert_eq!(map1.compute_root(), map2.compute_root());
}

#[test]
fn test_execution_receipt_builder() {
    use northroot_engine::ExecutionReceiptBuilder;
    use northroot_receipts::ReceiptKind;
    use uuid::Uuid;

    let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let method_ref = MethodRef {
        method_id: "com.acme/test".to_string(),
        version: "1.0.0".to_string(),
        method_shape_root:
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    };

    let receipt = ExecutionReceiptBuilder::new()
        .trace_id("test-trace".to_string())
        .method_ref(method_ref)
        .data_shape_hash(
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .add_span_commitment(
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .build(rid, "0.3.0".to_string(), ctx)
        .unwrap();

    assert_eq!(receipt.kind, ReceiptKind::Execution);
    assert!(receipt.validate_fast().is_ok());
}

#[test]
fn test_generate_trace_id() {
    let trace_id1 = generate_trace_id(Some("test-seed"));
    let trace_id2 = generate_trace_id(Some("test-seed"));
    let trace_id3 = generate_trace_id(Some("different-seed"));

    assert_eq!(trace_id1, trace_id2); // Deterministic
    assert_ne!(trace_id1, trace_id3); // Different seeds produce different IDs
}
