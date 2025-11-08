//! Northroot Commons — Shared utilities and cross-cutting concerns.
//!
//! **Boundary**: This crate provides cross-cutting utilities with NO internal dependencies.
//! All other crates may depend on this, but this crate must NOT depend on any other
//! northroot crate (see ADR_PLAYBOOK.md).
//!
//! **Purpose**: Common utilities, error types, logging, shared traits that don't belong
//! to a specific domain (receipts, engine, policy, ops).
//!
//! This crate provides common utilities used across the Northroot workspace,
//! including error types, logging helpers, and shared data structures.

#![deny(missing_docs)]

// Placeholder for future common utilities
// TODO: Add error types, logging helpers, etc. as needed
