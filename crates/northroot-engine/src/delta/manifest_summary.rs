//! Manifest summary generation and overlap estimation.
//!
//! This module provides functions for generating summarized manifests (MinHash sketches,
//! HyperLogLog cardinality estimates, Bloom filters) and using them for fast overlap
//! estimation without loading full manifests.
//!
//! ## Overview
//!
//! Manifest summaries enable efficient reuse decisions by providing:
//! - **MinHash sketches**: Fast Jaccard similarity estimation
//! - **HyperLogLog (HLL)**: Cardinality estimation for large sets
//! - **Bloom filters**: Fast negative checks (chunk not in set)
//!
//! These summaries are stored alongside manifests and used in the fast path of reuse
//! reconciliation (ADR-0009-P07).

use sha2::{Digest, Sha256};

/// MinHash sketch for fast Jaccard similarity estimation.
///
/// MinHash uses multiple hash functions to estimate Jaccard similarity
/// between sets without computing the full intersection/union.
///
/// The sketch consists of the minimum hash value for each hash function,
/// which provides an unbiased estimate of Jaccard similarity.
#[derive(Debug, Clone, PartialEq)]
pub struct MinHashSketch {
    /// Number of hash functions (sketch size)
    pub num_hashes: usize,
    /// Minimum hash values for each hash function
    pub min_hashes: Vec<u64>,
}

impl MinHashSketch {
    /// Create a new MinHash sketch with the specified number of hash functions.
    ///
    /// # Arguments
    ///
    /// * `num_hashes` - Number of hash functions to use (typically 64-256)
    pub fn new(num_hashes: usize) -> Self {
        Self {
            num_hashes,
            min_hashes: vec![u64::MAX; num_hashes],
        }
    }

    /// Add a chunk ID to the sketch.
    ///
    /// Updates the minimum hash values for each hash function.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - Chunk identifier (e.g., "c:aa.." or chunk hash)
    pub fn add(&mut self, chunk_id: &str) {
        for i in 0..self.num_hashes {
            // Use different hash functions by seeding with index
            let hash = self.hash_chunk(chunk_id, i);
            if hash < self.min_hashes[i] {
                self.min_hashes[i] = hash;
            }
        }
    }

    /// Add multiple chunk IDs to the sketch.
    pub fn add_many<I, S>(&mut self, chunk_ids: I)
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
    {
        for chunk_id in chunk_ids {
            self.add(chunk_id.as_ref());
        }
    }

