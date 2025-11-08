//! Incremental sum strategy: state-preserving aggregation.
//!
//! This strategy provides incremental aggregation over changing datasets
//! using Merkle Row-Map for deterministic state.

use crate::delta::{economic_delta, jaccard_similarity, CostModel, OverlapMetric};
use crate::execution::MerkleRowMap;
use crate::strategies::trait_::{ExecutionMode, Strategy, StrategyError};
use crate::ReuseIndexed;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::RwLock;

/// Cost allocation for partition-level economic tracking.
///
/// Tracks the economic delta (ΔC) and expected reuse window per partition
/// to enable deterministic FinOps receipts.
#[derive(Debug, Clone, PartialEq)]
pub struct CostAllocation {
    /// Economic delta: α · C_comp · J - C_id
    pub delta_c: f64,
    /// Expected reuse window in seconds (optional)
    pub expected_reuse_window: Option<u64>,
    /// Partition identifier (optional)
    pub partition_id: Option<String>,
}

/// Incremental sum strategy for state-preserving aggregation.
///
/// This strategy:
/// - Maintains state in Merkle Row-Map
/// - Supports delta updates (add/remove/changed rows)
/// - Produces deterministic state commitments
/// - Computes cost allocation (ΔC) per partition when CostModel is provided
#[derive(Debug)]
pub struct IncrementalSumStrategy {
    /// Strategy name
    name: String,
    /// Field name to sum (default: "value")
    sum_field: String,
    /// Computed overlap metric (set after execute) - thread-safe interior mutability
    overlap_metric: RwLock<Option<OverlapMetric>>,
    /// Optional cost model for computing economic delta (ΔC)
    cost_model: Option<CostModel>,
    /// Computed cost allocation (set after execute) - thread-safe interior mutability
    cost_allocation: RwLock<Option<CostAllocation>>,
}

impl IncrementalSumStrategy {
    /// Create a new incremental sum strategy.
    pub fn new() -> Self {
        Self {
            name: "incremental_sum".to_string(),
            sum_field: "value".to_string(),
            overlap_metric: RwLock::new(None),
            cost_model: None,
            cost_allocation: RwLock::new(None),
        }
    }

    /// Create with custom sum field name.
    pub fn with_field(field: String) -> Self {
        Self {
            name: "incremental_sum".to_string(),
            sum_field: field,
            overlap_metric: RwLock::new(None),
            cost_model: None,
            cost_allocation: RwLock::new(None),
        }
    }

    /// Create with cost model for economic delta computation.
    pub fn with_cost_model(mut self, cost_model: CostModel) -> Self {
        self.cost_model = Some(cost_model);
        self
    }

