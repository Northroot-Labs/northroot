//! Overlap computation for delta compute decisions.
//!
//! This module provides functions for computing Jaccard similarity and weighted
//! Jaccard on chunk sets, which are used to determine reuse opportunities.

use std::collections::HashSet;

/// Compute Jaccard similarity between two sets.
///
/// Jaccard similarity is defined as:
/// J(U,V) = |U ∩ V| / |U ∪ V|
///
/// Returns a value in [0, 1] where:
/// - 0 means no overlap
/// - 1 means identical sets
///
/// # Arguments
///
/// * `set1` - First set of chunk identifiers
/// * `set2` - Second set of chunk identifiers
///
/// # Returns
///
/// Jaccard similarity in [0, 1]
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::jaccard_similarity;
/// use std::collections::HashSet;
///
/// let set1: HashSet<String> = ["a".to_string(), "b".to_string(), "c".to_string()].into_iter().collect();
/// let set2: HashSet<String> = ["b".to_string(), "c".to_string(), "d".to_string()].into_iter().collect();
///
/// let j = jaccard_similarity(&set1, &set2);
/// // j = 2/4 = 0.5 (intersection: {b, c}, union: {a, b, c, d})
/// ```
pub fn jaccard_similarity<T>(set1: &HashSet<T>, set2: &HashSet<T>) -> f64
where
    T: std::hash::Hash + Eq + Clone,
{
    if set1.is_empty() && set2.is_empty() {
        return 1.0; // Both empty sets are identical
    }

    let intersection: HashSet<T> = set1.intersection(set2).cloned().collect();
    let union: HashSet<T> = set1.union(set2).cloned().collect();

    if union.is_empty() {
        0.0
    } else {
        intersection.len() as f64 / union.len() as f64
    }
}

/// Compute weighted Jaccard similarity between two sets.
///
/// Weighted Jaccard is defined as:
/// J_w(U,V) = Σ(c ∈ U ∩ V) w(c) / Σ(c ∈ U ∪ V) w(c)
///
/// This is useful when chunks have different costs or sizes, and we want
/// to weight the overlap by those values.
///
/// # Arguments
///
/// * `set1` - First set of chunk identifiers
/// * `set2` - Second set of chunk identifiers
/// * `weights` - Function that returns weight for each chunk identifier
///
/// # Returns
///
/// Weighted Jaccard similarity in [0, 1]
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::weighted_jaccard_similarity;
/// use std::collections::HashSet;
///
/// let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
/// let set2: HashSet<String> = ["b".to_string(), "c".to_string()].into_iter().collect();
///
/// let weights = |chunk: &String| -> f64 {
///     match chunk.as_str() {
///         "a" => 10.0,
///         "b" => 5.0,
///         "c" => 15.0,
///         _ => 1.0,
///     }
/// };
///
/// let j = weighted_jaccard_similarity(&set1, &set2, weights);
/// // intersection weight: 5.0 (b), union weight: 30.0 (a:10 + b:5 + c:15)
/// // j = 5.0 / 30.0 ≈ 0.167
/// ```
pub fn weighted_jaccard_similarity<T, F>(set1: &HashSet<T>, set2: &HashSet<T>, weights: F) -> f64
where
    T: std::hash::Hash + Eq + Clone,
    F: Fn(&T) -> f64,
{
    let intersection: HashSet<T> = set1.intersection(set2).cloned().collect();
    let union: HashSet<T> = set1.union(set2).cloned().collect();

    let intersection_weight: f64 = intersection.iter().map(&weights).sum();
    let union_weight: f64 = union.iter().map(&weights).sum();

    if union_weight == 0.0 {
        0.0
    } else {
        intersection_weight / union_weight
    }
}

/// Verify exact set membership for strict reuse decisions.
///
/// This function performs exact set operations to confirm that a candidate
/// chunk set (from sketch-based estimation) actually matches the expected set.
/// This is required before claiming strict reuse.
///
/// # Arguments
///
/// * `candidate` - Candidate chunk set (from estimation)
/// * `expected` - Expected chunk set (exact)
///
/// # Returns
///
/// `true` if sets are identical, `false` otherwise
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::verify_exact_set;
/// use std::collections::HashSet;
///
/// let candidate: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
/// let expected: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
///
/// assert!(verify_exact_set(&candidate, &expected));
/// ```
pub fn verify_exact_set<T>(candidate: &HashSet<T>, expected: &HashSet<T>) -> bool
where
    T: std::hash::Hash + Eq,
{
    candidate == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_jaccard_similarity_identical() {
        let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let set2: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();

        assert_eq!(jaccard_similarity(&set1, &set2), 1.0);
    }

    #[test]
    fn test_jaccard_similarity_no_overlap() {
        let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let set2: HashSet<String> = ["c".to_string(), "d".to_string()].into_iter().collect();

        assert_eq!(jaccard_similarity(&set1, &set2), 0.0);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let set1: HashSet<String> = ["a".to_string(), "b".to_string(), "c".to_string()]
            .into_iter()
            .collect();
        let set2: HashSet<String> = ["b".to_string(), "c".to_string(), "d".to_string()]
            .into_iter()
            .collect();

        // Intersection: {b, c} = 2, Union: {a, b, c, d} = 4
        assert_eq!(jaccard_similarity(&set1, &set2), 0.5);
    }

    #[test]
    fn test_jaccard_similarity_empty() {
        let set1: HashSet<String> = HashSet::new();
        let set2: HashSet<String> = HashSet::new();

        assert_eq!(jaccard_similarity(&set1, &set2), 1.0);
    }

    #[test]
    fn test_jaccard_similarity_one_empty() {
        let set1: HashSet<String> = ["a".to_string()].into_iter().collect();
        let set2: HashSet<String> = HashSet::new();

        assert_eq!(jaccard_similarity(&set1, &set2), 0.0);
    }

    #[test]
    fn test_weighted_jaccard_similarity() {
        let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let set2: HashSet<String> = ["b".to_string(), "c".to_string()].into_iter().collect();

        let weights = |chunk: &String| -> f64 {
            match chunk.as_str() {
                "a" => 10.0,
                "b" => 5.0,
                "c" => 15.0,
                _ => 1.0,
            }
        };

        // Intersection: {b} = 5.0
        // Union: {a, b, c} = 10.0 + 5.0 + 15.0 = 30.0
        // J = 5.0 / 30.0 ≈ 0.167
        let j = weighted_jaccard_similarity(&set1, &set2, weights);
        assert!((j - 5.0 / 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_verify_exact_set() {
        let set1: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let set2: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let set3: HashSet<String> = ["a".to_string(), "c".to_string()].into_iter().collect();

        assert!(verify_exact_set(&set1, &set2));
        assert!(!verify_exact_set(&set1, &set3));
    }
}
