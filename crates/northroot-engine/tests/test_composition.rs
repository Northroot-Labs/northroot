//! Tests for composition operations.
//!
//! Note: Root computation baseline values are locked in `test_drift_detection.rs`.
//! If root computation algorithms change, update `BASELINE_ROOTS` in that file.

use northroot_engine::composition::*;
use northroot_receipts::generate_sequential_chain;
use uuid::Uuid;

#[test]
fn test_validate_sequential_with_test_utils() {
    // Use the test utilities from receipts crate
    let chain = generate_sequential_chain();
    assert!(validate_sequential(&chain).is_ok());
}

#[test]
fn test_compute_tensor_root_order_independent() {
    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let h3 = "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    // Order-independent: same hashes in different order produce same root
    let root1 = compute_tensor_root(&[h1.clone(), h2.clone(), h3.clone()]);
    let root2 = compute_tensor_root(&[h3.clone(), h1.clone(), h2.clone()]);
    let root3 = compute_tensor_root(&[h2.clone(), h3.clone(), h1.clone()]);

    assert_eq!(root1, root2);
    assert_eq!(root2, root3);

    // Different hashes produce different root
    let h4 = "sha256:4444444444444444444444444444444444444444444444444444444444444444".to_string();
    let root4 = compute_tensor_root(&[h1.clone(), h2.clone(), h4]);
    assert_ne!(root1, root4);
}

#[test]
fn test_compute_tensor_root_single_hash() {
    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let root = compute_tensor_root(&[h1.clone()]);
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71);
}

#[test]
fn test_compute_tensor_root_empty() {
    let root = compute_tensor_root(&[]);
    assert!(root.starts_with("sha256:"));
    assert_eq!(root.len(), 71);
}

#[test]
fn test_build_sequential_chain() {
    // Verify all 6 receipt kinds from proof algebra spec are present:
    // data_shape → method_shape → reasoning_shape → execution → spend → settlement
    let chain = generate_sequential_chain();
    let built = build_sequential_chain(chain.clone()).unwrap();
    assert_eq!(built.len(), 6); // generate_sequential_chain returns 6 receipts (all kinds)
    assert!(validate_sequential(&built).is_ok());
}

#[test]
fn test_build_sequential_chain_invalid() {
    use northroot_receipts::{Context, DataShapePayload, Payload, Receipt, ReceiptKind};

    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let r1 = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: Vec::new(),
        ctx: ctx.clone(),
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let r1_hash = r1.compute_hash().unwrap();
    let r1 = Receipt {
        hash: r1_hash,
        ..r1
    };

    let r2 = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:9999999999999999999999999999999999999999999999999999999999999999".to_string(), // Mismatch!
        cod: "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        links: Vec::new(),
        ctx,
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:9999999999999999999999999999999999999999999999999999999999999999"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let r2_hash = r2.compute_hash().unwrap();
    let r2 = Receipt {
        hash: r2_hash,
        ..r2
    };

    let result = build_sequential_chain(vec![r1, r2]);
    assert!(result.is_err());
    match result.unwrap_err() {
        CompositionError::SequentialMismatch { receipt_index, .. } => {
            assert_eq!(receipt_index, 1);
        }
        _ => panic!("Expected SequentialMismatch error"),
    }
}

#[test]
fn test_validate_all_links() {
    use northroot_receipts::{Context, DataShapePayload, Payload, Receipt, ReceiptKind};
    use std::collections::HashMap;

    let parent_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let child1_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let child2_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let parent = Receipt {
        rid: parent_rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: vec![child1_rid, child2_rid],
        ctx: ctx.clone(),
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let parent_hash = parent.compute_hash().unwrap();
    let parent = Receipt {
        hash: parent_hash,
        ..parent
    };

    let child1 = Receipt {
        rid: child1_rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        cod: "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        links: Vec::new(),
        ctx: ctx.clone(),
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let child1_hash = child1.compute_hash().unwrap();
    let child1 = Receipt {
        hash: child1_hash,
        ..child1
    };

    let child2 = Receipt {
        rid: child2_rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        cod: "sha256:4444444444444444444444444444444444444444444444444444444444444444".to_string(),
        links: Vec::new(),
        ctx,
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let child2_hash = child2.compute_hash().unwrap();
    let child2 = Receipt {
        hash: child2_hash,
        ..child2
    };

    let mut children = HashMap::new();
    children.insert(child1_rid, child1);
    children.insert(child2_rid, child2);

    assert!(validate_all_links(&parent, &children).is_ok());
}

#[test]
fn test_validate_all_links_missing_child() {
    use northroot_receipts::{Context, DataShapePayload, Payload, Receipt, ReceiptKind};
    use std::collections::HashMap;

    let parent_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let child1_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let child2_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let parent = Receipt {
        rid: parent_rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: vec![child1_rid, child2_rid],
        ctx: ctx.clone(),
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let parent_hash = parent.compute_hash().unwrap();
    let parent = Receipt {
        hash: parent_hash,
        ..parent
    };

    let child1 = Receipt {
        rid: child1_rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        cod: "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        links: Vec::new(),
        ctx,
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let child1_hash = child1.compute_hash().unwrap();
    let child1 = Receipt {
        hash: child1_hash,
        ..child1
    };

    let mut children = HashMap::new();
    children.insert(child1_rid, child1);
    // child2 is missing!

    let result = validate_all_links(&parent, &children);
    assert!(result.is_err());
    match result.unwrap_err() {
        CompositionError::LinkValidationFailed { child_rid, .. } => {
            assert_eq!(child_rid, child2_rid);
        }
        _ => panic!("Expected LinkValidationFailed error"),
    }
}
