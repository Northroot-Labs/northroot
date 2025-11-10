//! Composable compute strategies for delta/incremental compute.
//!
//! This module provides a trait-based framework for implementing incremental
//! recomputation strategies with reuse decisions.

pub mod incremental_sum;
pub mod partition;
pub mod registry;
pub mod trait_;

pub use incremental_sum::*;
pub use partition::*;
pub use registry::*;
pub use trait_::*;
