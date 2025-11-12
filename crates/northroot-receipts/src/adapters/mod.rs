//! Adapter layer for external format compatibility.
//!
//! This module provides thin adapters for converting between CBOR (internal)
//! and JSON (external) formats. JSON is only used at API boundaries and
//! for human-readable test vectors.

pub mod json;

pub use json::{cbor_to_json, json_to_cbor, receipt_from_json, receipt_to_json, AdapterError};