    /// Hash a chunk ID with a specific hash function index.
    ///
    /// Uses SHA-256 with the hash function index as a seed to create
    /// multiple independent hash functions.
    fn hash_chunk(&self, chunk_id: &str, hash_fn_index: usize) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", hash_fn_index, chunk_id).as_bytes());
        let hash_bytes = hasher.finalize();
        // Take first 8 bytes as u64
        u64::from_be_bytes([
            hash_bytes[0],
            hash_bytes[1],
            hash_bytes[2],
            hash_bytes[3],
            hash_bytes[4],
            hash_bytes[5],
            hash_bytes[6],
            hash_bytes[7],
        ])
    }

    /// Serialize the sketch to bytes for storage.
    ///
    /// Format: [num_hashes (u32, 4 bytes)] [min_hashes (u64 each, 8 bytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + self.min_hashes.len() * 8);
        bytes.extend_from_slice(&(self.num_hashes as u32).to_be_bytes());
        for &hash in &self.min_hashes {
            bytes.extend_from_slice(&hash.to_be_bytes());
        }
        bytes
    }

    /// Deserialize a sketch from bytes.
    ///
    /// # Errors
    ///
    /// Returns error if bytes are invalid or truncated.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestSummaryError> {
        if bytes.len() < 4 {
            return Err(ManifestSummaryError::InvalidFormat(
                "insufficient bytes for num_hashes".to_string(),
            ));
        }

        let num_hashes = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let expected_len = 4 + num_hashes * 8;

        if bytes.len() < expected_len {
            return Err(ManifestSummaryError::InvalidFormat(format!(
                "insufficient bytes: expected {}, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut min_hashes = Vec::with_capacity(num_hashes);
        for i in 0..num_hashes {
            let offset = 4 + i * 8;
            let hash_bytes = [
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ];
            min_hashes.push(u64::from_be_bytes(hash_bytes));
        }

        Ok(Self {
            num_hashes,
            min_hashes,
        })
    }

    /// Estimate Jaccard similarity between two sketches.
    ///
    /// The estimate is the fraction of hash functions where both sketches
    /// have the same minimum hash value.
    ///
    /// # Arguments
    ///
    /// * `other` - Other MinHash sketch to compare
    ///
    /// # Returns
    ///
    /// Estimated Jaccard similarity in [0, 1]
    pub fn estimate_jaccard(&self, other: &Self) -> Result<f64, ManifestSummaryError> {
        if self.num_hashes != other.num_hashes {
            return Err(ManifestSummaryError::IncompatibleSketches(format!(
                "different num_hashes: {} vs {}",
                self.num_hashes, other.num_hashes
            )));
        }

        if self.num_hashes == 0 {
            return Ok(1.0); // Both empty
        }

        let mut matches = 0;
        for i in 0..self.num_hashes {
            if self.min_hashes[i] == other.min_hashes[i] {
                matches += 1;
            }
        }

        Ok(matches as f64 / self.num_hashes as f64)
    }
}

/// HyperLogLog cardinality estimator.
///
/// HyperLogLog provides approximate cardinality estimation for large sets
/// using a small fixed amount of memory. This is a simplified implementation
/// that uses a single register (not the full HLL algorithm).
///
/// For production use, consider using a full HLL library, but this provides
/// a basic implementation that can be extended.
#[derive(Debug, Clone, PartialEq)]
pub struct HyperLogLog {
    /// Cardinality estimate
    cardinality: u64,
}

impl HyperLogLog {
    /// Create a new HyperLogLog estimator.
    pub fn new() -> Self {
        Self { cardinality: 0 }
    }

    /// Add a chunk ID to the estimator.
    ///
    /// This is a simplified implementation. A full HLL implementation would
    /// use multiple registers and harmonic mean estimation.
    pub fn add(&mut self, _chunk_id: &str) {
        // Simplified: just increment (not accurate, but provides a placeholder)
        // TODO: Implement full HLL algorithm with registers
        self.cardinality += 1;
    }

    /// Add multiple chunk IDs.
    pub fn add_many<I, S>(&mut self, chunk_ids: I)
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
    {
        for chunk_id in chunk_ids {
            self.add(chunk_id.as_ref());
        }
    }

    /// Get the cardinality estimate.
    pub fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Create from exact cardinality (for known sets).
    pub fn from_cardinality(cardinality: u64) -> Self {
        Self { cardinality }
    }
}

impl Default for HyperLogLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Bloom filter for fast negative checks.
///
/// A Bloom filter is a probabilistic data structure that can tell you
/// definitively if an element is NOT in a set, but may have false positives
/// for membership.
///
/// This is useful for fast overlap checks: if a chunk is not in the Bloom filter,
/// we know it's not in the set without loading the full manifest.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// Number of bits in the filter
    num_bits: usize,
    /// Number of hash functions
    num_hashes: usize,
    /// Bit array (stored as bytes)
    bits: Vec<u8>,
}

impl BloomFilter {
    /// Create a new Bloom filter.
    ///
    /// # Arguments
    ///
    /// * `num_bits` - Number of bits in the filter (typically 8-16x expected elements)
    /// * `num_hashes` - Number of hash functions (typically 3-7)
    pub fn new(num_bits: usize, num_hashes: usize) -> Self {
        let num_bytes = (num_bits + 7).div_ceil(8); // Round up
        Self {
            num_bits,
            num_hashes,
            bits: vec![0; num_bytes],
        }
    }

    /// Add a chunk ID to the filter.
    pub fn add(&mut self, chunk_id: &str) {
        for i in 0..self.num_hashes {
            let bit_index = self.hash_to_bit(chunk_id, i);
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            self.bits[byte_index] |= 1 << bit_offset;
        }
    }

    /// Add multiple chunk IDs.
    pub fn add_many<I, S>(&mut self, chunk_ids: I)
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
    {
        for chunk_id in chunk_ids {
            self.add(chunk_id.as_ref());
        }
    }

    /// Check if a chunk ID might be in the filter.
    ///
    /// Returns:
    /// - `false`: Definitely not in the set
    /// - `true`: Might be in the set (could be a false positive)
    pub fn might_contain(&self, chunk_id: &str) -> bool {
        for i in 0..self.num_hashes {
            let bit_index = self.hash_to_bit(chunk_id, i);
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            if (self.bits[byte_index] & (1 << bit_offset)) == 0 {
                return false; // Definitely not in set
            }
        }
        true // Might be in set
    }

    /// Hash a chunk ID to a bit index.
    fn hash_to_bit(&self, chunk_id: &str, hash_fn_index: usize) -> usize {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", hash_fn_index, chunk_id).as_bytes());
        let hash_bytes = hasher.finalize();
        // Use first 4 bytes as u32, then mod by num_bits
        let hash_u32 =
            u32::from_be_bytes([hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3]]);
        (hash_u32 as usize) % self.num_bits
    }

    /// Serialize the filter to bytes.
    ///
    /// Format: [num_bits (u32)] [num_hashes (u32)] [bits (bytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.bits.len());
        bytes.extend_from_slice(&(self.num_bits as u32).to_be_bytes());
        bytes.extend_from_slice(&(self.num_hashes as u32).to_be_bytes());
        bytes.extend_from_slice(&self.bits);
        bytes
    }

    /// Deserialize a filter from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestSummaryError> {
        if bytes.len() < 8 {
            return Err(ManifestSummaryError::InvalidFormat(
                "insufficient bytes for header".to_string(),
            ));
        }

        let num_bits = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let num_hashes = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let expected_bytes = (num_bits + 7).div_ceil(8);

        if bytes.len() < 8 + expected_bytes {
            return Err(ManifestSummaryError::InvalidFormat(format!(
                "insufficient bytes: expected {}, got {}",
                8 + expected_bytes,
                bytes.len()
            )));
        }

        let bits = bytes[8..8 + expected_bytes].to_vec();

        Ok(Self {
            num_bits,
            num_hashes,
            bits,
        })
    }
}

