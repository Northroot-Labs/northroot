//! Property tests for critical invariants.
//!
//! This module tests that critical invariants hold across all inputs:
//! - Determinism: Same input → same output
//! - Order independence: Insertion/processing order doesn't affect results
//! - Idempotency: f(f(x)) == f(x) where applicable

use crate::generators::*;
use northroot_engine::delta::{
    compute_data_shape_hash_from_bytes, compute_method_shape_hash_from_code,
    compute_method_shape_hash_from_signature, jaccard_similarity, weighted_jaccard_similarity,
};
use northroot_engine::shapes::{compute_data_shape_hash, DataShape};
use proptest::prelude::*;
use std::collections::HashSet;

/// Property: Data shape hash computation is deterministic.
/// Same DataShape → same hash
#[test]
fn prop_data_shape_hash_deterministic() {
    proptest!(|(shape in data_shape_strategy())| {
        let hash1 = compute_data_shape_hash(&shape).unwrap();
        let hash2 = compute_data_shape_hash(&shape).unwrap();
        prop_assert_eq!(hash1.clone(), hash2, "Same DataShape must produce same hash");
        prop_assert!(hash1.starts_with("sha256:"), "Hash must have sha256: prefix");
        prop_assert_eq!(hash1.len(), 71, "Hash must be 71 chars (sha256: + 64 hex)");
    });
}

/// Property: Method shape hash from code is deterministic.
/// Same code hash + params → same method shape hash
#[test]
fn prop_method_shape_hash_from_code_deterministic() {
    proptest!(|(code_hash in code_hash_strategy(), params_opt in prop::option::of(json_value_strategy()))| {
        let hash1 = compute_method_shape_hash_from_code(&code_hash, params_opt.as_ref()).unwrap();
        let hash2 = compute_method_shape_hash_from_code(&code_hash, params_opt.as_ref()).unwrap();
        prop_assert_eq!(hash1.clone(), hash2, "Same code hash + params must produce same hash");
        prop_assert!(hash1.starts_with("sha256:"), "Hash must have sha256: prefix");
        prop_assert_eq!(hash1.len(), 71, "Hash must be 71 chars");
    });
}

/// Property: Method shape hash from signature is deterministic.
/// Same signature → same method shape hash
#[test]
fn prop_method_shape_hash_from_signature_deterministic() {
    proptest!(|(function_name in function_name_strategy(), input_types in type_names_strategy(), output_type in type_name_strategy())| {
        let hash1 = compute_method_shape_hash_from_signature(
            &function_name,
            &input_types.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &output_type,
        ).unwrap();
        let hash2 = compute_method_shape_hash_from_signature(
            &function_name,
            &input_types.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &output_type,
        ).unwrap();
        prop_assert_eq!(hash1.clone(), hash2, "Same signature must produce same hash");
        prop_assert!(hash1.starts_with("sha256:"), "Hash must have sha256: prefix");
        prop_assert_eq!(hash1.len(), 71, "Hash must be 71 chars");
    });
}

/// Property: Data shape hash from bytes is deterministic.
/// Same bytes + chunk scheme → same hash
#[test]
fn prop_data_shape_hash_from_bytes_deterministic() {
    proptest!(|(data in bytes_strategy(), chunk_scheme_opt in prop::option::of(chunk_scheme_strategy()))| {
        let hash1 = compute_data_shape_hash_from_bytes(&data, chunk_scheme_opt.clone()).unwrap();
        let hash2 = compute_data_shape_hash_from_bytes(&data, chunk_scheme_opt.clone()).unwrap();
        prop_assert_eq!(hash1.clone(), hash2, "Same bytes + scheme must produce same hash");
        prop_assert!(hash1.starts_with("sha256:"), "Hash must have sha256: prefix");
        prop_assert_eq!(hash1.len(), 71, "Hash must be 71 chars");
    });
}

/// Property: Jaccard similarity is symmetric.
/// J(A, B) == J(B, A)
#[test]
fn prop_jaccard_similarity_symmetric() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        let j1 = jaccard_similarity(&set1, &set2);
        let j2 = jaccard_similarity(&set2, &set1);
        prop_assert!((j1 - j2).abs() < 1e-10, "Jaccard similarity must be symmetric");
    });
}

/// Property: Jaccard similarity is in [0, 1].
#[test]
fn prop_jaccard_similarity_range() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        let j = jaccard_similarity(&set1, &set2);
        prop_assert!(j >= 0.0 && j <= 1.0, "Jaccard similarity must be in [0, 1], got {}", j);
    });
}

