//! Proof-Addressable Cache (PAC) key computation.
//!
//! This module provides functions for computing deterministic, content-addressed
//! keys that uniquely identify reusable computations.

use sha2::{Digest, Sha256};

/// Compute Proof-Addressable Cache (PAC) key from execution parameters.
///
/// PAC keys uniquely identify reusable computations based on:
/// - Namespace and version
/// - Data shape hash
/// - Method shape hash
/// - Change epoch ID (snapshot/commit)
/// - Determinism class
/// - Policy reference
/// - Output schema version
///
/// The key is computed as SHA-256 hash of a deterministic string format:
/// `ns|version||data_shape||method_shape||epoch||determinism||policy||schema`
///
/// # Arguments
///
/// * `namespace` - Namespace identifier (e.g., "northroot")
/// * `version` - Version string (e.g., "0.3.0")
/// * `data_shape_hash` - SHA-256 hash of data shape (format: "sha256:...")
/// * `method_shape_hash` - SHA-256 hash of method shape (format: "sha256:...")
/// * `change_epoch_id` - Change epoch identifier (e.g., "snap-123", "commit-abc", or "none")
/// * `determinism_class` - Determinism class (e.g., "strict", "bounded", "observational")
/// * `policy_ref` - Policy reference (e.g., "pol:finops/cost-attribution@1", or "none")
/// * `output_schema_version` - Output schema version (e.g., "1.0")
///
/// # Returns
///
/// 32-byte binary key for deterministic lookup
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::pac::compute_pac_key;
///
/// let pac = compute_pac_key(
///     "northroot",
///     "0.3.0",
///     "sha256:abcd1234...",
///     "sha256:efgh5678...",
///     "snap-123",
///     "strict",
///     "pol:finops/cost-attribution@1",
///     "1.0"
/// );
/// ```
pub fn compute_pac_key(
    namespace: &str,
    version: &str,
    data_shape_hash: &str,
    method_shape_hash: &str,
    change_epoch_id: &str,
    determinism_class: &str,
    policy_ref: &str,
    output_schema_version: &str,
) -> [u8; 32] {
    // Format: ns|version||data_shape||method_shape||epoch||determinism||policy||schema
    // Using || as separator between major components, | for sub-components
    let combined = format!(
        "{}|{}||{}||{}||{}||{}||{}||{}",
        namespace,
        version,
        data_shape_hash,
        method_shape_hash,
        change_epoch_id,
        determinism_class,
        policy_ref,
        output_schema_version
    );
    let hash = Sha256::digest(combined.as_bytes());
    hash.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pac_key_determinism() {
        let pac1 = compute_pac_key(
            "northroot",
            "0.3.0",
            "sha256:abcd1234",
            "sha256:efgh5678",
            "snap-123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        let pac2 = compute_pac_key(
            "northroot",
            "0.3.0",
            "sha256:abcd1234",
            "sha256:efgh5678",
            "snap-123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        assert_eq!(pac1, pac2, "Same inputs should produce identical PAC keys");
    }

    #[test]
    fn test_pac_key_uniqueness() {
        let pac1 = compute_pac_key(
            "northroot",
            "0.3.0",
            "sha256:abcd1234",
            "sha256:efgh5678",
            "snap-123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        let pac2 = compute_pac_key(
            "northroot",
            "0.3.0",
            "sha256:abcd1235", // Different data shape
            "sha256:efgh5678",
            "snap-123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        assert_ne!(
            pac1, pac2,
            "Different inputs should produce different PAC keys"
        );
    }

    #[test]
    fn test_pac_key_empty_strings() {
        let pac1 = compute_pac_key("", "", "", "", "", "", "", "");
        let pac2 = compute_pac_key("", "", "", "", "", "", "", "");
        assert_eq!(pac1, pac2, "Empty strings should be handled consistently");
    }

    #[test]
    fn test_pac_key_special_characters() {
        let pac1 = compute_pac_key(
            "ns|with|pipes",
            "version:1.0",
            "sha256:hash||with||separators",
            "sha256:method",
            "epoch|123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        // Should not panic and should be deterministic
        let pac2 = compute_pac_key(
            "ns|with|pipes",
            "version:1.0",
            "sha256:hash||with||separators",
            "sha256:method",
            "epoch|123",
            "strict",
            "pol:test@1",
            "1.0",
        );
        assert_eq!(pac1, pac2, "Special characters should be handled correctly");
    }

    #[test]
    fn test_pac_key_all_different() {
        let pacs: Vec<[u8; 32]> = vec![
            compute_pac_key("ns1", "v1", "h1", "m1", "e1", "d1", "p1", "s1"),
            compute_pac_key("ns2", "v1", "h1", "m1", "e1", "d1", "p1", "s1"),
            compute_pac_key("ns1", "v2", "h1", "m1", "e1", "d1", "p1", "s1"),
            compute_pac_key("ns1", "v1", "h2", "m1", "e1", "d1", "p1", "s1"),
            compute_pac_key("ns1", "v1", "h1", "m2", "e1", "d1", "p1", "s1"),
            compute_pac_key("ns1", "v1", "h1", "m1", "e2", "d1", "p1", "s1"),
            compute_pac_key("ns1", "v1", "h1", "m1", "e1", "d2", "p1", "s1"),
            compute_pac_key("ns1", "v1", "h1", "m1", "e1", "d1", "p2", "s1"),
            compute_pac_key("ns1", "v1", "h1", "m1", "e1", "d1", "p1", "s2"),
        ];

        // All should be unique
        for i in 0..pacs.len() {
            for j in (i + 1)..pacs.len() {
                assert_ne!(
                    pacs[i], pacs[j],
                    "PAC keys should be unique when any parameter differs"
                );
            }
        }
    }
}