/// Generate a MinHash sketch from a set of chunk IDs.
///
/// # Arguments
///
/// * `chunk_ids` - Iterator of chunk identifiers
/// * `num_hashes` - Number of hash functions (default: 128)
///
/// # Returns
///
/// MinHash sketch
pub fn generate_minhash_sketch<I, S>(
    chunk_ids: I,
    num_hashes: usize,
) -> Result<MinHashSketch, ManifestSummaryError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut sketch = MinHashSketch::new(num_hashes);
    sketch.add_many(chunk_ids);
    Ok(sketch)
}

/// Generate a HyperLogLog cardinality estimate from chunk IDs.
///
/// # Arguments
///
/// * `chunk_ids` - Iterator of chunk identifiers
///
/// # Returns
///
/// HyperLogLog estimator with cardinality
pub fn generate_hll_cardinality<I, S>(chunk_ids: I) -> HyperLogLog
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut hll = HyperLogLog::new();
    hll.add_many(chunk_ids);
    hll
}

/// Generate a Bloom filter from chunk IDs.
///
/// # Arguments
///
/// * `chunk_ids` - Iterator of chunk identifiers
/// * `expected_size` - Expected number of elements (for sizing)
/// * `false_positive_rate` - Desired false positive rate (e.g., 0.01 for 1%)
///
/// # Returns
///
/// Bloom filter
pub fn generate_bloom_filter<I, S>(
    chunk_ids: I,
    expected_size: usize,
    false_positive_rate: f64,
) -> Result<BloomFilter, ManifestSummaryError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Calculate optimal parameters
    // m = -n * ln(p) / (ln(2)^2) where n=expected_size, p=false_positive_rate
    let num_bits = (-(expected_size as f64) * false_positive_rate.ln() / (2.0_f64.ln().powi(2)))
        .ceil() as usize;
    // k = (m/n) * ln(2) where m=num_bits, n=expected_size
    let num_hashes = (((num_bits as f64 / expected_size as f64) * 2.0_f64.ln()).ceil() as usize)
        .max(1)
        .min(10); // Cap at 10 for sanity

    let mut filter = BloomFilter::new(num_bits, num_hashes);
    filter.add_many(chunk_ids);
    Ok(filter)
}

