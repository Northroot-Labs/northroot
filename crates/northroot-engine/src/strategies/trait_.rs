//! Strategy trait for composable compute patterns.
//!
//! This module defines the common interface for all compute strategies.

use crate::execution::MerkleRowMap;
use serde_json::Value as JsonValue;

/// Execution mode for strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Full execution: recompute everything
    Full,
    /// Delta execution: reuse previous results where possible
    Delta,
}

/// Strategy trait for composable compute patterns.
///
/// Strategies implement incremental recomputation with reuse decisions.
/// They can be composed in pipelines to enable efficient delta compute workflows.
pub trait Strategy {
    /// Execute the strategy in the given mode.
    ///
    /// # Arguments
    ///
    /// * `input` - Input data
    /// * `mode` - Execution mode (full or delta)
    /// * `prev_state` - Previous state (for delta mode)
    ///
    /// # Returns
    ///
    /// Output value and new state
    fn execute(
        &self,
        input: &JsonValue,
        mode: ExecutionMode,
        prev_state: Option<&MerkleRowMap>,
    ) -> Result<(JsonValue, MerkleRowMap), StrategyError>;

    /// Get the strategy name.
    fn name(&self) -> &str;
}

/// Error types for strategy execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyError {
    /// Strategy execution failed
    ExecutionFailed(String),
    /// Invalid input format
    InvalidInput(String),
    /// State mismatch
    StateMismatch(String),
}

impl std::fmt::Display for StrategyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyError::ExecutionFailed(msg) => {
                write!(f, "Strategy execution failed: {}", msg)
            }
            StrategyError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
            StrategyError::StateMismatch(msg) => {
                write!(f, "State mismatch: {}", msg)
            }
        }
    }
}

impl std::error::Error for StrategyError {}
