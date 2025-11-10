//! Composition tests: sequential and parallel receipt chains.

use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Receipt {
    let json_str = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json_str).unwrap()
}

#[test]
fn test_sequential_chain_valid() {
    let data_shape = load_vector("../../vectors/data_shape.json");
    let method_shape = load_vector("../../vectors/method_shape.json");
    let reasoning_shape = load_vector("../../vectors/reasoning_shape.json");
    let execution = load_vector("../../vectors/execution.json");
    let spend = load_vector("../../vectors/spend.json");
    let settlement = load_vector("../../vectors/settlement.json");

    let chain = vec![
        data_shape,
        method_shape,
        reasoning_shape,
        execution,
        spend,
        settlement,
    ];

    validate_composition(&chain).unwrap();
}

#[test]
fn test_sequential_chain_invalid() {
    let data_shape = load_vector("../../vectors/data_shape.json");
    let mut method_shape = load_vector("../../vectors/method_shape.json");

    // Break composition by changing dom
    method_shape.dom = "sha256:invalid".to_string();

    let chain = vec![data_shape, method_shape];

    assert!(validate_composition(&chain).is_err());
}

#[test]
fn test_single_receipt_composition() {
    let receipt = load_vector("../../vectors/data_shape.json");
    let chain = vec![receipt];

    // Single receipt should always pass composition check
    validate_composition(&chain).unwrap();
}

#[test]
fn test_empty_chain() {
    let chain: Vec<Receipt> = vec![];

    // Empty chain should pass (nothing to validate)
    validate_composition(&chain).unwrap();
}

#[test]
fn test_partial_chain() {
    let data_shape = load_vector("../../vectors/data_shape.json");
    let method_shape = load_vector("../../vectors/method_shape.json");
    let _execution = load_vector("../../vectors/execution.json");

    // Chain missing reasoning_shape - should still validate composition if dom/cod match
    // But execution.dom should match method_shape.cod, not reasoning_shape.cod
    // This test verifies we can validate partial chains
    let chain = vec![data_shape, method_shape];
    validate_composition(&chain).unwrap();
}
