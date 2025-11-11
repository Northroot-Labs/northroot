//! Merkle Row-Map implementation for deterministic state representation.
//!
//! This module implements a Merkle tree over key-value pairs using CBOR
//! canonicalization and domain-separated hashing for deterministic state commitments.

use ciborium::value::Value as CborValue;
use std::collections::BTreeMap;

/// Merkle Row-Map: deterministic state representation as Merkle tree.
///
/// This structure maintains a Merkle tree over key-value pairs where:
/// - Keys are row identifiers (chunk IDs, row hashes, etc.)
/// - Values are state values (numbers, strings, bytes, etc.) as CBOR values
/// - Tree root provides deterministic commitment to the entire state
/// - Uses CBOR canonicalization (RFC 8949) for deterministic hashing
#[derive(Debug, Clone)]
pub struct MerkleRowMap {
    entries: BTreeMap<String, CborValue>,
}

impl MerkleRowMap {
    /// Create a new empty Merkle Row-Map.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert a numeric value (convenience method).
    pub fn insert_number(&mut self, key: String, value: f64) -> Option<CborValue> {
        // Convert f64 to CBOR Integer or Float
        let cbor_value = if value.fract() == 0.0 && value >= 0.0 {
            // Integer value (non-negative)
            CborValue::Integer((value as i64).into())
        } else if value.fract() == 0.0 {
            // Negative integer value
            CborValue::Integer((value as i64).into())
        } else {
            // Float value
            CborValue::Float(value)
        };
        self.entries.insert(key, cbor_value)
    }

    /// Get a numeric value (convenience method).
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.entries.get(key).and_then(|v| match v {
            CborValue::Integer(i) => {
                // Extract integer value - ciborium::value::Integer wraps u64/i64
                // Use Debug format to extract, or try direct conversion
                // For now, serialize to bytes and parse back, or use a helper
                let debug_str = format!("{:?}", i);
                // Try to parse from debug string (format: "Integer(42)" or similar)
                debug_str
                    .trim_start_matches("Integer(")
                    .trim_end_matches(")")
                    .parse::<i64>()
                    .ok()
                    .map(|n| n as f64)
            }
            CborValue::Float(f) => Some(*f),
            _ => None,
        })
    }

    /// Insert or update a key-value pair.
    pub fn insert(&mut self, key: String, value: CborValue) -> Option<CborValue> {
        self.entries.insert(key, value)
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&CborValue> {
        self.entries.get(key)
    }

    /// Remove a key-value pair.
    pub fn remove(&mut self, key: &str) -> Option<CborValue> {
        self.entries.remove(key)
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over entries in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CborValue)> {
        self.entries.iter()
    }

    /// Compute the Merkle root of the Row-Map.
    ///
    /// Uses domain-separated hashing with CBOR canonicalization:
    /// - Leaf hash = H("leaf:" || cbor_canonical({k, v}))
    /// - Parent hash = H("node:" || left || right)
    /// - Empty tree root = H("leaf:" || "")
    ///
    /// CBOR canonicalization ensures deterministic encoding per RFC 8949,
    /// providing type fidelity (bytes, big integers, timestamps) and avoiding
    /// JSON precision traps.
    ///
    /// # Returns
    ///
    /// Merkle root in format `sha256:<64hex>`
    pub fn compute_root(&self) -> String {
        use sha2::{Digest, Sha256};
        use northroot_receipts::canonical::encode_canonical;

        if self.entries.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(b"leaf:");
            hasher.update(b"");
            return format!("sha256:{:x}", hasher.finalize());
        }

        // Compute leaf hashes: H("leaf:" || cbor_canonical({k, v}))
        let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
        for (key, value) in &self.entries {
            // Create CBOR map for {k, v}
            // CborValue::Map uses Vec<(CborValue, CborValue)>, not BTreeMap
            let mut entry_map = Vec::new();
            entry_map.push((
                CborValue::Text(key.clone()),
                value.clone(),
            ));
            let entry = CborValue::Map(entry_map);

            // Canonicalize to CBOR bytes
            let cbor_bytes = encode_canonical(&entry)
                .expect("CBOR canonicalization should never fail for valid CBOR values");

            // Leaf hash = H("leaf:" || cbor_bytes)
            let mut hasher = Sha256::new();
            hasher.update(b"leaf:");
            hasher.update(&cbor_bytes);
            leaf_hashes.push(hasher.finalize().into());
        }

        // Build Merkle tree: pair hashes, promote odd nodes
        let mut current_level = leaf_hashes;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            // Pair hashes left-to-right
            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    // Parent = H("node:" || left || right)
                    let mut hasher = Sha256::new();
                    hasher.update(b"node:");
                    hasher.update(&current_level[i]);
                    hasher.update(&current_level[i + 1]);
                    next_level.push(hasher.finalize().into());
                } else {
                    // Odd node: promote upward unchanged
                    next_level.push(current_level[i]);
                }
            }

            current_level = next_level;
        }

        // Root is the final remaining hash
        let root_bytes = current_level[0];
        let hex_str: String = root_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        format!("sha256:{}", hex_str)
    }

    /// Get state hash (alias for compute_root for compatibility).
    pub fn state_hash(&self) -> String {
        self.compute_root()
    }
}

