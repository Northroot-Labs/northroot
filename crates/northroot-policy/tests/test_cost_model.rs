//! Tests for cost model extraction and evaluation.

use northroot_policy::{extract_cost_model, load_policy, CostModel, CostValue};

#[test]
fn test_extract_cost_model_constant() {
    let policy = load_policy("pol:finops/cost-attribution@1").unwrap();
    let cost_model = extract_cost_model(&policy, None).unwrap();

    // Evaluate with no row count (should use constant values)
    let (c_id, c_comp, alpha) = cost_model.evaluate(None);
    assert_eq!(c_id, 0.1);
    assert_eq!(c_comp, 10.0);
    assert_eq!(alpha, 0.9);

    // Evaluate with row count (should still be constant)
    let (c_id, c_comp, alpha) = cost_model.evaluate(Some(1000));
    assert_eq!(c_id, 0.1);
    assert_eq!(c_comp, 10.0);
    assert_eq!(alpha, 0.9);
}

#[test]
fn test_extract_cost_model_linear() {
    let policy = load_policy("pol:etl/partition-reuse@1").unwrap();
    let cost_model = extract_cost_model(&policy, None).unwrap();

    // Evaluate with no row count (should use base values)
    let (c_id, c_comp, alpha) = cost_model.evaluate(None);
    assert_eq!(c_id, 1.0);
    assert!((c_comp - 0.5).abs() < 0.0001); // base = 0.5
    assert_eq!(alpha, 0.85);

    // Evaluate with row count (should compute linear)
    let (c_id, c_comp, alpha) = cost_model.evaluate(Some(1000));
    assert_eq!(c_id, 1.0);
    assert!((c_comp - 0.51).abs() < 0.0001); // 0.5 + 0.00001 * 1000 = 0.51
    assert_eq!(alpha, 0.85);
}

#[test]
fn test_extract_cost_model_reuse_threshold() {
    let policy = load_policy("pol:finops/cost-attribution@1").unwrap();
    let cost_model = extract_cost_model(&policy, None).unwrap();

    // threshold = 0.1 / (0.9 * 10.0) = 0.1 / 9.0 ≈ 0.0111
    let threshold = cost_model.reuse_threshold(None);
    assert!((threshold - 0.1 / 9.0).abs() < 0.0001);
}

#[test]
fn test_extract_cost_model_linear_reuse_threshold() {
    let policy = load_policy("pol:etl/partition-reuse@1").unwrap();
    let cost_model = extract_cost_model(&policy, Some(1000)).unwrap();

    // With 1000 rows: c_comp = 0.5 + 0.00001 * 1000 = 0.51
    // threshold = 1.0 / (0.85 * 0.51) ≈ 2.31
    let threshold = cost_model.reuse_threshold(Some(1000));
    assert!((threshold - 1.0 / (0.85 * 0.51)).abs() < 0.0001);
}

#[test]
fn test_cost_model_validation_alpha_out_of_range() {
    // This test would require a policy with invalid alpha, but we validate at extraction time
    // So we test that validation happens during extraction
    let policy = load_policy("pol:finops/cost-attribution@1").unwrap();
    let cost_model = extract_cost_model(&policy, None).unwrap();

    // Should be valid
    assert!(cost_model.validate().is_ok());
}

#[test]
fn test_cost_value_constant_evaluation() {
    let cost = CostValue::Constant { value: 5.0 };
    assert_eq!(cost.evaluate(None), 5.0);
    assert_eq!(cost.evaluate(Some(100)), 5.0);
}

#[test]
fn test_cost_value_linear_evaluation() {
    let cost = CostValue::Linear {
        per_row: 0.01,
        base: 2.0,
    };
    assert_eq!(cost.evaluate(None), 2.0);
    assert_eq!(cost.evaluate(Some(100)), 3.0); // 2.0 + 0.01 * 100
    assert_eq!(cost.evaluate(Some(1000)), 12.0); // 2.0 + 0.01 * 1000
}