    /// Get chunk IDs from MerkleRowMap state.
    fn chunk_ids_from_state(state: &MerkleRowMap) -> HashSet<String> {
        state.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Compute cost allocation from overlap metric and cost model.
    ///
    /// Computes economic delta (ΔC) = α · C_comp · J - C_id
    ///
    /// # Arguments
    ///
    /// * `overlap_metric` - Overlap metric with Jaccard similarity
    /// * `cost_model` - Cost model with α, C_comp, C_id
    /// * `partition_id` - Optional partition identifier
    ///
    /// # Returns
    ///
    /// CostAllocation with computed ΔC
    pub fn compute_cost_allocation(
        overlap_metric: &OverlapMetric,
        cost_model: &CostModel,
        partition_id: Option<String>,
    ) -> CostAllocation {
        let delta_c = economic_delta(overlap_metric.jaccard, cost_model);
        CostAllocation {
            delta_c,
            expected_reuse_window: None, // Can be set based on historical data
            partition_id,
        }
    }

    /// Get the computed cost allocation (if available).
    pub fn cost_allocation(&self) -> Option<CostAllocation> {
        self.cost_allocation.read().unwrap().clone()
    }
}

impl Default for IncrementalSumStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for IncrementalSumStrategy {
    fn execute(
        &self,
        input: &Value,
        mode: ExecutionMode,
        prev_state: Option<&MerkleRowMap>,
    ) -> Result<(Value, MerkleRowMap), StrategyError> {
        // Validate input format
        let rows = input
            .as_array()
            .ok_or_else(|| StrategyError::InvalidInput("Input must be an array".to_string()))?;

        // Validate that prev_state is provided in delta mode
        if matches!(mode, ExecutionMode::Delta) && prev_state.is_none() {
            return Err(StrategyError::StateMismatch(
                "Previous state is required for delta mode execution".to_string(),
            ));
        }

        // Initialize or load previous state
        let mut state = prev_state.cloned().unwrap_or_else(MerkleRowMap::new);
        
        // Compute overlap metric during execution
        let previous_chunks: HashSet<String> = prev_state
            .map(|ps| Self::chunk_ids_from_state(ps))
            .unwrap_or_default();

        let mut incremental_sum = 0.0;
        let mut total_sum = 0.0;

        // Process each row
        for row in rows {
            // Extract row identifier (use "id" field or hash of row)
            let row_id = if let Some(id) = row.get("id") {
                id.to_string()
            } else {
                // Fallback: hash of entire row
                let row_str = serde_json::to_string(row)
                    .map_err(|e| StrategyError::ExecutionFailed(format!("Serialization failed: {}", e)))?;
                crate::delta::chunk_id_from_str(&row_str)
            };

            // Extract value to sum
            let value = row
                .get(&self.sum_field)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| {
                    StrategyError::InvalidInput(format!(
                        "Row missing or invalid '{}' field",
                        self.sum_field
                    ))
                })?;

            // Handle delta mode: only process new/changed rows
            let is_new_or_changed = match mode {
                ExecutionMode::Delta => {
                    // In delta mode, check if row is new or changed
                    let prev_value = state.get(&row_id)
                        .and_then(|v| v.as_f64());
                    prev_value.is_none() || prev_value != Some(value)
                }
                ExecutionMode::Full => {
                    // In full mode, process all rows
                    true
                }
            };

            if is_new_or_changed {
                // Update state: row_id -> value
                let prev_value = state.insert(
                    row_id.clone(),
                    Value::Number(serde_json::Number::from_f64(value).unwrap())
                );

                // Track incremental sum (delta mode) or total sum (full mode)
                match mode {
                    ExecutionMode::Delta => {
                        // In delta mode, only sum the change
                        if let Some(prev) = prev_value.and_then(|v| v.as_f64()) {
                            incremental_sum += value - prev; // Change in value
                        } else {
                            incremental_sum += value; // New row
                        }
                    }
                    ExecutionMode::Full => {
                        // In full mode, sum all values
                        incremental_sum += value;
                    }
                }
            }

            // Always track total sum for output
            total_sum += value;
        }

        // In full mode, total_sum is the sum of all rows
        // In delta mode, output the incremental sum (change) not the total
        // The test expects the sum of new/changed rows only
        let output_sum = match mode {
            ExecutionMode::Full => total_sum,
            ExecutionMode::Delta => incremental_sum, // Delta mode outputs incremental change
        };

        // Compute overlap metric: compare current chunks with previous chunks
        let current_chunks = Self::chunk_ids_from_state(&state);
        let intersection: HashSet<String> = current_chunks
            .intersection(&previous_chunks)
            .cloned()
            .collect();
        let jaccard = jaccard_similarity(&current_chunks, &previous_chunks);
        let metric = OverlapMetric::new(
            jaccard,
            current_chunks.len(),
            previous_chunks.len(),
            intersection.len(),
        );
        *self.overlap_metric.write().unwrap() = Some(metric.clone());

        // Compute cost allocation if cost model is provided
        if let Some(ref cost_model) = self.cost_model {
            let cost_alloc = Self::compute_cost_allocation(&metric, cost_model, None);
            *self.cost_allocation.write().unwrap() = Some(cost_alloc);
        }

        // Output includes sum and state hash
        let output = serde_json::json!({
            "sum": output_sum,
            "incremental_sum": incremental_sum,
            "state_hash": state.state_hash(),
            "row_count": state.len(),
        });

