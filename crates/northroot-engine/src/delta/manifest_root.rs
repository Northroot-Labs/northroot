//! Manifest root computation for multi-part outputs.
//!
//! This module provides functions for computing RFC-6962 Merkle roots over
//! output subparts (multi-file/partition outputs), enabling partial reuse proofs.
//!
//! This is part of ADR-0009-P07: Reuse Reconciliation Flow (deferred from P4).

use sha2::{Digest, Sha256};

/// Compute manifest root for multi-part outputs.
///
/// This function computes an RFC-6962 Merkle root over multiple output parts
/// (files, partitions, etc.), enabling partial reuse proofs when output
/// consists of multiple files/partitions.
///
/// The root is computed using a binary Merkle tree with RFC-6962 domain separation:
/// - Leaf nodes: `0x00 || part_hash`
/// - Internal nodes: `0x01 || left_hash || right_hash`
///
/// # Arguments
///
/// * `part_hashes` - Iterator of part hashes (SHA-256 hashes as 32-byte arrays)
///
/// # Returns
///
/// Merkle root as 32-byte array
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_manifest_root;
///
/// let parts = vec![
///     [0u8; 32], // Part 1 hash
///     [1u8; 32], // Part 2 hash
///     [2u8; 32], // Part 3 hash
/// ];
/// let root = compute_manifest_root(parts.iter());
/// ```
pub fn compute_manifest_root<I>(part_hashes: I) -> [u8; 32]
where
    I: Iterator<Item = [u8; 32]>,
{
    let parts: Vec<[u8; 32]> = part_hashes.collect();

    if parts.is_empty() {
        // Empty manifest - return zero hash
        return [0u8; 32];
    }

    if parts.len() == 1 {
        // Single part - return leaf hash
        return hash_leaf(&parts[0]);
    }

    // Build binary Merkle tree
    let mut level: Vec<[u8; 32]> = parts.iter().map(hash_leaf).collect();

    while level.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs
        for i in (0..level.len()).step_by(2) {
            if i + 1 < level.len() {
                // Pair: hash together
                next_level.push(hash_node(&level[i], &level[i + 1]));
            } else {
                // Odd one out: promote to next level
                next_level.push(level[i]);
            }
        }

        level = next_level;
    }

    level[0]
}

/// Hash a leaf node (part hash).
///
/// Format: `0x00 || part_hash`
fn hash_leaf(part_hash: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00u8]); // Leaf prefix (RFC-6962)
    hasher.update(part_hash);
    hasher.finalize().into()
}

/// Hash an internal node (two child hashes).
///
/// Format: `0x01 || left_hash || right_hash`
fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01u8]); // Node prefix (RFC-6962)
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Compute manifest root from part hashes as strings.
///
/// Convenience function that parses hex-encoded hash strings.
///
/// # Arguments
///
/// * `part_hash_strings` - Iterator of part hashes as hex strings (with or without "sha256:" prefix)
///
/// # Returns
///
/// Merkle root as 32-byte array
///
/// # Errors
///
/// Returns error if any hash string is invalid
pub fn compute_manifest_root_from_strings<I, S>(
    part_hash_strings: I,
) -> Result<[u8; 32], ManifestRootError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut parts = Vec::new();

    for hash_str in part_hash_strings {
        let hash_str = hash_str.as_ref();
        // Remove "sha256:" prefix if present
        let hash_hex = hash_str.strip_prefix("sha256:").unwrap_or(hash_str);

        // Parse hex to bytes
        let hash_bytes = hex::decode(hash_hex)
            .map_err(|e| ManifestRootError::InvalidHash(format!("Failed to decode hex: {}", e)))?;

        if hash_bytes.len() != 32 {
            return Err(ManifestRootError::InvalidHash(format!(
                "Hash must be 32 bytes, got {}",
                hash_bytes.len()
            )));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        parts.push(hash);
    }

    Ok(compute_manifest_root(parts.into_iter()))
}

/// Errors for manifest root computation.
#[derive(Debug, thiserror::Error)]
pub enum ManifestRootError {
    #[error("Invalid hash: {0}")]
    InvalidHash(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_root_empty() {
        let root = compute_manifest_root([].iter().cloned());
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn test_manifest_root_single() {
        let part = [1u8; 32];
        let root = compute_manifest_root([part].iter().cloned());
        // Should be hash of 0x00 || part
        assert_ne!(root, [0u8; 32]);
        assert_ne!(root, part);
    }

    #[test]
    fn test_manifest_root_two_parts() {
        let part1 = [1u8; 32];
        let part2 = [2u8; 32];
        let root = compute_manifest_root([part1, part2].iter().cloned());

        // Should be hash of 0x01 || hash_leaf(part1) || hash_leaf(part2)
        assert_ne!(root, [0u8; 32]);
        assert_ne!(root, part1);
        assert_ne!(root, part2);
    }

    #[test]
    fn test_manifest_root_three_parts() {
        let parts = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = compute_manifest_root(parts.iter().cloned());
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_manifest_root_deterministic() {
        let parts = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root1 = compute_manifest_root(parts.iter().cloned());
        let root2 = compute_manifest_root(parts.iter().cloned());
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_manifest_root_from_strings() {
        let hashes =
            vec!["sha256:0101010101010101010101010101010101010101010101010101010101010101"];
        let root = compute_manifest_root_from_strings(hashes.iter()).unwrap();
        assert_ne!(root, [0u8; 32]);
    }
}
