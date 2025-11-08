//! Reuse decision logic for delta compute.
//!
//! This module implements the reuse decision rule and cost model evaluation
//! for determining when to reuse previous computation results.

use northroot_receipts::ReuseJustification;
use std::collections::HashSet;

/// Cost model for reuse decisions.
///
/// This struct captures the economic parameters needed to evaluate
/// whether reuse is beneficial.
#[derive(Debug, Clone, PartialEq)]
pub struct CostModel {
    /// Identity/integration cost: cost to locate, validate, and splice reused results
    pub c_id: f64,
    /// Baseline compute cost: cost to (re)execute operator
    pub c_comp: f64,
    /// Operator incrementality factor [0,1]: how efficiently deltas can be applied
    pub alpha: f64,
}

impl CostModel {
    /// Create a new cost model.
    ///
    /// # Arguments
    ///
    /// * `c_id` - Identity/integration cost
    /// * `c_comp` - Baseline compute cost
    /// * `alpha` - Incrementality factor [0,1]
    ///
    /// # Panics
    ///
    /// Panics if `alpha` is not in [0, 1] or if costs are negative.
    pub fn new(c_id: f64, c_comp: f64, alpha: f64) -> Self {
        assert!(c_id >= 0.0, "c_id must be non-negative");
        assert!(c_comp >= 0.0, "c_comp must be non-negative");
        assert!((0.0..=1.0).contains(&alpha), "alpha must be in [0, 1]");

        Self { c_id, c_comp, alpha }
    }

    /// Compute the reuse threshold.
    ///
    /// Returns the minimum Jaccard overlap required for reuse to be beneficial:
    /// threshold = C_id / (α · C_comp)
    ///
    /// # Returns
    ///
    /// Reuse threshold in [0, ∞). If denominator is zero, returns infinity.
    pub fn reuse_threshold(&self) -> f64 {
        let denominator = self.alpha * self.c_comp;
        if denominator == 0.0 {
            f64::INFINITY
        } else {
            self.c_id / denominator
        }
    }
}

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
/// * `overlap_j` - Jaccard overlap [0,1]
/// * `cost_model` - Cost model parameters
///
/// # Returns
///
/// Reuse decision and justification parameters
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::{decide_reuse, CostModel};
///
/// let cost_model = CostModel::new(10.0, 100.0, 0.9);
/// let overlap_j = 0.15; // 15% overlap
///
/// let (decision, justification) = decide_reuse(overlap_j, &cost_model);
/// // threshold = 10.0 / (0.9 * 100.0) = 0.111
/// // Since 0.15 > 0.111, decision will be Reuse
/// ```
pub fn decide_reuse(overlap_j: f64, cost_model: &CostModel) -> (ReuseDecision, ReuseJustification) {
    let threshold = cost_model.reuse_threshold();

    let decision = if overlap_j > threshold {
        ReuseDecision::Reuse
    } else if overlap_j > 0.0 {
        // Some overlap but below threshold - could be hybrid in future
        ReuseDecision::Recompute
    } else {
        ReuseDecision::Recompute
    };

    let justification = ReuseJustification {
        overlap_j: Some(overlap_j),
        alpha: Some(cost_model.alpha),
        c_id: Some(cost_model.c_id),
        c_comp: Some(cost_model.c_comp),
        decision: Some(match decision {
            ReuseDecision::Reuse => "reuse".to_string(),
            ReuseDecision::Recompute => "recompute".to_string(),
            ReuseDecision::Hybrid => "hybrid".to_string(),
        }),
        layer: None, // Caller should set layer based on context
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
/// * `overlap_j` - Jaccard overlap [0,1]
/// * `cost_model` - Cost model parameters
///
/// # Returns
///
/// Economic delta (positive = savings, negative = cost)
pub fn economic_delta(overlap_j: f64, cost_model: &CostModel) -> f64 {
    cost_model.alpha * cost_model.c_comp * overlap_j - cost_model.c_id
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
) -> (ReuseDecision, ReuseJustification) {
    let (decision, mut justification) = decide_reuse(overlap_j, cost_model);
    justification.layer = Some(layer.to_string());
    (decision, justification)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_model_reuse_threshold() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        // threshold = 10.0 / (0.9 * 100.0) = 10.0 / 90.0 ≈ 0.111
        let threshold = model.reuse_threshold();
        assert!((threshold - 10.0 / 90.0).abs() < 0.0001);
    }

    #[test]
    fn test_cost_model_reuse_threshold_zero_alpha() {
        let model = CostModel::new(10.0, 100.0, 0.0);
        assert_eq!(model.reuse_threshold(), f64::INFINITY);
    }

    #[test]
    fn test_decide_reuse_above_threshold() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        let overlap_j = 0.15; // Above threshold of ~0.111

        let (decision, justification) = decide_reuse(overlap_j, &model);
        assert_eq!(decision, ReuseDecision::Reuse);
        assert_eq!(justification.overlap_j, Some(overlap_j));
        assert_eq!(justification.decision, Some("reuse".to_string()));
    }

    #[test]
    fn test_decide_reuse_below_threshold() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        let overlap_j = 0.05; // Below threshold of ~0.111

        let (decision, justification) = decide_reuse(overlap_j, &model);
        assert_eq!(decision, ReuseDecision::Recompute);
        assert_eq!(justification.decision, Some("recompute".to_string()));
    }

    #[test]
    fn test_decide_reuse_with_layer() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        let overlap_j = 0.15;

        let (decision, justification) = decide_reuse_with_layer(overlap_j, &model, "data");
        assert_eq!(decision, ReuseDecision::Reuse);
        assert_eq!(justification.layer, Some("data".to_string()));
    }

    #[test]
    fn test_economic_delta() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        let overlap_j = 0.15;

        // ΔC = 0.9 * 100.0 * 0.15 - 10.0 = 13.5 - 10.0 = 3.5
        let delta = economic_delta(overlap_j, &model);
        assert!((delta - 3.5).abs() < 0.0001);
    }

    #[test]
    fn test_economic_delta_negative() {
        let model = CostModel::new(10.0, 100.0, 0.9);
        let overlap_j = 0.05;

        // ΔC = 0.9 * 100.0 * 0.05 - 10.0 = 4.5 - 10.0 = -5.5
        let delta = economic_delta(overlap_j, &model);
        assert!((delta - (-5.5)).abs() < 0.0001);
    }
}

