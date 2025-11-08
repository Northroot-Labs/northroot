//! Northroot Engine: Proof algebra implementation and execution engine.
//!
//! This crate provides the core engine for the Northroot proof algebra system,
//! including receipt composition, validation, and execution tracking.

pub mod commitments;

pub use commitments::{commit_seq_root, commit_set_root, jcs, sha256_prefixed};
