//! Merkle Row-Map implementation for deterministic state representation.
//!
//! This module implements a Merkle tree over key-value pairs using RFC-6962
//! style domain separation for deterministic state commitments.

use serde_json::Value;
use std::collections::BTreeMap;

/// Merkle Row-Map: deterministic state representation as Merkle tree.
///
/// This structure maintains a Merkle tree over key-value pairs where:
/// - Keys are row identifiers (chunk IDs, row hashes, etc.)
/// - Values are state values (numbers, strings, etc.)
/// - Tree root provides deterministic commitment to the entire state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleRowMap {
    entries: BTreeMap<String, Value>,
}

impl MerkleRowMap {
    /// Create a new empty Merkle Row-Map.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert or update a key-value pair.
    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.entries.insert(key, value)
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key)
    }

    /// Remove a key-value pair.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
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
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter()
    }

    /// Compute the Merkle root of the Row-Map.
    ///
    /// Uses RFC-6962 style domain separation:
    /// - Leaf hash = H(0x00 || JCS({k, v}))
    /// - Parent hash = H(0x01 || left || right)
    /// - Empty tree root = H(0x00 || "")
    ///
    /// # Returns
    ///
    /// Merkle root in format `sha256:<64hex>`
    pub fn compute_root(&self) -> String {
        use sha2::{Digest, Sha256};

        if self.entries.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(&[0x00u8]);
            hasher.update(b"");
            return format!("sha256:{:x}", hasher.finalize());
        }

        // Compute leaf hashes: H(0x00 || JCS({k, v}))
        let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
        for (key, value) in &self.entries {
            // Create canonical JSON for {k, v}
            let entry = serde_json::json!({
                "k": key,
                "v": value
            });
            let canonical = crate::commitments::jcs(&entry);

            // Leaf hash = H(0x00 || canonical_bytes)
            let mut hasher = Sha256::new();
            hasher.update(&[0x00u8]);
            hasher.update(canonical.as_bytes());
            leaf_hashes.push(hasher.finalize().into());
        }

        // Build Merkle tree: pair hashes, promote odd nodes
        let mut current_level = leaf_hashes;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            // Pair hashes left-to-right
            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    // Parent = H(0x01 || left || right)
                    let mut hasher = Sha256::new();
                    hasher.update(&[0x01u8]);
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
        map.insert("key1".to_string(), serde_json::json!(42));
        map.insert("key2".to_string(), serde_json::json!("value"));

        assert_eq!(map.get("key1"), Some(&serde_json::json!(42)));
        assert_eq!(map.get("key2"), Some(&serde_json::json!("value")));
        assert_eq!(map.get("key3"), None);
    }

    #[test]
    fn test_merkle_row_map_deterministic() {
        let mut map1 = MerkleRowMap::new();
        map1.insert("a".to_string(), serde_json::json!(1));
        map1.insert("b".to_string(), serde_json::json!(2));
        map1.insert("c".to_string(), serde_json::json!(3));

        let mut map2 = MerkleRowMap::new();
        map2.insert("a".to_string(), serde_json::json!(1));
        map2.insert("b".to_string(), serde_json::json!(2));
        map2.insert("c".to_string(), serde_json::json!(3));

        // Same entries should produce same root
        assert_eq!(map1.compute_root(), map2.compute_root());
    }

    #[test]
    fn test_merkle_row_map_order_independent() {
        let mut map1 = MerkleRowMap::new();
        map1.insert("a".to_string(), serde_json::json!(1));
        map1.insert("b".to_string(), serde_json::json!(2));

        let mut map2 = MerkleRowMap::new();
        map2.insert("b".to_string(), serde_json::json!(2));
        map2.insert("a".to_string(), serde_json::json!(1));

        // BTreeMap maintains sorted order, so roots should be same
        assert_eq!(map1.compute_root(), map2.compute_root());
    }

    #[test]
    fn test_merkle_row_map_remove() {
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), serde_json::json!(42));
        map.insert("key2".to_string(), serde_json::json!("value"));

        assert_eq!(map.len(), 2);
        map.remove("key1");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key1"), None);
        assert_eq!(map.get("key2"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_merkle_row_map_single_entry() {
        let mut map = MerkleRowMap::new();
        map.insert("key1".to_string(), serde_json::json!(42));
        
        let root = map.compute_root();
        assert!(root.starts_with("sha256:"));
        assert_eq!(root.len(), 71);
        assert_eq!(map.len(), 1);
    }
}

