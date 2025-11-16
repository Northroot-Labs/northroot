//! Test vector integrity: verify engine test vectors are valid and produce expected results.
//!
//! This test validates all engine test vectors to ensure they:
//! - Load correctly
//! - Produce expected root values
//! - Match computed values for delta compute scenarios
//! - Validate composition chains correctly

use ciborium::value::Value as CborValue;
use northroot_engine::*;
use northroot_receipts::{adapters::json, Receipt};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;

// Helper to convert JSON Value to CBOR Value
fn json_to_cbor_value(json: &Value) -> CborValue {
    match json {
        Value::Null => CborValue::Null,
        Value::Bool(b) => CborValue::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CborValue::Integer(i.into())
            } else if let Some(f) = n.as_f64() {
                CborValue::Float(f)
            } else {
                // Fallback: try to parse as string
                CborValue::Text(n.to_string())
            }
        }
        Value::String(s) => CborValue::Text(s.clone()),
        Value::Array(a) => CborValue::Array(a.iter().map(json_to_cbor_value).collect()),
        Value::Object(o) => {
            let mut map = Vec::new();
            for (k, v) in o {
                map.push((CborValue::Text(k.clone()), json_to_cbor_value(v)));
            }
            CborValue::Map(map)
        }
    }
}

fn load_vector(path: &str) -> Result<Vec<Receipt>, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    // Parse JSON array of receipts
    let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
    let mut receipts = Vec::new();
    if let serde_json::Value::Array(arr) = json_value {
        for item in arr {
            let json_str = serde_json::to_string(&item)?;
            // Test vectors now have CBOR-based hashes
            let receipt = json::receipt_from_json(&json_str)?;
            receipts.push(receipt);
        }
    }
    Ok(receipts)
}

fn load_json_vector(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&json_str)?;
    Ok(value)
}

#[test]
fn test_composition_chain_valid_integrity() {
    let chain = load_vector("../../vectors/engine/composition_chain_valid.json")
        .unwrap_or_else(|e| panic!("Failed to load valid chain: {}", e));

    // Verify chain has all 6 receipt kinds
    assert_eq!(chain.len(), 6, "Valid chain must have all 6 receipt kinds");

    // Verify sequential composition (cod(R_i) == dom(R_{i+1}))
    for i in 0..chain.len().saturating_sub(1) {
        assert_eq!(
            chain[i].cod,
            chain[i + 1].dom,
            "Sequential mismatch at index {}: cod({}) != dom({})",
            i,
            i,
            i + 1
        );
    }

    // Verify all receipts validate
    for (idx, receipt) in chain.iter().enumerate() {
        receipt
            .validate()
            .unwrap_or_else(|e| panic!("Receipt {} in valid chain failed validation: {}", idx, e));
    }

    // Verify composition validation passes
    assert!(
        validate_sequential(&chain).is_ok(),
        "Valid chain should pass validate_sequential"
    );
}

#[test]
fn test_composition_chain_invalid_integrity() {
    let chain = load_vector("../../vectors/engine/composition_chain_invalid.json")
        .unwrap_or_else(|e| panic!("Failed to load invalid chain: {}", e));

    // Verify invalid chain fails validation
    let result = validate_sequential(&chain);
    assert!(
        result.is_err(),
        "Invalid chain should fail validate_sequential"
    );

    // Verify it's a SequentialMismatch error
    match result.unwrap_err() {
        CompositionError::SequentialMismatch { .. } => {}
        _ => panic!("Expected SequentialMismatch error for invalid chain"),
    }
}

