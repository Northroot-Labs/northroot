//! Execution tracking and state management.
//!
//! This module provides utilities for tracking execution state, managing
//! trace IDs, span commitments, and computing execution roots.

pub mod builder;
pub mod state;

// MerkleRowMap moved to rowmap.rs module (RFC-6962 domain separation)
// Re-export for backward compatibility
pub use crate::rowmap::MerkleRowMap;

pub use builder::*;
pub use state::*;
