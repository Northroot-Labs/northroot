//! Partition strategy: stable row-based chunking.
//!
//! This strategy provides stable, deterministic chunking of input data
//! with per-row hashing for delta compute operations.

use crate::delta::{chunk_id_from_bytes, jaccard_similarity, OverlapMetric};
use crate::execution::MerkleRowMap;
use crate::strategies::trait_::{ExecutionMode, Strategy, StrategyError};
use crate::ReuseIndexed;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::RwLock;

/// Partition strategy for stable row-based chunking.
///
/// This strategy:
/// - Parses input data into rows
/// - Generates stable chunk IDs per row
/// - Creates a chunk index for delta compute
#[derive(Debug)]
pub struct PartitionStrategy {
    /// Strategy name
    name: String,
    /// Computed overlap metric (set after execute) - thread-safe interior mutability
    overlap_metric: RwLock<Option<OverlapMetric>>,
}

impl PartitionStrategy {
    /// Create a new partition strategy.
    pub fn new() -> Self {
        Self {
            name: "partition".to_string(),
            overlap_metric: RwLock::new(None),
        }
    }

    /// Get chunk IDs from MerkleRowMap state.
    fn chunk_ids_from_state(state: &MerkleRowMap) -> HashSet<String> {
        state.iter().map(|(k, _)| k.clone()).collect()
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
        prev_state: Option<&MerkleRowMap>,
    ) -> Result<(Value, MerkleRowMap), StrategyError> {
        // Parse input as array of rows
        let rows = input
            .as_array()
            .ok_or_else(|| StrategyError::InvalidInput("Input must be an array".to_string()))?;

        // Compute overlap metric during execution
        let previous_chunks: HashSet<String> = prev_state
            .map(|ps| Self::chunk_ids_from_state(ps))
            .unwrap_or_default();

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

        // Compute overlap metric: compare current chunks with previous chunks
        let current_chunks = Self::chunk_ids_from_state(&chunk_index);
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
        *self.overlap_metric.write().unwrap() = Some(metric);

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

impl ReuseIndexed for PartitionStrategy {
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

    #[test]
    fn test_partition_reuse_indexed() {
        use crate::ReuseIndexed;

        let strategy = PartitionStrategy::new();

        // Initial state
        let input1 = serde_json::json!([
            {"id": 1, "value": "a"},
            {"id": 2, "value": "b"},
        ]);

        let (_, state1) = strategy
            .execute(&input1, ExecutionMode::Full, None)
            .unwrap();

        // Delta: add one row, keep one row
        let input2 = serde_json::json!([
            {"id": 2, "value": "b"}, // Same row
            {"id": 3, "value": "c"}, // New row
        ]);

        let (_, _) = strategy
            .execute(&input2, ExecutionMode::Delta, Some(&state1))
            .unwrap();

        // Check overlap metric
        let metric = strategy.overlap();
        // Note: Partition strategy creates new chunk index each time, so
        // current state has 2 chunks (from input2), previous state has 2 chunks (from input1)
        // Intersection: 1 chunk (id:2)
        // Union: 3 chunks (id:1, id:2, id:3)
        // Jaccard = 1 / 3 ≈ 0.333
        assert!(metric.jaccard > 0.3 && metric.jaccard < 0.34);
        assert_eq!(metric.chunk_count_current, 2);
        assert_eq!(metric.chunk_count_previous, 2);
        assert_eq!(metric.chunk_count_intersection, 1);
    }
}

