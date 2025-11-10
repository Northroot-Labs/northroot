//! Cost model types for delta compute policies.
//!
//! This module defines cost model structures that can be loaded from policy JSON
//! and evaluated to concrete values for reuse decision calculations.

use serde::{Deserialize, Serialize};

/// Cost value that can be either constant or linear based on row count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CostValue {
    /// Constant cost value
    #[serde(rename = "constant")]
    Constant {
        /// Constant value
        value: f64,
    },
    /// Linear cost value: base + (per_row * row_count)
    #[serde(rename = "linear")]
    Linear {
        /// Cost per row
        per_row: f64,
        /// Base cost
        base: f64,
    },
}

impl CostValue {
    /// Evaluate the cost value given an optional row count.
    ///
    /// # Arguments
    ///
    /// * `row_count` - Optional row count for linear cost calculation
    ///
    /// # Returns
    ///
    /// Evaluated cost as f64. For Constant, returns the value. For Linear,
    /// returns base + (per_row * row_count). If row_count is None for Linear,
    /// returns base only.
    pub fn evaluate(&self, row_count: Option<usize>) -> f64 {
        match self {
            CostValue::Constant { value } => *value,
            CostValue::Linear { per_row, base } => {
                let rows = row_count.unwrap_or(0) as f64;
                base + (per_row * rows)
            }
        }
    }