        Ok((output, state))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl ReuseIndexed for IncrementalSumStrategy {
    fn overlap(&self) -> OverlapMetric {
        self.overlap_metric
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                // Default: no overlap if metric not computed yet
                OverlapMetric::new(0.0, 0, 0, 0)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_sum_strategy_full() {
        let strategy = IncrementalSumStrategy::new();
        let input = serde_json::json!([
            {"id": "1", "value": 10.0},
            {"id": "2", "value": 20.0},
            {"id": "3", "value": 30.0},
        ]);

        let (output, state) = strategy
            .execute(&input, ExecutionMode::Full, None)
            .unwrap();

        assert_eq!(output["sum"], 60.0);
        assert_eq!(state.len(), 3);
    }

    #[test]
    fn test_incremental_sum_strategy_delta() {
        let strategy = IncrementalSumStrategy::new();

        // Initial state
        let input1 = serde_json::json!([
            {"id": "1", "value": 10.0},
            {"id": "2", "value": 20.0},
        ]);

        let (_, state1) = strategy
            .execute(&input1, ExecutionMode::Full, None)
            .unwrap();

        // Delta: add one row
        let input2 = serde_json::json!([
            {"id": "3", "value": 30.0},
        ]);

        let (output2, state2) = strategy
            .execute(&input2, ExecutionMode::Delta, Some(&state1))
            .unwrap();

        // State should include all rows
        assert_eq!(state2.len(), 3);
        assert_eq!(output2["sum"], 30.0); // Only new row's value
    }

    #[test]
    fn test_incremental_sum_reuse_indexed() {
        use crate::ReuseIndexed;

        let strategy = IncrementalSumStrategy::new();

        // Initial state
        let input1 = serde_json::json!([
            {"id": "1", "value": 10.0},
            {"id": "2", "value": 20.0},
        ]);

        let (_, state1) = strategy
            .execute(&input1, ExecutionMode::Full, None)
            .unwrap();

        // Delta: add one row, keep one row
        let input2 = serde_json::json!([
            {"id": "2", "value": 20.0}, // Same row
            {"id": "3", "value": 30.0}, // New row
        ]);

        let (_, _) = strategy
            .execute(&input2, ExecutionMode::Delta, Some(&state1))
            .unwrap();

        // Check overlap metric
        let metric = strategy.overlap();
        // Current state (after input2): {id:1, id:2, id:3} (state accumulates all rows)
        // Previous state (after input1): {id:1, id:2}
        // Intersection: {id:1, id:2} = 2 chunks
        // Union: {id:1, id:2, id:3} = 3 chunks
        // Jaccard = 2 / 3 ≈ 0.667
        assert!(metric.jaccard > 0.66 && metric.jaccard < 0.67);
        assert_eq!(metric.chunk_count_current, 3);
        assert_eq!(metric.chunk_count_previous, 2);
        assert_eq!(metric.chunk_count_intersection, 2);
    }

    #[test]
    fn test_incremental_sum_cost_allocation() {
        // Test cost allocation computation: ΔC = α · C_comp · J - C_id
        // With α=0.9, C_comp=100.0, C_id=10.0, J=0.8
        // Expected: ΔC = 0.9 * 100.0 * 0.8 - 10.0 = 72.0 - 10.0 = 62.0

        let cost_model = CostModel::new(10.0, 100.0, 0.9);
        let strategy = IncrementalSumStrategy::new().with_cost_model(cost_model.clone());

        // Initial state with 5 rows
        let initial_input = serde_json::json!([
            {"id": "1", "value": 10.0},
            {"id": "2", "value": 20.0},
            {"id": "3", "value": 30.0},
            {"id": "4", "value": 40.0},
            {"id": "5", "value": 50.0},
        ]);

        let (_, prev_state) = strategy
            .execute(&initial_input, ExecutionMode::Full, None)
            .unwrap();

        // Delta input: 4 rows overlap (80%), 1 new row
        // Jaccard = 4 / 6 = 0.667 (4 overlap, 6 total unique)
        let delta_input = serde_json::json!([
            {"id": "1", "value": 10.0}, // Same
            {"id": "2", "value": 20.0}, // Same
            {"id": "3", "value": 30.0}, // Same
            {"id": "4", "value": 40.0}, // Same
            {"id": "6", "value": 60.0}, // New
        ]);

        let (_, _) = strategy
            .execute(&delta_input, ExecutionMode::Delta, Some(&prev_state))
            .unwrap();

        // Check cost allocation
        let cost_alloc = strategy.cost_allocation().expect("Cost allocation should be computed");

        // Verify ΔC computation
        // State accumulates all rows, so:
        // Previous state: {1, 2, 3, 4, 5} = 5 rows
        // Current state: {1, 2, 3, 4, 5, 6} = 6 rows (includes all from previous + new row 6)
        // Intersection: {1, 2, 3, 4, 5} = 5 rows
        // Union: {1, 2, 3, 4, 5, 6} = 6 rows
        // J = 5/6 = 0.833
        // ΔC = 0.9 * 100.0 * 0.833 - 10.0 = 75.0 - 10.0 = 65.0
        let expected_delta_c = 0.9 * 100.0 * (5.0 / 6.0) - 10.0;
        assert!(
            (cost_alloc.delta_c - expected_delta_c).abs() < 0.01,
            "ΔC should be approximately {:.2}, got {:.2}",
            expected_delta_c,
            cost_alloc.delta_c
        );
        assert!(cost_alloc.delta_c > 0.0, "ΔC should be positive for reuse case");
    }
}