impl Default for MerkleRowMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value as CborValue;

    // Helper to convert JSON-like values to CBOR for testing
    fn json_to_cbor_value(v: i64) -> CborValue {
        CborValue::Integer(v.into())
    }

    fn json_to_cbor_str(s: &str) -> CborValue {
        CborValue::Text(s.to_string())
    }

    #[test]
    fn test_merkle_row_map_empty() {
        let map = MerkleRowMap::new();
        let root = map.compute_root();
        assert!(root.starts_with("sha256:"));
        assert_eq!(root.len(), 71);
    }

    #[test]
    fn test_merkle_row_map_insert_get() {
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), json_to_cbor_value(42));
        map.insert("key2".to_string(), json_to_cbor_str("value"));

        assert_eq!(map.get("key1"), Some(&json_to_cbor_value(42)));
        assert_eq!(map.get("key2"), Some(&json_to_cbor_str("value")));
        assert_eq!(map.get("key3"), None);
    }

    #[test]
    fn test_merkle_row_map_deterministic() {
        let mut map1 = MerkleRowMap::new();
        map1.insert("a".to_string(), json_to_cbor_value(1));
        map1.insert("b".to_string(), json_to_cbor_value(2));
        map1.insert("c".to_string(), json_to_cbor_value(3));

        let mut map2 = MerkleRowMap::new();
        map2.insert("a".to_string(), json_to_cbor_value(1));
        map2.insert("b".to_string(), json_to_cbor_value(2));
        map2.insert("c".to_string(), json_to_cbor_value(3));

        // Same entries should produce same root
        assert_eq!(map1.compute_root(), map2.compute_root());
    }

    #[test]
    fn test_merkle_row_map_order_independent() {
        let mut map1 = MerkleRowMap::new();
        map1.insert("a".to_string(), json_to_cbor_value(1));
        map1.insert("b".to_string(), json_to_cbor_value(2));

        let mut map2 = MerkleRowMap::new();
        map2.insert("b".to_string(), json_to_cbor_value(2));
        map2.insert("a".to_string(), json_to_cbor_value(1));

        // BTreeMap maintains sorted order, so roots should be same
        assert_eq!(map1.compute_root(), map2.compute_root());
    }

    #[test]
    fn test_merkle_row_map_remove() {
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), json_to_cbor_value(42));
        map.insert("key2".to_string(), json_to_cbor_str("value"));

        assert_eq!(map.len(), 2);
        map.remove("key1");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key1"), None);
        assert_eq!(map.get("key2"), Some(&json_to_cbor_str("value")));
    }

    #[test]
    fn test_merkle_row_map_single_entry() {
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), json_to_cbor_value(42));

        let root = map.compute_root();
        assert!(root.starts_with("sha256:"));
        assert_eq!(root.len(), 71);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_merkle_row_map_bytes_support() {
        // Test that CBOR can handle bytes natively (not possible with JSON)
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), CborValue::Bytes(vec![0x01, 0x02, 0x03]));
        map.insert("key2".to_string(), json_to_cbor_value(42));

        let root = map.compute_root();
        assert!(root.starts_with("sha256:"));
        assert_eq!(root.len(), 71);
    }
}
