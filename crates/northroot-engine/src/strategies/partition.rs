//! Partition strategy: stable row-based chunking.
//!
//! This strategy provides stable, deterministic chunking of input data
//! with per-row hashing for delta compute operations.

use crate::delta::chunk_id_from_bytes;
use crate::execution::MerkleRowMap;
use crate::strategies::trait_::{ExecutionMode, Strategy, StrategyError};
use serde_json::Value;

/// Partition strategy for stable row-based chunking.
///
/// This strategy:
/// - Parses input data into rows
/// - Generates stable chunk IDs per row
/// - Creates a chunk index for delta compute
#[derive(Debug, Clone)]
pub struct PartitionStrategy {
    /// Strategy name
    name: String,
}

impl PartitionStrategy {
    /// Create a new partition strategy.
    pub fn new() -> Self {
        Self {
            name: "partition".to_string(),
        }
    }
}

impl Default for PartitionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for PartitionStrategy {
    fn execute(
        &self,
        input: &Value,
        _mode: ExecutionMode,
        _prev_state: Option<&MerkleRowMap>,
    ) -> Result<(Value, MerkleRowMap), StrategyError> {
        // Parse input as array of rows
        let rows = input
            .as_array()
            .ok_or_else(|| StrategyError::InvalidInput("Input must be an array".to_string()))?;

        // Create chunk index: map row hash -> row index
        let mut chunk_index = MerkleRowMap::new();

        for (idx, row) in rows.iter().enumerate() {
            // Canonicalize row to bytes
            let row_bytes = serde_json::to_string(row)
                .map_err(|e| StrategyError::ExecutionFailed(format!("Serialization failed: {}", e)))?;

            // Generate stable chunk ID
            let chunk_id = chunk_id_from_bytes(row_bytes.as_bytes());

            // Store in chunk index: chunk_id -> row index
            chunk_index.insert(chunk_id, Value::Number(idx.into()));
        }

        // Output is the chunk index state
        let output = serde_json::json!({
            "chunk_count": chunk_index.len(),
            "state_hash": chunk_index.state_hash(),
        });

        Ok((output, chunk_index))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_strategy() {
        let strategy = PartitionStrategy::new();
        let input = serde_json::json!([
            {"id": 1, "value": "a"},
            {"id": 2, "value": "b"},
            {"id": 3, "value": "c"},
        ]);

        let (output, state) = strategy
            .execute(&input, ExecutionMode::Full, None)
            .unwrap();

        assert_eq!(output["chunk_count"], 3);
        assert!(state.len() == 3);
    }

    #[test]
    fn test_partition_strategy_deterministic() {
        let strategy = PartitionStrategy::new();
        let input = serde_json::json!([
            {"id": 1, "value": "a"},
            {"id": 2, "value": "b"},
        ]);

        let (_, state1) = strategy
            .execute(&input, ExecutionMode::Full, None)
            .unwrap();

        let (_, state2) = strategy
            .execute(&input, ExecutionMode::Full, None)
            .unwrap();

        // Same input should produce same state hash
        assert_eq!(state1.state_hash(), state2.state_hash());
    }
}