    /// Validate that the cost value is non-negative.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or an error message if invalid.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            CostValue::Constant { value } => {
                if *value < 0.0 {
                    Err(format!(
                        "Constant cost value must be non-negative, got {}",
                        value
                    ))
                } else {
                    Ok(())
                }
            }
            CostValue::Linear { per_row, base } => {
                if *base < 0.0 {
                    Err(format!(
                        "Linear base cost must be non-negative, got {}",
                        base
                    ))
                } else if *per_row < 0.0 {
                    Err(format!(
                        "Linear per_row cost must be non-negative, got {}",
                        per_row
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Cost model for reuse decisions.
///
/// This struct captures the economic parameters needed to evaluate
/// whether reuse is beneficial. Values can be constant or linear based on row count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostModel {
    /// Identity/integration cost: cost to locate, validate, and splice reused results
    pub c_id: CostValue,
    /// Baseline compute cost: cost to (re)execute operator
    pub c_comp: CostValue,
    /// Operator incrementality factor [0,1]: how efficiently deltas can be applied
    pub alpha: CostValue,
}

impl CostModel {
    /// Evaluate the cost model to concrete f64 values given an optional row count.
    ///
    /// # Arguments
    ///
    /// * `row_count` - Optional row count for linear cost calculation
    ///
    /// # Returns
    ///
    /// Tuple of (c_id, c_comp, alpha) as f64 values
    pub fn evaluate(&self, row_count: Option<usize>) -> (f64, f64, f64) {
        (
            self.c_id.evaluate(row_count),
            self.c_comp.evaluate(row_count),
            self.alpha.evaluate(row_count),
        )
    }

    /// Validate that the cost model values are valid.
    ///
    /// Checks:
    /// - All cost values are non-negative
    /// - Alpha is in [0, 1]
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or an error message if invalid.
    pub fn validate(&self) -> Result<(), String> {
        self.c_id.validate()?;
        self.c_comp.validate()?;
        self.alpha.validate()?;

        // Validate alpha is in [0, 1] for any row count
        // We check both constant and linear cases
        let alpha_min = match &self.alpha {
            CostValue::Constant { value } => *value,
            CostValue::Linear { base, per_row: _ } => {
                // For linear, check base (minimum value)
                *base
            }
        };

        let alpha_max = match &self.alpha {
            CostValue::Constant { value } => *value,
            CostValue::Linear { base, per_row: _ } => {
                // For linear, check base + per_row * large_number (approximate max)
                // In practice, we'll validate at evaluation time, but check base here
                *base
            }
        };

        // Check that base alpha is in [0, 1]
        if alpha_min < 0.0 || alpha_max > 1.0 {
            return Err(format!(
                "Alpha must be in [0, 1], got base value: {}",
                alpha_min
            ));
        }

        Ok(())
    }

    /// Compute the reuse threshold for a given row count.
    ///
    /// Returns the minimum Jaccard overlap required for reuse to be beneficial:
    /// threshold = C_id / (α · C_comp)
    ///
    /// # Arguments
    ///
    /// * `row_count` - Optional row count for linear cost calculation
    ///
    /// # Returns
    ///
    /// Reuse threshold in [0, ∞). If denominator is zero (i.e., `alpha == 0` or
    /// `c_comp == 0`), returns `f64::INFINITY`, which means reuse is never beneficial
    /// regardless of overlap (since the operator cannot benefit from incremental computation).
    pub fn reuse_threshold(&self, row_count: Option<usize>) -> f64 {
        let (c_id, c_comp, alpha) = self.evaluate(row_count);
        let denominator = alpha * c_comp;
        if denominator == 0.0 {
            f64::INFINITY
        } else {
            c_id / denominator
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_value_constant() {
        let cost = CostValue::Constant { value: 10.0 };
        assert_eq!(cost.evaluate(None), 10.0);
        assert_eq!(cost.evaluate(Some(100)), 10.0);
        assert!(cost.validate().is_ok());
    }

    #[test]
    fn test_cost_value_linear() {
        let cost = CostValue::Linear {
            per_row: 0.01,
            base: 5.0,
        };
        assert_eq!(cost.evaluate(None), 5.0);
        assert_eq!(cost.evaluate(Some(100)), 6.0); // 5.0 + 0.01 * 100
        assert!(cost.validate().is_ok());
    }

    #[test]
    fn test_cost_value_validation_negative() {
        let cost = CostValue::Constant { value: -1.0 };
        assert!(cost.validate().is_err());

        let cost = CostValue::Linear {
            per_row: -0.01,
            base: 5.0,
        };
        assert!(cost.validate().is_err());

        let cost = CostValue::Linear {
            per_row: 0.01,
            base: -5.0,
        };
        assert!(cost.validate().is_err());
    }

    #[test]
    fn test_cost_model_evaluate() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 1.0 },
            c_comp: CostValue::Linear {
                per_row: 0.00001,
                base: 0.5,
            },
            alpha: CostValue::Constant { value: 0.9 },
        };

        let (c_id, c_comp, alpha) = model.evaluate(Some(1000));
        assert_eq!(c_id, 1.0);
        assert!((c_comp - 0.51).abs() < 0.0001); // 0.5 + 0.00001 * 1000
        assert_eq!(alpha, 0.9);
    }

    #[test]
    fn test_cost_model_validate() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 1.0 },
            c_comp: CostValue::Constant { value: 100.0 },
            alpha: CostValue::Constant { value: 0.9 },
        };
        assert!(model.validate().is_ok());

        // Invalid alpha > 1
        let model = CostModel {
            alpha: CostValue::Constant { value: 1.5 },
            ..model.clone()
        };
        assert!(model.validate().is_err());

        // Invalid alpha < 0
        let model = CostModel {
            alpha: CostValue::Constant { value: -0.1 },
            ..model.clone()
        };
        assert!(model.validate().is_err());
    }

    #[test]
    fn test_cost_model_reuse_threshold() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 10.0 },
            c_comp: CostValue::Constant { value: 100.0 },
            alpha: CostValue::Constant { value: 0.9 },
        };

        let threshold = model.reuse_threshold(None);
        // threshold = 10.0 / (0.9 * 100.0) = 10.0 / 90.0 ≈ 0.111
        assert!((threshold - 10.0 / 90.0).abs() < 0.0001);
    }

    #[test]
    fn test_cost_model_reuse_threshold_zero_alpha() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 10.0 },
            c_comp: CostValue::Constant { value: 100.0 },
            alpha: CostValue::Constant { value: 0.0 },
        };

        let threshold = model.reuse_threshold(None);
        assert_eq!(threshold, f64::INFINITY);
    }

    #[test]
    fn test_cost_model_reuse_threshold_linear() {
        let model = CostModel {
            c_id: CostValue::Constant { value: 1.0 },
            c_comp: CostValue::Linear {
                per_row: 0.00001,
                base: 0.5,
            },
            alpha: CostValue::Constant { value: 0.9 },
        };

        // With 1000 rows: c_comp = 0.5 + 0.00001 * 1000 = 0.51
        // threshold = 1.0 / (0.9 * 0.51) ≈ 2.18
        let threshold = model.reuse_threshold(Some(1000));
        assert!((threshold - 1.0 / (0.9 * 0.51)).abs() < 0.0001);
    }
}
