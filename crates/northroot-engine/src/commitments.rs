//! Commitment computation primitives for deterministic hashing.
//!
//! This module provides canonical hashing functions used throughout the engine
//! for computing deterministic commitments to data structures. All functions
//! use SHA-256 with domain separation patterns inspired by RFC-6962.
//!
//! ## Core Functions
//!
//! - **`sha256_prefixed()`**: SHA-256 hash with `sha256:` prefix format
//! - **`jcs()`**: JSON Canonicalization (RFC 8785) with sorted keys - **for engine internal computations only, NOT receipt canonicalization**
//! - **`cbor_deterministic()`**: CBOR deterministic encoding (RFC 8949) - *re-exported from `northroot-receipts`*
//! - **`cbor_hash()`**: SHA-256 hash of deterministic CBOR - *re-exported from `northroot-receipts`*
//! - **`validate_cbor_deterministic()`**: Validate CBOR deterministic encoding - *re-exported from `northroot-receipts`*
//! - **`commit_set_root()`**: Merkle root for unordered sets (order-independent)
//! - **`commit_seq_root()`**: Merkle root for ordered sequences (order-dependent)
//!
//! **Note:** CBOR canonicalization functions are re-exported from `northroot-receipts` per ADR-002,
//! which states that receipt canonicalization belongs in the receipts crate. The `jcs()` function
//! in this module is for engine-internal computations (e.g., Merkle tree leaf hashing) and is
//! separate from receipt canonicalization, which always uses CBOR.
//!
//! ## Domain Separation
//!
//! The engine uses domain separation to prevent hash collisions:
//! - Set roots: Sorted elements joined with `"|"` separator
//! - Sequence roots: Elements joined with `"|"` separator (preserves order)
//! - Merkle trees: Use prefix bytes (0x00 for leaves, 0x01 for parents)
//!
//! ## Examples
//!
//! ```rust
//! use northroot_engine::commitments::*;
//! use serde_json::json;
//!
//! // Canonical JSON (sorted keys)
//! let value = json!({"b": 2, "a": 1});
//! let canonical = jcs(&value); // {"a":1,"b":2}
//!
//! // Set root (order-independent)
//! let parts = vec!["c".to_string(), "a".to_string(), "b".to_string()];
//! let root1 = commit_set_root(&parts);
//! let parts2 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
//! let root2 = commit_set_root(&parts2); // Same result
//! assert_eq!(root1, root2);
//!
//! // Sequence root (order-dependent)
//! let seq1 = commit_seq_root(&["a".to_string(), "b".to_string(), "c".to_string()]);
//! let seq2 = commit_seq_root(&["c".to_string(), "b".to_string(), "a".to_string()]); // Different result
//! assert_ne!(seq1, seq2);
//! ```

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// Re-export CBOR canonicalization functions from receipts (per ADR-002)
pub use northroot_receipts::canonical::{
    cbor_deterministic, cbor_hash, validate_cbor_deterministic,
};

pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256:{:x}", h.finalize())
}

pub fn jcs(value: &Value) -> String {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut bm = BTreeMap::new();
                for (k, v) in m {
                    bm.insert(k.clone(), sort(v));
                }
                // Convert BTreeMap to serde_json::Map
                let json_map: serde_json::Map<String, Value> = bm.into_iter().collect();
                Value::Object(json_map)
            }
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            _ => v.clone(),
        }
    }
    serde_json::to_string(&sort(value)).unwrap()
}

pub fn commit_set_root(parts: &[String]) -> String {
    let mut v = parts.to_vec();
    v.sort();
    sha256_prefixed(v.join("|").as_bytes())
}

pub fn commit_seq_root(parts: &[String]) -> String {
    sha256_prefixed(parts.join("|").as_bytes())
}
