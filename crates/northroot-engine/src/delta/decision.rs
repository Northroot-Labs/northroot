//! Reuse decision logic for delta compute.
//!
//! This module implements the **stabilized** reuse decision rule and cost model evaluation
//! for determining when to reuse previous computation results.
//!
//! ## Stabilized Reuse Criteria (v0.1)
//!
//! The reuse decision rule is **locked** for v0.1:
//!
//! ```text
//! Reuse iff J > C_id / (α · C_comp)
//! ```
//!
//! Where:
//! - **J**: Jaccard overlap [0,1] between prior and current chunk sets
//! - **C_id**: Identity/integration cost (cost to locate, validate, and splice reused results)
//! - **C_comp**: Baseline compute cost (cost to re-execute operator)
//! - **α**: Operator incrementality factor [0,1] (how efficiently deltas can be applied)
//!
//! ### Economic Delta
//!
//! The economic delta (savings estimate) is:
//!
//! ```text
//! ΔC ≈ α · C_comp · J - C_id
//! ```
//!
//! Positive values indicate savings from reuse. Reuse is rational when ΔC > 0.
//!
//! ### Decision Semantics
//!
//! - **Reuse**: Overlap exceeds threshold → reuse previous results
//! - **Recompute**: Overlap below threshold → recompute from scratch
//! - **Hybrid**: Reserved for future implementation (partial reuse)
//!
//! ### Implementation Status
//!
//! - ✅ Core decision logic (`decide_reuse`) - **STABILIZED**
//! - ✅ Economic delta calculation (`economic_delta`) - **STABILIZED**
//! - ✅ Cost model evaluation (`CostModel::reuse_threshold`) - **STABILIZED**
//! - ✅ Fast path overlap estimation (MinHash sketches) - **STABILIZED**
//! - ⚠️ Exact path manifest parsing - **KNOWN LIMITATION** (returns 0.0 overlap, fast path used instead)
//!
//! The exact path limitation is acceptable for v0.1 since the fast path provides sufficient
//! accuracy for most use cases. Full manifest parsing will be implemented in a future version.

use northroot_policy::CostModel;
use northroot_receipts::ReuseJustification;

/// Reuse decision result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReuseDecision {
    /// Reuse previous results
    Reuse,
    /// Recompute from scratch
    Recompute,
    /// Hybrid: reuse some, recompute some
    Hybrid,
}

/// Decide whether to reuse based on overlap and cost model.
///
/// Implements the reuse rule:
/// Reuse iff J > C_id / (α · C_comp)
///
/// Where:
/// - J: Jaccard overlap [0,1] between prior and current chunk sets
/// - C_id: Identity/integration cost
/// - C_comp: Baseline compute cost
/// - α: Operator incrementality factor [0,1]
///
/// # Arguments
///
/// * `overlap_j` - Jaccard overlap [0,1]. Values outside [0,1] are clamped to this range.
/// * `cost_model` - Cost model parameters
///
/// # Returns
///
/// Reuse decision and justification parameters
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::decide_reuse;
/// use northroot_policy::{CostModel, CostValue};
///
/// let cost_model = CostModel {
///     c_id: CostValue::Constant { value: 10.0 },
///     c_comp: CostValue::Constant { value: 100.0 },
///     alpha: CostValue::Constant { value: 0.9 },
/// };
/// let overlap_j = 0.15; // 15% overlap
///
/// let (decision, justification) = decide_reuse(overlap_j, &cost_model, None);
/// // threshold = 10.0 / (0.9 * 100.0) = 0.111
/// // Since 0.15 > 0.111, decision will be Reuse
/// ```
pub fn decide_reuse(
    overlap_j: f64,
    cost_model: &CostModel,
    row_count: Option<usize>,
) -> (ReuseDecision, ReuseJustification) {
    // Clamp overlap to valid range [0, 1]
    let overlap_j = overlap_j.clamp(0.0, 1.0);

    let threshold = cost_model.reuse_threshold(row_count);

    let decision = if overlap_j > threshold {
        ReuseDecision::Reuse
    } else if overlap_j > 0.0 {
        // Some overlap but below threshold - could be hybrid in future
        ReuseDecision::Recompute
    } else {
        ReuseDecision::Recompute
    };

    // Evaluate cost model to get concrete values
    let (c_id, c_comp, alpha) = cost_model.evaluate(row_count);

    let justification = ReuseJustification {
        overlap_j: Some(overlap_j),
        alpha: Some(alpha),
        c_id: Some(c_id),
        c_comp: Some(c_comp),
        decision: Some(match decision {
            ReuseDecision::Reuse => "reuse".to_string(),
            ReuseDecision::Recompute => "recompute".to_string(),
            ReuseDecision::Hybrid => "hybrid".to_string(),
        }),
        layer: None,          // Caller should set layer based on context
        minhash_sketch: None, // Caller should set minhash_sketch for FinOps use cases
    };

    (decision, justification)
}

