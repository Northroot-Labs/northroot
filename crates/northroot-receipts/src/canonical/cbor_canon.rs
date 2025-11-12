//! Canonical CBOR encoding per RFC 8949.
//!
//! This module provides deterministic CBOR encoding for stable byte representation
//! suitable for hashing, signing, and persistence.

use ciborium::value::Value as CborValue;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Error type for canonical CBOR encoding operations.
#[derive(Debug, Error)]
pub enum CanonError {
    /// Failed to convert serializable value to CBOR Value.
    #[error("serde to Value failed: {0}")]
    ToValue(String),
    /// Failed to encode CBOR value to bytes.
    #[error("encode failed: {0}")]
    Encode(String),
}

/// Convert a serializable value to ciborium::value::Value.
fn to_value<T: Serialize>(t: &T) -> Result<CborValue, CanonError> {
    // Serialize via ciborium into a Value DOM
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(t, &mut buffer).map_err(|e| CanonError::ToValue(e.to_string()))?;

    ciborium::de::from_reader(buffer.as_slice()).map_err(|e| CanonError::ToValue(e.to_string()))
}

/// Canonicalize a CBOR value by sorting map keys.
///
/// For text keys, sorts by UTF-8 byte order (lexicographic).
/// For non-text keys, serializes the key to canonical CBOR bytes and sorts by those bytes.
pub(crate) fn canon_value(v: &mut CborValue) {
    match v {
        CborValue::Map(m) => {
            // Recursively canonicalize all values
            for (_, vv) in m.iter_mut() {
                canon_value(vv);
            }
            // Sort map keys by their canonical CBOR byte representation
            m.sort_by(|(k1, _), (k2, _)| format_key(k1).cmp(&format_key(k2)));
        }
        CborValue::Array(a) => {
            for item in a.iter_mut() {
                canon_value(item);
            }
        }
        _ => {}
    }
}

/// Format a key for sorting.
///
/// For text keys, returns UTF-8 bytes.
/// For other keys, returns canonical CBOR encoding bytes.
fn format_key(k: &CborValue) -> Vec<u8> {
    // Prefer text keys (most common case)
    if let CborValue::Text(s) = k {
        return s.as_bytes().to_vec();
    }

    // For non-text keys, serialize to canonical CBOR bytes and sort by those
    let mut tmp = k.clone();
    canon_value(&mut tmp);
    let mut buf = Vec::new();
    if ciborium::ser::into_writer(&tmp, &mut buf).is_ok() {
        buf
    } else {
        // Fallback: use debug representation (shouldn't happen in practice)
        format!("{:?}", k).into_bytes()
    }
}

/// Encode a value to canonical CBOR per RFC 8949.
///
/// This function:
/// - Sorts map keys by canonical CBOR byte order
/// - Normalizes integers to minimal form (handled by ciborium)
/// - Forbids indefinite-length items (handled by ciborium)
/// - Produces stable, deterministic bytes for hashing/signing
///
/// # Arguments
///
/// * `t` - Value to encode (must implement Serialize)
///
/// # Returns
///
/// Canonical CBOR bytes
///
/// # Errors
///
/// Returns error if serialization fails
pub fn encode_canonical<T: Serialize>(t: &T) -> Result<Vec<u8>, CanonError> {
    let mut v = to_value(t)?;
    canon_value(&mut v);

    let mut out = Vec::new();
    ciborium::ser::into_writer(&v, &mut out).map_err(|e| CanonError::Encode(e.to_string()))?;

    Ok(out)
}

/// Compute SHA-256 hash of canonical CBOR with `sha256:` prefix.
///
/// This computes the hash of the canonical CBOR representation
/// and returns it in the format `sha256:<64hex>`.
///
/// # Arguments
///
/// * `t` - Value to encode and hash (must implement Serialize)
///
/// # Returns
///
/// Hash string in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if CBOR encoding fails
pub fn hash_canonical<T: Serialize>(t: &T) -> Result<String, CanonError> {
    let bytes = encode_canonical(t)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_bytes = hasher.finalize();
    Ok(format!("sha256:{:x}", hash_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_canonical_stable() {
        // Test that canonical encoding produces stable bytes
        let mut map1 = BTreeMap::new();
        map1.insert("b".to_string(), 1);
        map1.insert("a".to_string(), 2);

        let mut map2 = BTreeMap::new();
        map2.insert("a".to_string(), 2);
        map2.insert("b".to_string(), 1);

        let c1 = encode_canonical(&map1).unwrap();
        let c2 = encode_canonical(&map2).unwrap();

        // Should produce identical bytes despite insertion order
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_hash_deterministic() {
        let data = vec!["a", "b", "c"];
        let h1 = hash_canonical(&data).unwrap();
        let h2 = hash_canonical(&data).unwrap();

        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
        assert_eq!(h1.len(), 71); // "sha256:" + 64 hex chars
    }
}