#[test]
fn test_tensor_composition_integrity() {
    let tensor_data = load_json_vector("../../vectors/engine/tensor_composition.json")
        .unwrap_or_else(|e| panic!("Failed to load tensor composition: {}", e));

    let child_hashes: Vec<String> = tensor_data["child_receipt_hashes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let expected_root = tensor_data["tensor_root"].as_str().unwrap();

    // Compute tensor root
    let computed_root = compute_tensor_root(&child_hashes);

    assert_eq!(
        computed_root, expected_root,
        "Computed tensor root should match expected"
    );

    // Verify order independence
    let mut reversed = child_hashes.clone();
    reversed.reverse();
    let reversed_root = compute_tensor_root(&reversed);
    assert_eq!(
        computed_root, reversed_root,
        "Tensor root should be order-independent"
    );
}

#[test]
fn test_delta_compute_scenarios_integrity() {
    let scenarios = load_json_vector("../../vectors/engine/delta_compute_scenarios.json")
        .unwrap_or_else(|e| panic!("Failed to load delta compute scenarios: {}", e));

    // Test Jaccard similarity
    let jaccard_data = &scenarios["jaccard_similarity"];
    let set1: HashSet<String> = jaccard_data["set1"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let set2: HashSet<String> = jaccard_data["set2"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let expected_j = jaccard_data["expected"].as_f64().unwrap();
    let computed_j = delta::jaccard_similarity(&set1, &set2);

    assert!(
        (computed_j - expected_j).abs() < 0.0001,
        "Jaccard similarity mismatch: expected {}, computed {}",
        expected_j,
        computed_j
    );

    // Test reuse decision
    let reuse_data = &scenarios["reuse_decision"];
    let overlap_j = reuse_data["overlap_j"].as_f64().unwrap();
    let cost_data = &reuse_data["cost_model"];
    use northroot_policy::{CostModel, CostValue};
    let cost_model = CostModel {
        c_id: CostValue::Constant {
            value: cost_data["c_id"].as_f64().unwrap(),
        },
        c_comp: CostValue::Constant {
            value: cost_data["c_comp"].as_f64().unwrap(),
        },
        alpha: CostValue::Constant {
            value: cost_data["alpha"].as_f64().unwrap(),
        },
    };
    let (decision, justification) = delta::decide_reuse(overlap_j, &cost_model, None);
    let expected_decision = reuse_data["decision"].as_str().unwrap();

    assert_eq!(
        format!("{:?}", decision),
        expected_decision,
        "Reuse decision mismatch"
    );
    assert_eq!(
        justification.overlap_j.unwrap(),
        overlap_j,
        "Justification overlap_j should match input"
    );

    // Test economic delta
    let delta_data = &scenarios["economic_delta"];
    let delta_overlap_j = delta_data["overlap_j"].as_f64().unwrap();
    let delta_cost_data = &delta_data["cost_model"];
    let delta_cost_model = CostModel {
        c_id: CostValue::Constant {
            value: delta_cost_data["c_id"].as_f64().unwrap(),
        },
        c_comp: CostValue::Constant {
            value: delta_cost_data["c_comp"].as_f64().unwrap(),
        },
        alpha: CostValue::Constant {
            value: delta_cost_data["alpha"].as_f64().unwrap(),
        },
    };
    let expected_delta = delta_data["expected"].as_f64().unwrap();
    let computed_delta = delta::economic_delta(delta_overlap_j, &delta_cost_model, None);

    assert!(
        (computed_delta - expected_delta).abs() < 0.0001,
        "Economic delta mismatch: expected {}, computed {}",
        expected_delta,
        computed_delta
    );
}

#[test]
fn test_execution_roots_integrity() {
    let roots_data = load_json_vector("../../vectors/engine/execution_roots.json")
        .unwrap_or_else(|e| panic!("Failed to load execution roots: {}", e));

    for scenario in roots_data["scenarios"].as_array().unwrap() {
        let span_commitments: Vec<String> = scenario["span_commitments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let identity_root = scenario["identity_root"].as_str().unwrap().to_string();
        let expected_trace_set = scenario["trace_set_root"].as_str().unwrap();
        let expected_trace_seq = scenario["trace_seq_root"].as_str().unwrap();

        let roots = compute_execution_roots(&span_commitments, identity_root);

        assert_eq!(
            roots.trace_set_root,
            expected_trace_set,
            "Trace set root mismatch for scenario: {}",
            scenario["name"].as_str().unwrap()
        );
        assert_eq!(
            roots.trace_seq_root.as_ref().unwrap(),
            expected_trace_seq,
            "Trace seq root mismatch for scenario: {}",
            scenario["name"].as_str().unwrap()
        );
    }
}

#[test]
fn test_merkle_row_map_integrity() {
    // MerkleRowMap tests removed - rowmap module deleted as dead weight
    /*
    let merkle_data = load_json_vector("../../vectors/engine/merkle_row_map_examples.json")
        .unwrap_or_else(|e| panic!("Failed to load Merkle Row-Map examples: {}", e));

    // Test empty map
    let empty_map = MerkleRowMap::new();
    let empty_expected = merkle_data["empty_map"]["root"].as_str().unwrap();
    assert_eq!(
        empty_map.compute_root(),
        empty_expected,
        "Empty map root mismatch"
    );

    // Test single entry
    let mut single_map = MerkleRowMap::new();
    let single_entries = merkle_data["single_entry"]["entries"].as_object().unwrap();
    for (key, value) in single_entries {
        single_map.insert(key.clone(), json_to_cbor_value(value));
    }
    let single_expected = merkle_data["single_entry"]["root"].as_str().unwrap();
    assert_eq!(
        single_map.compute_root(),
        single_expected,
        "Single entry map root mismatch"
    );

    // Test multiple entries
    let mut multi_map = MerkleRowMap::new();
    let multi_entries = merkle_data["multiple_entries"]["entries"]
        .as_object()
        .unwrap();
    for (key, value) in multi_entries {
        multi_map.insert(key.clone(), json_to_cbor_value(value));
    }
    let multi_expected = merkle_data["multiple_entries"]["root"].as_str().unwrap();
    assert_eq!(
        multi_map.compute_root(),
        multi_expected,
        "Multiple entries map root mismatch"
    );

    // Test order independence
    let order_data = &merkle_data["order_independence"];
    let map1_entries = order_data["map1"]["entries"].as_object().unwrap();
    let map2_entries = order_data["map2"]["entries"].as_object().unwrap();

    let mut map1 = MerkleRowMap::new();
    for (key, value) in map1_entries {
        map1.insert(key.clone(), json_to_cbor_value(value));
    }

    let mut map2 = MerkleRowMap::new();
    for (key, value) in map2_entries {
        map2.insert(key.clone(), json_to_cbor_value(value));
    }

    assert_eq!(
        map1.compute_root(),
        map2.compute_root(),
        "Order independence test failed: roots should match regardless of insertion order"
    );
    */
}