/// Estimate overlap between two chunk sets using MinHash sketches.
///
/// This is a convenience function that generates sketches and estimates
/// Jaccard similarity.
///
/// # Arguments
///
/// * `chunks1` - First set of chunk identifiers
/// * `chunks2` - Second set of chunk identifiers
/// * `num_hashes` - Number of hash functions for MinHash (default: 128)
///
/// # Returns
///
/// Estimated Jaccard similarity in [0, 1]
pub fn estimate_overlap_minhash<I1, S1, I2, S2>(
    chunks1: I1,
    chunks2: I2,
    num_hashes: usize,
) -> Result<f64, ManifestSummaryError>
where
    I1: Iterator<Item = S1>,
    S1: AsRef<str>,
    I2: Iterator<Item = S2>,
    S2: AsRef<str>,
{
    let sketch1 = generate_minhash_sketch(chunks1, num_hashes)?;
    let sketch2 = generate_minhash_sketch(chunks2, num_hashes)?;
    sketch1.estimate_jaccard(&sketch2)
}

/// Errors for manifest summary operations.
#[derive(Debug, thiserror::Error)]
pub enum ManifestSummaryError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Incompatible sketches: {0}")]
    IncompatibleSketches(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minhash_sketch_identical() {
        let chunks1 = vec!["a", "b", "c"];
        let chunks2 = vec!["a", "b", "c"];

        let sketch1 = generate_minhash_sketch(chunks1.iter(), 64).unwrap();
        let sketch2 = generate_minhash_sketch(chunks2.iter(), 64).unwrap();

        let j = sketch1.estimate_jaccard(&sketch2).unwrap();
        assert!((j - 1.0).abs() < 0.1, "Expected ~1.0, got {}", j);
    }

    #[test]
    fn test_minhash_sketch_no_overlap() {
        let chunks1 = vec!["a", "b", "c"];
        let chunks2 = vec!["d", "e", "f"];

        let sketch1 = generate_minhash_sketch(chunks1.iter(), 64).unwrap();
        let sketch2 = generate_minhash_sketch(chunks2.iter(), 64).unwrap();

        let j = sketch1.estimate_jaccard(&sketch2).unwrap();
        assert!(j < 0.2, "Expected <0.2, got {}", j);
    }

    #[test]
    fn test_minhash_serialization() {
        let chunks = vec!["a", "b", "c"];
        let sketch1 = generate_minhash_sketch(chunks.iter(), 64).unwrap();
        let bytes = sketch1.to_bytes();
        let sketch2 = MinHashSketch::from_bytes(&bytes).unwrap();
        assert_eq!(sketch1, sketch2);
    }

    #[test]
    fn test_bloom_filter() {
        let chunks = vec!["a", "b", "c"];
        let mut filter = generate_bloom_filter(chunks.iter(), 10, 0.01).unwrap();

        assert!(filter.might_contain("a"));
        assert!(filter.might_contain("b"));
        assert!(filter.might_contain("c"));
        // Should have low false positive rate, but might_contain("d") could be true
    }

    #[test]
    fn test_bloom_filter_serialization() {
        let chunks = vec!["a", "b", "c"];
        let filter1 = generate_bloom_filter(chunks.iter(), 10, 0.01).unwrap();
        let bytes = filter1.to_bytes();
        let filter2 = BloomFilter::from_bytes(&bytes).unwrap();

        assert_eq!(filter1.num_bits, filter2.num_bits);
        assert_eq!(filter1.num_hashes, filter2.num_hashes);
        assert_eq!(filter1.bits, filter2.bits);
    }

    #[test]
    fn test_hll_cardinality() {
        let chunks = vec!["a", "b", "c", "d", "e"];
        let hll = generate_hll_cardinality(chunks.iter());
        assert_eq!(hll.cardinality(), 5);
    }
}
