//! Domain-agnostic governance event schemas for Northroot.
//!
//! This crate provides typed event structures for checkpoint and attestation events,
//! which are domain-agnostic governance primitives that can be used by any domain.

#![deny(missing_docs)]

pub mod attestation;
pub mod checkpoint;
pub mod signature;

pub use attestation::AttestationEvent;
pub use checkpoint::{CheckpointEvent, MerkleWindow};
pub use signature::Signature;

