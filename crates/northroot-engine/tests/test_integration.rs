//! Integration tests for end-to-end engine workflows.
//!
//! These tests verify that the engine components work together correctly
//! in realistic scenarios, including:
//! - Full receipt composition chains
//! - Delta compute workflows
//! - Error propagation

use northroot_engine::*;
use northroot_receipts::{
    Context, DataShapePayload, ExecutionPayload, MethodRef, Payload, Receipt, ReceiptKind,
};
use serde_json::json;
use uuid::Uuid;

fn create_test_context() -> Context {
    Context {
        policy_ref: Some("pol:test/policy@1.0.0".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    }
}

fn create_test_receipt(
    rid: Uuid,
    kind: ReceiptKind,
    dom: String,
    cod: String,
    payload: Payload,
) -> Receipt {
    let ctx = create_test_context();
    let receipt = Receipt {
        rid,
        version: "0.3.0".to_string(),
        kind,
        dom,
        cod,
        links: Vec::new(),
        ctx,
        payload,
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

#[test]
fn test_full_composition_workflow() {
    // Create a simple sequential chain: DataShape -> Execution
    let shape_hash =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let execution_hash =
        "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();

    let data_shape = create_test_receipt(
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        ReceiptKind::DataShape,
        shape_hash.clone(),
        execution_hash.clone(),
        Payload::DataShape(DataShapePayload {
            schema_hash: shape_hash.clone(),
            sketch_hash: None,
        }),
    );

    let method_ref = MethodRef {
        method_id: "com.test/method".to_string(),
        version: "1.0.0".to_string(),
        method_shape_root: shape_hash.clone(),
    };

    let execution = create_test_receipt(
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        ReceiptKind::Execution,
        execution_hash.clone(),
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        Payload::Execution(ExecutionPayload {
            trace_id: "trace:test".to_string(),
            method_ref: method_ref.clone(),
            data_shape_hash: shape_hash.clone(),
            span_commitments: vec![
                "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                    .to_string(),
            ],
            roots: compute_execution_roots(
                &[
                    "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                        .to_string(),
                ],
                "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                    .to_string(),
            ),
            cdf_metadata: None,
            pac: None,
            change_epoch: None,
            minhash_signature: None,
            hll_cardinality: None,
            chunk_manifest_hash: None,
            chunk_manifest_size_bytes: None,
            merkle_root: None,
            prev_execution_rid: None,
            output_digest: None,
            manifest_root: None,
            output_mime_type: None,
            output_size_bytes: None,
            input_locator_refs: None,
            output_locator_ref: None,
        }),
    );

    let chain = vec![data_shape.clone(), execution.clone()];

    // Validate sequential composition
    assert!(validate_sequential(&chain).is_ok());

    // Verify build_sequential_chain accepts it
    let built = build_sequential_chain(chain.clone()).unwrap();
    assert_eq!(built.len(), 2);

    // Verify all receipts are valid
    for receipt in &built {
        assert!(receipt.validate_fast().is_ok());
    }
}

// Strategy tests removed - strategies module deleted as dead weight

#[test]
fn test_error_propagation() {
    // Test that errors propagate correctly through composition chains

    // Test 1: Invalid sequential chain (mismatched cod/dom)
    let r1 = create_test_receipt(
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        ReceiptKind::DataShape,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
    );

    let r2 = create_test_receipt(
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        ReceiptKind::DataShape,
        "sha256:9999999999999999999999999999999999999999999999999999999999999999".to_string(), // Mismatch!
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:9999999999999999999999999999999999999999999999999999999999999999"
                .to_string(),
            sketch_hash: None,
        }),
    );

    let invalid_chain = vec![r1, r2];
    let result = validate_sequential(&invalid_chain);
    assert!(result.is_err());
    match result.unwrap_err() {
        CompositionError::SequentialMismatch { receipt_index, .. } => {
            assert_eq!(receipt_index, 1);
        }
        _ => panic!("Expected SequentialMismatch error"),
    }

    // Test 2: Circular dependency
    let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let r3 = create_test_receipt(
        rid,
        ReceiptKind::DataShape,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
    );

    let circular_chain = vec![r3.clone(), r3.clone()];
    let result = validate_sequential(&circular_chain);
    assert!(result.is_err());
    match result.unwrap_err() {
        CompositionError::InvalidChain(msg) => {
            assert!(msg.contains("Circular dependency"));
        }
        _ => panic!("Expected InvalidChain error for circular dependency"),
    }

    // Strategy error propagation tests removed - strategies module deleted
}

#[test]
fn test_parallel_composition() {
    // Test tensor (parallel) composition
    let hash1 =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let hash2 =
        "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let hash3 =
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    // Order-independent: same hashes in different order produce same root
    let root1 = compute_tensor_root(&[hash1.clone(), hash2.clone(), hash3.clone()]);
    let root2 = compute_tensor_root(&[hash3.clone(), hash1.clone(), hash2.clone()]);
    assert_eq!(root1, root2);

    // Different hashes produce different root
    let root3 = compute_tensor_root(&[hash1.clone(), hash2.clone()]);
    assert_ne!(root1, root3);
}

// Strategy registry tests removed - strategies module deleted as dead weight
