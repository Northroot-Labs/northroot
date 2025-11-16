//! Execution tracking and state management.
//!
//! This module provides utilities for tracking execution state, managing
//! trace IDs, span commitments, and computing execution roots.

pub mod builder;
pub mod state;

pub use builder::*;
pub use state::*;