/// Compute economic delta (savings estimate) from reuse decision.
///
/// Economic delta is defined as:
/// ΔC ≈ α · C_comp · J - C_id
///
/// Positive values indicate savings from reuse.
///
/// # Arguments
///
/// * `overlap_j` - Jaccard overlap [0,1]. Values outside [0,1] are clamped to this range.
/// * `cost_model` - Cost model parameters
///
/// # Returns
///
/// Economic delta (positive = savings, negative = cost)
pub fn economic_delta(overlap_j: f64, cost_model: &CostModel, row_count: Option<usize>) -> f64 {
    // Clamp overlap to valid range [0, 1]
    let overlap_j = overlap_j.clamp(0.0, 1.0);
    let (c_id, c_comp, alpha) = cost_model.evaluate(row_count);
    alpha * c_comp * overlap_j - c_id
}

/// Decide reuse with layer tracking.
///
/// Same as `decide_reuse` but also sets the layer field in justification.
///
/// # Arguments
///
/// * `overlap_j` - Jaccard overlap [0,1]
/// * `cost_model` - Cost model parameters
/// * `layer` - Semantic level of shape equivalence ("data"|"method"|"reasoning"|"execution")
///
/// # Returns
///
/// Reuse decision and justification with layer set
pub fn decide_reuse_with_layer(
    overlap_j: f64,
    cost_model: &CostModel,
    layer: &str,
    row_count: Option<usize>,
) -> (ReuseDecision, ReuseJustification) {
    let (decision, mut justification) = decide_reuse(overlap_j, cost_model, row_count);
    justification.layer = Some(layer.to_string());
    (decision, justification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_policy::CostValue;

    fn create_test_cost_model() -> CostModel {
        CostModel {
            c_id: CostValue::Constant { value: 10.0 },
            c_comp: CostValue::Constant { value: 100.0 },
            alpha: CostValue::Constant { value: 0.9 },
        }
    }

    #[test]
    fn test_cost_model_reuse_threshold() {
        let model = create_test_cost_model();
        // threshold = 10.0 / (0.9 * 100.0) = 10.0 / 90.0 ≈ 0.111
        let threshold = model.reuse_threshold(None);
        assert!((threshold - 10.0 / 90.0).abs() < 0.0001);
    }

    #[test]
    fn test_cost_model_reuse_threshold_zero_alpha() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 10.0 },
            c_comp: CostValue::Constant { value: 100.0 },
            alpha: CostValue::Constant { value: 0.0 },
        };
        assert_eq!(model.reuse_threshold(None), f64::INFINITY);
    }

    #[test]
    fn test_decide_reuse_above_threshold() {
        let model = create_test_cost_model();
        let overlap_j = 0.15; // Above threshold of ~0.111

        let (decision, justification) = decide_reuse(overlap_j, &model, None);
        assert_eq!(decision, ReuseDecision::Reuse);
        assert_eq!(justification.overlap_j, Some(overlap_j));
        assert_eq!(justification.decision, Some("reuse".to_string()));
    }

    #[test]
    fn test_decide_reuse_below_threshold() {
        let model = create_test_cost_model();
        let overlap_j = 0.05; // Below threshold of ~0.111

        let (decision, justification) = decide_reuse(overlap_j, &model, None);
        assert_eq!(decision, ReuseDecision::Recompute);
        assert_eq!(justification.decision, Some("recompute".to_string()));
    }

    #[test]
    fn test_decide_reuse_with_layer() {
        let model = create_test_cost_model();
        let overlap_j = 0.15;

        let (decision, justification) = decide_reuse_with_layer(overlap_j, &model, "data", None);
        assert_eq!(decision, ReuseDecision::Reuse);
        assert_eq!(justification.layer, Some("data".to_string()));
    }

    #[test]
    fn test_economic_delta() {
        let model = create_test_cost_model();
        let overlap_j = 0.15;

        // ΔC = 0.9 * 100.0 * 0.15 - 10.0 = 13.5 - 10.0 = 3.5
        let delta = economic_delta(overlap_j, &model, None);
        assert!((delta - 3.5).abs() < 0.0001);
    }

    #[test]
    fn test_economic_delta_negative() {
        let model = create_test_cost_model();
        let overlap_j = 0.05;

        // ΔC = 0.9 * 100.0 * 0.05 - 10.0 = 4.5 - 10.0 = -5.5
        let delta = economic_delta(overlap_j, &model, None);
        assert!((delta - (-5.5)).abs() < 0.0001);
    }
}
