//! Tests for delta compute operations.
//!
//! Note: Test vectors for delta compute scenarios (Jaccard similarity, reuse decisions,
//! economic delta) are in `vectors/engine/delta_compute_scenarios.json` and validated
//! by `test_engine_vector_integrity.rs`.

use northroot_engine::delta::*;
use northroot_policy::{CostModel, CostValue};
use std::collections::HashSet;

fn create_test_cost_model(c_id: f64, c_comp: f64, alpha: f64) -> CostModel {
    CostModel {
        c_id: CostValue::Constant { value: c_id },
        c_comp: CostValue::Constant { value: c_comp },
        alpha: CostValue::Constant { value: alpha },
    }
}

#[test]
fn test_jaccard_similarity_integration() {
    let set1: HashSet<String> = ["a".to_string(), "b".to_string(), "c".to_string()]
        .into_iter()
        .collect();
    let set2: HashSet<String> = ["b".to_string(), "c".to_string(), "d".to_string()]
        .into_iter()
        .collect();

    let j = jaccard_similarity(&set1, &set2);
    // Intersection: {b, c} = 2, Union: {a, b, c, d} = 4
    assert_eq!(j, 0.5);
}

#[test]
fn test_weighted_jaccard_integration() {
    let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
    let set2: HashSet<String> = ["b".to_string(), "c".to_string()].into_iter().collect();

    let weights = |chunk: &String| -> f64 {
        match chunk.as_str() {
            "a" => 10.0,
            "b" => 5.0,
            "c" => 15.0,
            _ => 1.0,
        }
    };

    let j = weighted_jaccard_similarity(&set1, &set2, weights);
    // Intersection weight: 5.0 (b), Union weight: 30.0 (a:10 + b:5 + c:15)
    assert!((j - 5.0 / 30.0).abs() < 0.0001);
}

#[test]
fn test_decide_reuse_integration() {
    let model = create_test_cost_model(10.0, 100.0, 0.9);
    let overlap_j = 0.15; // Above threshold

    let (decision, justification) = decide_reuse(overlap_j, &model, None);
    assert_eq!(decision, ReuseDecision::Reuse);
    assert_eq!(justification.overlap_j, Some(overlap_j));
    assert_eq!(justification.alpha, Some(0.9));
    assert_eq!(justification.c_id, Some(10.0));
    assert_eq!(justification.c_comp, Some(100.0));
}

#[test]
fn test_decide_reuse_below_threshold() {
    let model = create_test_cost_model(10.0, 100.0, 0.9);
    let overlap_j = 0.05; // Below threshold

    let (decision, _) = decide_reuse(overlap_j, &model, None);
    assert_eq!(decision, ReuseDecision::Recompute);
}

#[test]
fn test_decide_reuse_with_layer() {
    let model = create_test_cost_model(10.0, 100.0, 0.9);
    let overlap_j = 0.15;

    let (decision, justification) = decide_reuse_with_layer(overlap_j, &model, "data", None);
    assert_eq!(decision, ReuseDecision::Reuse);
    assert_eq!(justification.layer, Some("data".to_string()));
}

#[test]
fn test_economic_delta_positive() {
    let model = create_test_cost_model(10.0, 100.0, 0.9);
    let overlap_j = 0.15;

    let delta = economic_delta(overlap_j, &model, None);
    // ΔC = 0.9 * 100.0 * 0.15 - 10.0 = 13.5 - 10.0 = 3.5
    assert!((delta - 3.5).abs() < 0.0001);
}

#[test]
fn test_chunk_id_generation() {
    let content = b"test content";
    let id1 = chunk_id_from_bytes(content);
    let id2 = chunk_id_from_bytes(content);

    assert_eq!(id1, id2);
    assert!(id1.starts_with("sha256:"));
    assert_eq!(id1.len(), 71);
}

#[test]
fn test_chunk_set_operations() {
    let mut set1 = ChunkSet::new();
    set1.insert("chunk1".to_string());
    set1.insert("chunk2".to_string());
    set1.insert("chunk3".to_string());

    let mut set2 = ChunkSet::new();
    set2.insert("chunk2".to_string());
    set2.insert("chunk3".to_string());
    set2.insert("chunk4".to_string());

    let intersection = set1.intersection(&set2);
    assert_eq!(intersection.len(), 2);

    let union = set1.union(&set2);
    assert_eq!(union.len(), 4);

    let difference = set1.difference(&set2);
    assert_eq!(difference.len(), 1);
    assert!(difference.contains("chunk1"));
}

#[test]
fn test_verify_exact_set() {
    let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
    let set2: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
    let set3: HashSet<String> = ["a".to_string(), "c".to_string()].into_iter().collect();

    assert!(verify_exact_set(&set1, &set2));
    assert!(!verify_exact_set(&set1, &set3));
}

#[test]
fn test_cost_model_threshold_edge_cases() {
    // Test with very high c_id
    let model1 = create_test_cost_model(1000.0, 100.0, 0.9);
    assert!(model1.reuse_threshold(None) > 10.0);

    // Test with very low c_id
    let model2 = create_test_cost_model(0.1, 100.0, 0.9);
    assert!(model2.reuse_threshold(None) < 0.01);

    // Test with zero alpha
    let model3 = create_test_cost_model(10.0, 100.0, 0.0);
    assert_eq!(model3.reuse_threshold(None), f64::INFINITY);
}
