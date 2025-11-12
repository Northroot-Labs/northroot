//! Property tests for edge cases and boundary conditions.
//!
//! This module tests edge cases that might not be caught by normal unit tests:
//! - Empty inputs
//! - Single item inputs
//! - Boundary conditions (max sizes, overflow)
//! - Malformed inputs
//! - Stress tests (large datasets)

use crate::generators::*;
use northroot_engine::delta::{
    compute_data_shape_hash_from_bytes, compute_method_shape_hash_from_code,
    compute_method_shape_hash_from_signature, jaccard_similarity, weighted_jaccard_similarity,
};
use northroot_engine::shapes::{compute_data_shape_hash, ChunkScheme, DataShape, KeyFormat, RowValueRepr};
use proptest::prelude::*;
use std::collections::HashSet;

/// Edge case: Empty data bytes.
/// Empty input should produce valid hash
#[test]
fn prop_edge_empty_bytes() {
    proptest!(|(chunk_scheme_opt in prop::option::of(chunk_scheme_strategy()))| {
        let data = vec![];
        let result = compute_data_shape_hash_from_bytes(&data, chunk_scheme_opt);
        prop_assert!(result.is_ok(), "Empty bytes should produce valid hash");
        let hash = result.unwrap();
        prop_assert!(hash.starts_with("sha256:"), "Hash must have sha256: prefix");
        prop_assert_eq!(hash.len(), 71, "Hash must be 71 chars");
    });
}

/// Edge case: Single byte.
#[test]
fn prop_edge_single_byte() {
    proptest!(|(chunk_scheme_opt in prop::option::of(chunk_scheme_strategy()))| {
        let data = vec![0u8];
        let result = compute_data_shape_hash_from_bytes(&data, chunk_scheme_opt);
        prop_assert!(result.is_ok(), "Single byte should produce valid hash");
    });
}

/// Edge case: Very large data (stress test).
/// Test with large byte arrays
#[test]
fn prop_edge_large_data() {
    proptest!(|(chunk_scheme_opt in prop::option::of(chunk_scheme_strategy()))| {
        // Generate large data (up to 1MB)
        let data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
        let result = compute_data_shape_hash_from_bytes(&data, chunk_scheme_opt);
        prop_assert!(result.is_ok(), "Large data should produce valid hash");
        let hash = result.unwrap();
        prop_assert!(hash.starts_with("sha256:"), "Hash must have sha256: prefix");
    });
}

/// Edge case: Empty chunk set.
/// Jaccard similarity of empty sets should be 1.0 (both empty)
#[test]
fn prop_edge_empty_chunk_sets() {
    let set1: HashSet<String> = HashSet::new();
    let set2: HashSet<String> = HashSet::new();
    let j = jaccard_similarity(&set1, &set2);
    assert!((j - 1.0).abs() < 1e-10, "Empty sets should have Jaccard = 1.0");
}

/// Edge case: One empty set.
/// Jaccard similarity with one empty set should be 0.0
#[test]
fn prop_edge_one_empty_chunk_set() {
    proptest!(|(set in chunk_set_strategy())| {
        let empty: HashSet<String> = HashSet::new();
        let j1 = jaccard_similarity(&set, &empty);
        let j2 = jaccard_similarity(&empty, &set);
        prop_assert_eq!(j1, j2, "Jaccard should be symmetric");
        if set.is_empty() {
            prop_assert!((j1 - 1.0).abs() < 1e-10, "Both empty should have Jaccard = 1.0");
        } else {
            prop_assert!((j1 - 0.0).abs() < 1e-10, "One empty set should have Jaccard = 0.0");
        }
    });
}

