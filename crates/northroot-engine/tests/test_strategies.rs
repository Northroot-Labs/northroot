//! Tests for strategies framework.

use northroot_engine::strategies::*;
use serde_json::json;

#[test]
fn test_partition_strategy() {
    let strategy = PartitionStrategy::new();
    let input = json!([
        {"id": 1, "value": "a"},
        {"id": 2, "value": "b"},
        {"id": 3, "value": "c"},
    ]);

    let (output, state) = strategy.execute(&input, ExecutionMode::Full, None).unwrap();

    assert_eq!(output["chunk_count"], 3);
    assert_eq!(state.len(), 3);
    assert_eq!(strategy.name(), "partition");
}

#[test]
fn test_partition_strategy_deterministic() {
    let strategy = PartitionStrategy::new();
    let input = json!([
        {"id": 1, "value": "a"},
        {"id": 2, "value": "b"},
    ]);

    let (_, state1) = strategy.execute(&input, ExecutionMode::Full, None).unwrap();

    let (_, state2) = strategy.execute(&input, ExecutionMode::Full, None).unwrap();

    // Same input should produce same state hash
    assert_eq!(state1.state_hash(), state2.state_hash());
}

#[test]
fn test_incremental_sum_strategy_full() {
    let strategy = IncrementalSumStrategy::new();
    let input = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
        {"id": "3", "value": 30.0},
    ]);

    let (output, state) = strategy.execute(&input, ExecutionMode::Full, None).unwrap();

    assert_eq!(output["sum"], 60.0);
    assert_eq!(state.len(), 3);
    assert_eq!(strategy.name(), "incremental_sum");
}

#[test]
fn test_incremental_sum_strategy_delta() {
    let strategy = IncrementalSumStrategy::new();

    // Initial state
    let input1 = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
    ]);

    let (_, state1) = strategy
        .execute(&input1, ExecutionMode::Full, None)
        .unwrap();

    // Delta: add one row
    let input2 = json!([
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
fn test_strategy_composition() {
    // Test that strategies can be composed
    let partition = PartitionStrategy::new();
    let sum = IncrementalSumStrategy::new();

    let input = json!([
        {"id": "1", "value": 10.0},
        {"id": "2", "value": 20.0},
    ]);

    // First strategy: partition
    let (_, partition_state) = partition
        .execute(&input, ExecutionMode::Full, None)
        .unwrap();

    // Second strategy: sum (can use partition state if needed)
    let (sum_output, _) = sum.execute(&input, ExecutionMode::Full, None).unwrap();

    assert_eq!(sum_output["sum"], 30.0);
    assert!(partition_state.len() > 0);
}
