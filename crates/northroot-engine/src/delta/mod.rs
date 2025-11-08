//! Delta compute operations for incremental recomputation.
//!
//! This module provides utilities for computing overlap between chunk sets,
//! making reuse decisions, and managing chunking strategies.

pub mod chunking;
pub mod decision;
pub mod overlap;

pub use chunking::*;
pub use decision::*;
pub use overlap::*;

use crate::commitments::sha256_prefixed;

/// Overlap metric representing Jaccard similarity and related statistics.
///
/// This struct captures the overlap between current and previous chunk sets,
/// enabling reuse decisions and economic analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlapMetric {
    /// Jaccard similarity [0,1] between current and previous chunk sets
    pub jaccard: f64,
    /// Number of chunks in current set
    pub chunk_count_current: usize,
    /// Number of chunks in previous set
    pub chunk_count_previous: usize,
    /// Number of chunks in intersection
    pub chunk_count_intersection: usize,
}

impl OverlapMetric {
    /// Create a new overlap metric.
    ///
    /// # Arguments
    ///
    /// * `jaccard` - Jaccard similarity [0,1]
    /// * `chunk_count_current` - Number of chunks in current set
    /// * `chunk_count_previous` - Number of chunks in previous set
    /// * `chunk_count_intersection` - Number of chunks in intersection
    pub fn new(
        jaccard: f64,
        chunk_count_current: usize,
        chunk_count_previous: usize,
        chunk_count_intersection: usize,
    ) -> Self {
        Self {
            jaccard,
            chunk_count_current,
            chunk_count_previous,
            chunk_count_intersection,
        }
    }
}

/// Compute MinHash sketch hash from resource tuples (billing graph nodes).
///
/// This function computes a deterministic hash representing a MinHash sketch
/// of the billing graph. The sketch is computed by:
/// 1. Converting each resource tuple to a chunk ID
/// 2. Sorting chunk IDs deterministically
/// 3. Computing SHA-256 hash of sorted chunk IDs
///
/// # Arguments
///
/// * `resource_tuples` - Iterator of resource tuples (e.g., (account_id, service, region, resource_type))
///
/// # Returns
///
/// SHA-256 hash of the MinHash sketch in format `sha256:<64hex>`
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_minhash_sketch;
///
/// let tuples = vec![
///     ("acct1", "s3", "us-east-1", "bucket"),
///     ("acct2", "ec2", "us-west-2", "instance"),
/// ];
/// let sketch_hash = compute_minhash_sketch(tuples.iter().map(|t| format!("{}:{}:{}:{}", t.0, t.1, t.2, t.3)));
/// ```
pub fn compute_minhash_sketch<I, S>(resource_tuples: I) -> String
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Convert each resource tuple to a chunk ID
    let mut chunk_ids: Vec<String> = resource_tuples
        .map(|tuple| chunk_id_from_str(tuple.as_ref()))
        .collect();

    // Sort deterministically
    chunk_ids.sort();

    // Compute hash of sorted chunk IDs
    let combined = chunk_ids.join("|");
    sha256_prefixed(combined.as_bytes())
}

