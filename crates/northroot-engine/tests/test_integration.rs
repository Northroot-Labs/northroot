//! Integration tests for end-to-end engine workflows.
//!
//! These tests verify that the engine components work together correctly
//! in realistic scenarios, including:
//! - Full receipt composition chains
//! - Strategy pipeline execution
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

#[test]
fn test_strategy_pipeline() {
    // Test partition -> incremental_sum pipeline
    let registry = default_registry();

    let partition_strategy = registry.get("partition").unwrap();
    let sum_strategy = registry.get("incremental_sum").unwrap();

    // Input data
    let input = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
        {"id": "3", "value": 30.0},
    ]);

    // Step 1: Partition
    let (partition_output, partition_state) = partition_strategy
        .execute(&input, ExecutionMode::Full, None)
        .unwrap();

    assert_eq!(partition_output["chunk_count"], 3);
    assert!(partition_state.len() == 3);

    // Step 2: Incremental sum
    let (sum_output, sum_state) = sum_strategy
        .execute(&input, ExecutionMode::Full, None)
        .unwrap();

    assert_eq!(sum_output["sum"], 60.0);
    assert_eq!(sum_state.len(), 3);
}

#[test]
fn test_delta_compute_workflow() {
    // Test full execution -> delta execution -> full execution
    let strategy = IncrementalSumStrategy::new();

    // Initial full execution
    let input1 = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
    ]);

    let (output1, state1) = strategy
        .execute(&input1, ExecutionMode::Full, None)
        .unwrap();

    assert_eq!(output1["sum"], 30.0);
    assert_eq!(state1.len(), 2);

    // Delta execution: add one row
    let input2 = json!([
        {"id": "3", "value": 30.0},
    ]);

    let (output2, state2) = strategy
        .execute(&input2, ExecutionMode::Delta, Some(&state1))
        .unwrap();

    // Delta mode outputs incremental sum (new row only) in the "sum" field
    assert_eq!(output2["sum"], 30.0); // Incremental sum (new row)
    assert_eq!(output2["incremental_sum"], 30.0);
    assert_eq!(state2.len(), 3);

    // Full execution again (should produce same result)
    let input3 = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
        {"id": "3", "value": 30.0},
    ]);

    let (output3, state3) = strategy
        .execute(&input3, ExecutionMode::Full, None)
        .unwrap();

    assert_eq!(output3["sum"], 60.0);
    assert_eq!(state3.len(), 3);
    // State hashes should match (same data)
    assert_eq!(state2.state_hash(), state3.state_hash());
}

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

    // Test 3: Strategy error propagation
    let strategy = IncrementalSumStrategy::new();

    // Invalid input (not an array)
    let invalid_input = json!({"not": "an array"});
    let result = strategy.execute(&invalid_input, ExecutionMode::Full, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        StrategyError::InvalidInput(msg) => {
            assert!(msg.contains("array"));
        }
        _ => panic!("Expected InvalidInput error"),
    }

    // Delta mode without previous state
    let valid_input = json!([{"id": "1", "value": 10.0}]);
    let result = strategy.execute(&valid_input, ExecutionMode::Delta, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        StrategyError::StateMismatch(msg) => {
            assert!(msg.contains("Previous state is required"));
        }
        _ => panic!("Expected StateMismatch error"),
    }
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

#[test]
fn test_strategy_registry_integration() {
    // Test that strategy registry works with actual strategies
    let mut registry = StrategyRegistry::new();

    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);

    registry.register(PartitionStrategy::new());
    registry.register(IncrementalSumStrategy::new());

    assert_eq!(registry.len(), 2);
    assert!(registry.contains("partition"));
    assert!(registry.contains("incremental_sum"));

    // Test default registry
    let default = default_registry();
    assert!(!default.is_empty());
    assert!(default.contains("partition"));
    assert!(default.contains("incremental_sum"));

    // Test strategy execution through registry
    let strategy = default.get("incremental_sum").unwrap();
    let input = json!([{"id": "1", "value": 42.0}]);
    let (output, _state) = strategy.execute(&input, ExecutionMode::Full, None).unwrap();
    assert_eq!(output["sum"], 42.0);
}