/// Edge case: Single chunk in set.
#[test]
fn prop_edge_single_chunk() {
    let mut set1 = HashSet::new();
    set1.insert("sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string());
    let mut set2 = HashSet::new();
    set2.insert("sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string());
    
    let j = jaccard_similarity(&set1, &set2);
    assert!((j - 1.0).abs() < 1e-10, "Identical single chunks should have Jaccard = 1.0");
}

/// Edge case: Very large chunk sets (stress test).
#[test]
fn prop_edge_large_chunk_sets() {
    // Generate large sets (up to 1000 chunks)
    let set1: HashSet<String> = (0..1000)
        .map(|i| format!("sha256:{:064x}", i))
        .collect();
    let set2: HashSet<String> = (500..1500)
        .map(|i| format!("sha256:{:064x}", i))
        .collect();
    
    let j = jaccard_similarity(&set1, &set2);
    assert!(j >= 0.0 && j <= 1.0, "Jaccard must be in [0, 1]");
    // Should have overlap (500-999)
    assert!(j > 0.0, "Large overlapping sets should have Jaccard > 0");
}

/// Edge case: DataShape with zero manifest_len.
#[test]
fn prop_edge_zero_manifest_len() {
    proptest!(|(manifest_root in hash_strategy(), chunk_scheme in chunk_scheme_strategy())| {
        let shape = DataShape::ByteStream {
            manifest_root,
            manifest_len: 0,
            chunk_scheme,
        };
        let result = compute_data_shape_hash(&shape);
        prop_assert!(result.is_ok(), "Zero manifest_len should produce valid hash");
    });
}

/// Edge case: DataShape with maximum manifest_len.
#[test]
fn prop_edge_max_manifest_len() {
    proptest!(|(manifest_root in hash_strategy(), chunk_scheme in chunk_scheme_strategy())| {
        let shape = DataShape::ByteStream {
            manifest_root,
            manifest_len: u64::MAX,
            chunk_scheme,
        };
        let result = compute_data_shape_hash(&shape);
        prop_assert!(result.is_ok(), "Max manifest_len should produce valid hash");
    });
}

/// Edge case: DataShape with zero row_count.
#[test]
fn prop_edge_zero_row_count() {
    proptest!(|(merkle_root in hash_strategy())| {
        let shape = DataShape::RowMap {
            merkle_root,
            row_count: 0,
            key_fmt: KeyFormat::Sha256Hex,
            value_repr: RowValueRepr::Number,
        };
        let result = compute_data_shape_hash(&shape);
        prop_assert!(result.is_ok(), "Zero row_count should produce valid hash");
    });
}

/// Edge case: Method shape hash with empty function name (should error).
#[test]
fn prop_edge_empty_function_name() {
    proptest!(|(input_types in type_names_strategy(), output_type in type_name_strategy())| {
        let result = compute_method_shape_hash_from_signature(
            "",
            &input_types.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &output_type
        );
        prop_assert!(result.is_err(), "Empty function name should error");
    });
}

/// Edge case: Method shape hash with empty input types.
#[test]
fn prop_edge_empty_input_types() {
    proptest!(|(function_name in function_name_strategy(), output_type in type_name_strategy())| {
        let result = compute_method_shape_hash_from_signature(
            &function_name,
            &[],
            &output_type
        );
        prop_assert!(result.is_ok(), "Empty input types should be valid");
        let hash = result.unwrap();
        prop_assert!(hash.starts_with("sha256:"), "Hash must have sha256: prefix");
    });
}

/// Edge case: Weighted Jaccard with zero weights.
#[test]
fn prop_edge_weighted_jaccard_zero_weights() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        let weights = |_chunk: &String| -> f64 { 0.0 };
        let j = weighted_jaccard_similarity(&set1, &set2, weights);
        // If all weights are zero, union_weight is 0, so result should be 0.0
        if set1.is_empty() && set2.is_empty() {
            // Both empty: should handle gracefully
            prop_assert!(j >= 0.0 && j <= 1.0, "Jaccard must be in [0, 1]");
        } else {
            prop_assert_eq!(j, 0.0, "Zero weights should produce 0.0 (or handle division by zero)");
        }
    });
}

/// Edge case: Weighted Jaccard with very large weights.
#[test]
fn prop_edge_weighted_jaccard_large_weights() {
    proptest!(|(set1 in chunk_set_strategy(), set2 in chunk_set_strategy())| {
        let weights = |_chunk: &String| -> f64 { 1e100 };
        let j = weighted_jaccard_similarity(&set1, &set2, weights);
        prop_assert!(j >= 0.0 && j <= 1.0, "Jaccard must be in [0, 1] even with large weights");
        prop_assert!(j.is_finite(), "Jaccard must be finite");
    });
}

/// Edge case: Chunk scheme with very small sizes.
#[test]
fn prop_edge_small_chunk_sizes() {
    let data = vec![0u8; 1000];
    let small_scheme = ChunkScheme::Fixed { size: 1 };
    let result = compute_data_shape_hash_from_bytes(&data, Some(small_scheme));
    assert!(result.is_ok(), "Small chunk size should work");
}

/// Edge case: Chunk scheme with very large sizes.
#[test]
fn prop_edge_large_chunk_sizes() {
    let data = vec![0u8; 1000];
    let large_scheme = ChunkScheme::Fixed { size: 1_000_000 };
    let result = compute_data_shape_hash_from_bytes(&data, Some(large_scheme));
    assert!(result.is_ok(), "Large chunk size should work");
}

/// Edge case: CDC with very small avg_size.
#[test]
fn prop_edge_small_cdc_avg_size() {
    let data = vec![0u8; 1000];
    let small_cdc = ChunkScheme::CDC { avg_size: 1 };
    let result = compute_data_shape_hash_from_bytes(&data, Some(small_cdc));
    assert!(result.is_ok(), "Small CDC avg_size should work");
}

/// Edge case: CDC with very large avg_size.
#[test]
fn prop_edge_large_cdc_avg_size() {
    let data = vec![0u8; 1000];
    let large_cdc = ChunkScheme::CDC { avg_size: 1_000_000 };
    let result = compute_data_shape_hash_from_bytes(&data, Some(large_cdc));
    assert!(result.is_ok(), "Large CDC avg_size should work");
}