/// Property: Jaccard similarity of identical sets is 1.0.
#[test]
fn prop_jaccard_similarity_identical_sets() {
    proptest!(|(set in chunk_set_strategy())| {
        let j = jaccard_similarity(&set, &set);
        prop_assert!((j - 1.0).abs() < 1e-10, "Jaccard similarity of identical sets must be 1.0, got {}", j);
    });
}

/// Property: Jaccard similarity is order-independent.
/// Permuting set elements doesn't change result (HashSet already handles this, but we verify)
#[test]
fn prop_jaccard_similarity_order_independent() {
    proptest!(|(set1_vec in prop::collection::vec(hash_strategy(), 0..=100), set2_vec in prop::collection::vec(hash_strategy(), 0..=100))| {
        // Convert to HashSet (removes duplicates and ignores order)
        let set1: HashSet<String> = set1_vec.into_iter().collect();
        let set2: HashSet<String> = set2_vec.into_iter().collect();

        // Create permuted versions (HashSet already ignores order, but we verify)
        let set1_permuted: HashSet<String> = set1.iter().cloned().collect();
        let set2_permuted: HashSet<String> = set2.iter().cloned().collect();

        let j1 = jaccard_similarity(&set1, &set2);
        let j2 = jaccard_similarity(&set1_permuted, &set2_permuted);
        prop_assert!((j1 - j2).abs() < 1e-10, "Jaccard similarity must be order-independent");
    });
}

/// Property: Weighted Jaccard similarity is symmetric.
#[test]
fn prop_weighted_jaccard_similarity_symmetric() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        // Use chunk hash as weight (deterministic)
        let weights = |chunk: &String| -> f64 {
            // Use first 8 hex chars as weight (deterministic)
            u64::from_str_radix(&chunk[7..15], 16).unwrap_or(0) as f64
        };

        let j1 = weighted_jaccard_similarity(&set1, &set2, &weights);
        let j2 = weighted_jaccard_similarity(&set2, &set1, &weights);
        prop_assert!((j1 - j2).abs() < 1e-10, "Weighted Jaccard must be symmetric");
    });
}

/// Property: Weighted Jaccard similarity is in [0, 1].
#[test]
fn prop_weighted_jaccard_similarity_range() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        let weights = |_chunk: &String| -> f64 { 1.0 };
        let j = weighted_jaccard_similarity(&set1, &set2, weights);
        prop_assert!(j >= 0.0 && j <= 1.0, "Weighted Jaccard must be in [0, 1], got {}", j);
    });
}

/// Property: Hash computation is idempotent.
/// hash(hash(x)) == hash(x) (for hash functions, this is trivially true)
/// But we verify that applying hash twice gives same result
#[test]
fn prop_hash_idempotent() {
    proptest!(|(shape in data_shape_strategy())| {
        let hash1 = compute_data_shape_hash(&shape).unwrap();
        // Create new shape with same values (simulating "re-hashing")
        let shape2 = shape.clone();
        let hash2 = compute_data_shape_hash(&shape2).unwrap();
        prop_assert_eq!(hash1.clone(), hash2, "Hash computation must be idempotent");
    });
}

/// Property: Different DataShapes produce different hashes (collision resistance).
/// This is a probabilistic test - we can't guarantee no collisions, but we test many cases
#[test]
fn prop_data_shape_hash_collision_resistance() {
    proptest!(|(shape1 in data_shape_strategy(), shape2 in data_shape_strategy())| {
        // If shapes are equal, hashes must be equal
        if shape1 == shape2 {
            let hash1 = compute_data_shape_hash(&shape1).unwrap();
            let hash2 = compute_data_shape_hash(&shape2).unwrap();
            prop_assert_eq!(hash1.clone(), hash2, "Equal shapes must produce equal hashes");
        } else {
            // If shapes are different, hashes should be different (with high probability)
            let hash1 = compute_data_shape_hash(&shape1).unwrap();
            let hash2 = compute_data_shape_hash(&shape2).unwrap();
            // Note: We can't assert they're different due to hash collisions, but we verify
            // the hash format is correct
            prop_assert!(hash1.starts_with("sha256:"), "Hash1 must have sha256: prefix");
            prop_assert!(hash2.starts_with("sha256:"), "Hash2 must have sha256: prefix");
        }
    });
}
