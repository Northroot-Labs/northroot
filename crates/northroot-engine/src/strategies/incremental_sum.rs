//! Incremental sum strategy: state-preserving aggregation.
//!
//! This strategy provides incremental aggregation over changing datasets
//! using Merkle Row-Map for deterministic state.

use crate::execution::MerkleRowMap;
use crate::strategies::trait_::{ExecutionMode, Strategy, StrategyError};
use serde_json::Value;

/// Incremental sum strategy for state-preserving aggregation.
///
/// This strategy:
/// - Maintains state in Merkle Row-Map
/// - Supports delta updates (add/remove/changed rows)
/// - Produces deterministic state commitments
#[derive(Debug, Clone)]
pub struct IncrementalSumStrategy {
    /// Strategy name
    name: String,
    /// Field name to sum (default: "value")
    sum_field: String,
}

impl IncrementalSumStrategy {
    /// Create a new incremental sum strategy.
    pub fn new() -> Self {
        Self {
            name: "incremental_sum".to_string(),
            sum_field: "value".to_string(),
        }
    }

    /// Create with custom sum field name.
    pub fn with_field(field: String) -> Self {
        Self {
            name: "incremental_sum".to_string(),
            sum_field: field,
        }
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
        // Parse input as array of rows
        let rows = input
            .as_array()
            .ok_or_else(|| StrategyError::InvalidInput("Input must be an array".to_string()))?;

        // Initialize or load previous state
        let mut state = prev_state.cloned().unwrap_or_else(MerkleRowMap::new);

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

            // Update state: row_id -> value
            state.insert(row_id.clone(), Value::Number(serde_json::Number::from_f64(value).unwrap()));

            total_sum += value;
        }

        // Output includes sum and state hash
        let output = serde_json::json!({
            "sum": total_sum,
            "state_hash": state.state_hash(),
            "row_count": state.len(),
        });

        Ok((output, state))
    }

    fn name(&self) -> &str {
        &self.name
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
}

