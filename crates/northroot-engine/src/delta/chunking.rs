//! Chunking utilities for stable chunk ID generation.
//!
//! This module provides functions for generating stable, deterministic chunk IDs
//! and managing chunk sets for delta compute operations.

use crate::commitments::sha256_prefixed;
use std::collections::HashSet;

/// Generate a stable chunk ID from content bytes.
///
/// Chunk IDs are computed as SHA-256 hash of the content, formatted as
/// `sha256:<64hex>`. This ensures deterministic, content-addressable chunking.
///
/// # Arguments
///
/// * `content` - Chunk content bytes
///
/// # Returns
///
/// Chunk ID in format `sha256:<64hex>`
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::chunk_id_from_bytes;
///
/// let content = b"hello world";
/// let chunk_id = chunk_id_from_bytes(content);
/// // Returns: "sha256:..."
/// ```
pub fn chunk_id_from_bytes(content: &[u8]) -> String {
    sha256_prefixed(content)
}

/// Generate a chunk ID from a string.
///
/// Convenience function that converts string to bytes and generates chunk ID.
///
/// # Arguments
///
/// * `content` - Chunk content as string
///
/// # Returns
///
/// Chunk ID in format `sha256:<64hex>`
pub fn chunk_id_from_str(content: &str) -> String {
    chunk_id_from_bytes(content.as_bytes())
}

/// Chunk set manager for tracking chunk collections.
///
/// This struct provides utilities for managing sets of chunks and computing
/// operations like union, intersection, and difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSet {
    chunks: HashSet<String>,
}

impl ChunkSet {
    /// Create a new empty chunk set.
    pub fn new() -> Self {
        Self {
            chunks: HashSet::new(),
        }
    }

    /// Create a chunk set from an iterator of chunk IDs.
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        Self {
            chunks: iter.into_iter().collect(),
        }
    }

    /// Add a chunk ID to the set.
    pub fn insert(&mut self, chunk_id: String) -> bool {
        self.chunks.insert(chunk_id)
    }

    /// Remove a chunk ID from the set.
    pub fn remove(&mut self, chunk_id: &str) -> bool {
        self.chunks.remove(chunk_id)
    }

    /// Check if a chunk ID is in the set.
    pub fn contains(&self, chunk_id: &str) -> bool {
        self.chunks.contains(chunk_id)
    }

    /// Get the number of chunks in the set.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get all chunk IDs as a set.
    pub fn chunks(&self) -> &HashSet<String> {
        &self.chunks
    }

    /// Compute intersection with another chunk set.
    pub fn intersection(&self, other: &ChunkSet) -> ChunkSet {
        ChunkSet {
            chunks: self.chunks.intersection(&other.chunks).cloned().collect(),
        }
    }

    /// Compute union with another chunk set.
    pub fn union(&self, other: &ChunkSet) -> ChunkSet {
        ChunkSet {
            chunks: self.chunks.union(&other.chunks).cloned().collect(),
        }
    }

    /// Compute difference (chunks in self but not in other).
    pub fn difference(&self, other: &ChunkSet) -> ChunkSet {
        ChunkSet {
            chunks: self.chunks.difference(&other.chunks).cloned().collect(),
        }
    }
}

impl Default for ChunkSet {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashSet<String>> for ChunkSet {
    fn from(chunks: HashSet<String>) -> Self {
        Self { chunks }
    }
}

impl From<ChunkSet> for HashSet<String> {
    fn from(chunk_set: ChunkSet) -> Self {
        chunk_set.chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_id_from_bytes() {
        let content = b"hello world";
        let chunk_id = chunk_id_from_bytes(content);
        assert!(chunk_id.starts_with("sha256:"));
        assert_eq!(chunk_id.len(), 71);
    }

    #[test]
    fn test_chunk_id_deterministic() {
        let content = b"test content";
        let id1 = chunk_id_from_bytes(content);
        let id2 = chunk_id_from_bytes(content);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_chunk_set_operations() {
        let mut set1 = ChunkSet::new();
        set1.insert("chunk1".to_string());
        set1.insert("chunk2".to_string());
        set1.insert("chunk3".to_string());

        let mut set2 = ChunkSet::new();
        set2.insert("chunk2".to_string());
        set2.insert("chunk3".to_string());
        set2.insert("chunk4".to_string());

        let intersection = set1.intersection(&set2);
        assert_eq!(intersection.len(), 2);
        assert!(intersection.contains("chunk2"));
        assert!(intersection.contains("chunk3"));

        let union = set1.union(&set2);
        assert_eq!(union.len(), 4);

        let difference = set1.difference(&set2);
        assert_eq!(difference.len(), 1);
        assert!(difference.contains("chunk1"));
    }

    #[test]
    fn test_chunk_set_from_iter() {
        let chunks = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let set = ChunkSet::from_iter(chunks);
        assert_eq!(set.len(), 3);
    }
}
