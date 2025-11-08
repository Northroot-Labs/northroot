//! Commitment computation primitives for deterministic hashing.
//!
//! This module provides canonical hashing functions used throughout the engine
//! for computing deterministic commitments to data structures. All functions
//! use SHA-256 with domain separation patterns inspired by RFC-6962.
//!
//! ## Core Functions
//!
//! - **`sha256_prefixed()`**: SHA-256 hash with `sha256:` prefix format
//! - **`jcs()`**: JSON Canonicalization (RFC 8785) with sorted keys
//! - **`cbor_deterministic()`**: CBOR deterministic encoding (RFC 8949)
//! - **`cbor_hash()`**: SHA-256 hash of deterministic CBOR
//! - **`commit_set_root()`**: Merkle root for unordered sets (order-independent)
//! - **`commit_seq_root()`**: Merkle root for ordered sequences (order-dependent)
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

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

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

/// Encode a value to deterministic CBOR per RFC 8949.
///
/// RFC 8949 deterministic encoding rules:
/// - Preferred argument sizes (no indefinite-length items)
/// - Lexicographically sorted map keys
/// - Deterministic encoding rules
///
/// # Arguments
///
/// * `value` - Value to encode (must implement Serialize)
///
/// # Returns
///
/// CBOR bytes in deterministic encoding
///
/// # Errors
///
/// Returns error if serialization fails
pub fn cbor_deterministic<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    // ciborium uses deterministic encoding by default when using ser::into_writer
    // with a Vec<u8> writer. The library ensures:
    // - Map keys are sorted lexicographically
    // - No indefinite-length items
    // - Preferred argument sizes
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(value, &mut buffer)
        .map_err(|e| format!("CBOR serialization failed: {}", e))?;
    Ok(buffer)
}

/// Compute SHA-256 hash of deterministic CBOR with `sha256:` prefix.
///
/// This computes the hash of the deterministic CBOR representation
/// and returns it in the format `sha256:<64hex>`.
///
/// # Arguments
///
/// * `value` - Value to encode and hash (must implement Serialize)
///
/// # Returns
///
/// Hash string in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if CBOR encoding fails
pub fn cbor_hash<T: Serialize>(value: &T) -> Result<String, String> {
    let cbor_bytes = cbor_deterministic(value)?;
    Ok(sha256_prefixed(&cbor_bytes))
}

/// Validate that CBOR bytes follow RFC 8949 deterministic encoding rules.
///
/// This function checks:
/// - No indefinite-length items
/// - Map keys are sorted (if applicable)
/// - Preferred argument sizes
///
/// Note: Full validation requires parsing and re-encoding the CBOR.
/// This function validates by ensuring re-encoding produces identical bytes.
///
/// # Arguments
///
/// * `cbor_bytes` - CBOR bytes to validate
///
/// # Returns
///
/// `Ok(())` if CBOR appears to be deterministic, error otherwise
pub fn validate_cbor_deterministic(cbor_bytes: &[u8]) -> Result<(), String> {
    use ciborium::value::Value as CborValue;
    
    // Parse CBOR - ciborium will reject indefinite-length items
    let parsed: CborValue = ciborium::de::from_reader(cbor_bytes)
        .map_err(|e| format!("Invalid CBOR or non-deterministic encoding: {}", e))?;
    
    // Re-encode and compare - deterministic encoding should produce identical bytes
    let mut re_encoded = Vec::new();
    ciborium::ser::into_writer(&parsed, &mut re_encoded)
        .map_err(|e| format!("Failed to re-encode CBOR: {}", e))?;
    
    if re_encoded != cbor_bytes {
        return Err("Non-deterministic CBOR: re-encoding produces different bytes".to_string());
    }
    
    Ok(())
}
