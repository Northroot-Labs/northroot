//! Execution tracking and state management.
//!
//! This module provides utilities for tracking execution state, managing
//! trace IDs, span commitments, and computing execution roots.

pub mod builder;
pub mod merkle_row_map;
pub mod state;

pub use builder::*;
pub use merkle_row_map::*;
pub use state::*;

