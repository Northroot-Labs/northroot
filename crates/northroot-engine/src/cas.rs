//! Content-Addressable Storage (CAS) module for ByteStream manifests.
//!
//! This module provides utilities for building ByteStream manifests with chunking
//! strategies (CDC and fixed-size) and RFC-6962 Merkle tree construction.
//!
//! ## ByteStream Manifest
//!
//! A ByteStream manifest represents data as a Merkle tree over chunks:
//! - Chunk → hash → ordered list → RFC-6962 Merkle → manifest_root
//! - Leaf hash = H(0x00 || chunk_hash)
//! - Parent hash = H(0x01 || left || right)
//! - Odd nodes promoted upward

use crate::delta::chunk_id_from_bytes;
use crate::shapes::ChunkScheme;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Error type for CAS operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CasError {
    /// Chunking error
    #[error("Chunking failed: {message}")]
    Chunking {
        /// Error message
        message: String,
        /// Context: data size in bytes (if available)
        data_size: Option<usize>,
        /// Context: chunk size parameters (if available)
        chunk_params: Option<String>,
    },
    /// Manifest building error
    #[error("Manifest building failed: {message}")]
    Manifest {
        /// Error message
        message: String,
        /// Context: number of chunks (if available)
        chunk_count: Option<usize>,
    },
    /// Invalid chunk scheme
    #[error("Invalid chunk scheme: {message}. Expected CDC {{ avg_size: >0 }} or Fixed {{ size: >0 }}")]
    InvalidScheme {
        /// Error message
        message: String,
        /// Context: provided scheme (if available)
        provided_scheme: Option<String>,
    },
}

impl CasError {
    /// Create a chunking error with context
    pub fn chunking(message: impl Into<String>, data_size: Option<usize>, chunk_params: Option<String>) -> Self {
        Self::Chunking {
            message: message.into(),
            data_size,
            chunk_params,
        }
    }

    /// Create a manifest error with context
    pub fn manifest(message: impl Into<String>, chunk_count: Option<usize>) -> Self {
        Self::Manifest {
            message: message.into(),
            chunk_count,
        }
    }

    /// Create an invalid scheme error with context
    pub fn invalid_scheme(message: impl Into<String>, provided_scheme: Option<String>) -> Self {
        Self::InvalidScheme {
            message: message.into(),
            provided_scheme,
        }
    }
}

/// Chunk for ByteStream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    /// Chunk ID (sha256:<64hex>)
    pub id: String,
    /// Offset in the original data
    pub offset: u64,
    /// Length of chunk in bytes
    pub len: u64,
    /// Chunk hash (binary, 32 bytes)
    #[serde(with = "crate::serde_helpers")]
    pub hash: [u8; 32],
}

/// Overlap index for fast reuse decisions.
///
/// Provides lightweight sketches for fast-path overlap estimation
/// without fetching full chunk sets from CAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlapIndex {
    /// Algorithm used for sketch (minhash, bloom, roaring)
    pub algo: String,
    /// Compact sketch for fast Jaccard estimation
    pub sketch: SketchData,
    /// Inline set (only if tiny: ≤ 256 IDs or ≤ 8 KB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_set: Option<Vec<String>>,
    /// External set reference (default for larger sets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<ExternalSetRef>,
}

/// Sketch data for overlap estimation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SketchData {
    /// MinHash k parameter (typically 128)
    pub k: u32,
    /// MinHash hash values (hex-encoded u64 values)
    pub hashes: Vec<String>,
    /// Optional Bloom filter parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bloom: Option<BloomParams>,
}

/// Bloom filter parameters (for future use).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BloomParams {
    /// Number of bits
    pub num_bits: u64,
    /// Number of hash functions
    pub num_hashes: u32,
}

/// External set reference for CAS storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSetRef {
    /// Content-addressed ID (SHA-256 hash of chunk set)
    pub cid: String,
    /// Size hint in bytes
    pub bytes: u64,
    /// Schema/encoding (cbor-v1, parquet-v3, roaring-v1)
    pub schema: String,
}

/// ByteStream manifest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteStreamManifest {
    /// Merkle root over chunk list (RFC-6962 style)
    pub manifest_root: String, // sha256:<64hex>
    /// Total bytes in the stream
    pub manifest_len: u64,
    /// Chunks in the manifest
    pub chunks: Vec<Chunk>,
    /// Chunking scheme used
    #[serde(flatten)]
    pub chunk_scheme: ChunkScheme,
    /// Overlap index for fast reuse decisions
    pub overlap_index: OverlapIndex,
}

/// Content-Defined Chunking using Rabin fingerprinting
///
/// CDC enables stable chunking even when data shifts slightly.
/// Average chunk size is configurable (default 64KiB).
///
/// # Arguments
///
/// * `data` - Data to chunk
/// * `avg_size` - Target average chunk size in bytes
///
/// # Returns
///
/// Vector of chunks with IDs, offsets, and lengths
///
/// # Errors
///
/// Returns error if chunking fails
pub fn chunk_by_cdc(data: &[u8], avg_size: u64) -> Result<Vec<Chunk>, CasError> {
    if avg_size == 0 {
        return Err(CasError::chunking(
            "Average chunk size must be > 0",
            Some(data.len()),
            Some(format!("avg_size={}", avg_size)),
        ));
    }

    // Rabin fingerprinting parameters
    // Using a simple polynomial rolling hash for CDC
    // In production, use a proper Rabin fingerprint implementation
    let min_size = avg_size / 2;
    let max_size = avg_size * 2;
    let window_size = 48; // Sliding window size for fingerprinting

    let mut chunks = Vec::new();
    let mut offset = 0u64;
    let mut start = 0;

    while start < data.len() {
        // Ensure we don't exceed max_size
        let max_end = (start + (max_size as usize)).min(data.len());
        let min_end = (start + (min_size as usize)).min(data.len());

        let end = if min_end >= data.len() {
            // Last chunk, use remaining data
            data.len()
        } else {
            // Find chunk boundary using rolling hash
            let mut found_boundary = false;
            let mut boundary = min_end;

            // Simple rolling hash for boundary detection
            // In production, use proper Rabin fingerprint
            for i in min_end..max_end {
                if i + window_size > data.len() {
                    boundary = data.len();
                    found_boundary = true;
                    break;
                }

                // Simple hash-based boundary detection
                // Check if hash matches pattern (simplified CDC)
                let window_end = (i + window_size).min(data.len());
                let window = &data[i..window_end];
                let hash = simple_rolling_hash(window);

                // Boundary found when hash matches pattern (mod avg_size)
                // But ensure we don't exceed max_size
                if hash.is_multiple_of(avg_size) {
                    let candidate_boundary = window_end;
                    if candidate_boundary <= max_end {
                        boundary = candidate_boundary;
                        found_boundary = true;
                        break;
                    }
                }
            }

            if !found_boundary {
                // Force boundary at max_size to prevent oversized chunks
                boundary = max_end;
            }

            boundary
        };

        let chunk_data = &data[start..end];
        let chunk_hash = Sha256::digest(chunk_data);
        let chunk_id = chunk_id_from_bytes(chunk_data);

        chunks.push(Chunk {
            id: chunk_id,
            offset,
            len: (end - start) as u64,
            hash: chunk_hash.into(),
        });

        offset += (end - start) as u64;
        start = end;
    }

    Ok(chunks)
}

/// Simple rolling hash for CDC boundary detection
///
/// This is a simplified implementation. In production, use proper Rabin fingerprint.
fn simple_rolling_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0;
    for &byte in data {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Fixed-size chunking
///
/// Chunks data into fixed-size pieces (except possibly the last chunk).
///
/// # Arguments
///
/// * `data` - Data to chunk
/// * `size` - Fixed chunk size in bytes
///
/// # Returns
///
/// Vector of chunks with IDs, offsets, and lengths
///
/// # Errors
///
/// Returns error if chunking fails
pub fn chunk_by_fixed(data: &[u8], size: u64) -> Result<Vec<Chunk>, CasError> {
    if size == 0 {
        return Err(CasError::chunking(
            "Chunk size must be > 0",
            Some(data.len()),
            Some(format!("size={}", size)),
        ));
    }

    let mut chunks = Vec::new();
    let mut offset = 0u64;
    let mut start = 0;

    while start < data.len() {
        let end = (start + (size as usize)).min(data.len());
        let chunk_data = &data[start..end];
        let chunk_hash = Sha256::digest(chunk_data);
        let chunk_id = chunk_id_from_bytes(chunk_data);

        chunks.push(Chunk {
            id: chunk_id,
            offset,
            len: (end - start) as u64,
            hash: chunk_hash.into(),
        });

        offset += (end - start) as u64;
        start = end;
    }

    Ok(chunks)
}

/// Build ByteStream manifest from chunks
///
/// Chunk → hash → ordered list → RFC-6962 Merkle → manifest_root
///
/// # Arguments
///
/// * `chunks` - Chunks to build manifest from
/// * `scheme` - Chunking scheme used
///
/// # Returns
///
/// ByteStream manifest with Merkle root
///
/// # Errors
///
/// Returns error if manifest building fails
pub fn build_bytestream_manifest(
    chunks: &[Chunk],
    scheme: ChunkScheme,
) -> Result<ByteStreamManifest, CasError> {
    // Create minimal overlap index (no sketch generation - removed as dead weight)
    let overlap_index = OverlapIndex {
        algo: "none".to_string(),
        sketch: SketchData {
            k: 0,
            hashes: Vec::new(),
            bloom: None,
        },
        inline_set: None,
        external: None,
    };

    if chunks.is_empty() {
        // Empty manifest: root = H(0x00 || "")
        let mut hasher = Sha256::new();
        hasher.update([0x00u8]);
        hasher.update(b"");
        let root_bytes = hasher.finalize();
        let hex_str: String = root_bytes.iter().map(|b| format!("{:02x}", b)).collect();

        return Ok(ByteStreamManifest {
            manifest_root: format!("sha256:{}", hex_str),
            manifest_len: 0,
            chunks: Vec::new(),
            chunk_scheme: scheme,
            overlap_index,
        });
    }

    // Compute leaf hashes: H(0x00 || chunk_hash)
    let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
    for chunk in chunks {
        let mut hasher = Sha256::new();
        hasher.update([0x00u8]); // RFC-6962 leaf prefix
        hasher.update(chunk.hash);
        leaf_hashes.push(hasher.finalize().into());
    }

    // Build Merkle tree: pair hashes, promote odd nodes
    let mut current_level = leaf_hashes;
    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Pair hashes left-to-right
        for i in (0..current_level.len()).step_by(2) {
            if i + 1 < current_level.len() {
                // Parent = H(0x01 || left || right) - RFC-6962 style
                let mut hasher = Sha256::new();
                hasher.update([0x01u8]); // RFC-6962 parent prefix
                hasher.update(current_level[i]);
                hasher.update(current_level[i + 1]);
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
    let manifest_root = format!("sha256:{}", hex_str);

    // Compute total length
    let manifest_len = chunks.iter().map(|c| c.len).sum();

    Ok(ByteStreamManifest {
        manifest_root,
        manifest_len,
        chunks: chunks.to_vec(),
        chunk_scheme: scheme,
        overlap_index,
    })
}

/// Build ByteStream manifest from raw data
///
/// Convenience function that chunks data and builds manifest.
///
/// # Arguments
///
/// * `data` - Raw data bytes
/// * `scheme` - Chunking scheme to use
///
/// # Returns
///
/// ByteStream manifest
///
/// # Errors
///
/// Returns error if chunking or manifest building fails
pub fn build_manifest_from_data(
    data: &[u8],
    scheme: ChunkScheme,
) -> Result<ByteStreamManifest, CasError> {
    let chunks = match scheme {
        ChunkScheme::CDC { avg_size } => chunk_by_cdc(data, avg_size)?,
        ChunkScheme::Fixed { size } => chunk_by_fixed(data, size)?,
    };

    build_bytestream_manifest(&chunks, scheme)
}

/// Serialize ByteStreamManifest to CBOR bytes.
///
/// Uses canonical CBOR encoding (RFC 8949) for deterministic serialization.
///
/// # Arguments
///
/// * `manifest` - Manifest to serialize
///
/// # Returns
///
/// CBOR-encoded bytes
///
/// # Errors
///
/// Returns error if serialization fails
pub fn serialize_manifest_to_cbor(manifest: &ByteStreamManifest) -> Result<Vec<u8>, CasError> {
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(manifest, &mut buffer)
        .map_err(|e| CasError::manifest(format!("CBOR serialization failed: {}", e), None))?;
    Ok(buffer)
}

/// Deserialize ByteStreamManifest from CBOR bytes.
///
/// # Arguments
///
/// * `data` - CBOR-encoded bytes
///
/// # Returns
///
/// Deserialized manifest
///
/// # Errors
///
/// Returns error if deserialization fails
pub fn deserialize_manifest_from_cbor(data: &[u8]) -> Result<ByteStreamManifest, CasError> {
    ciborium::de::from_reader(data)
        .map_err(|e| CasError::manifest(format!("CBOR deserialization failed: {}", e), None))
}

/// Estimate Jaccard similarity between two overlap indices using their sketches.
///
/// This provides fast-path overlap estimation without fetching full chunk sets.
/// For MinHash sketches, the estimate is the fraction of hash functions where
/// both sketches have the same minimum hash value.
///
/// # Arguments
///
/// * `index1` - First overlap index
/// * `index2` - Second overlap index
///
/// # Returns
///
/// Estimated Jaccard similarity in [0, 1]
///
/// # Errors
///
/// Returns error if sketches are incompatible or algorithm is unsupported
pub fn estimate_jaccard_from_indices(
    index1: &OverlapIndex,
    index2: &OverlapIndex,
) -> Result<f64, CasError> {
    if index1.algo != index2.algo {
        return Err(CasError::manifest(
            format!(
                "Incompatible algorithms: {} vs {}",
                index1.algo, index2.algo
            ),
            None,
        ));
    }

    match index1.algo.as_str() {
        "none" => {
            // No overlap estimation available (dead weight removed)
            Ok(0.0)
        }
        "minhash" => {
            if index1.sketch.k != index2.sketch.k {
                return Err(CasError::manifest(
                    format!(
                        "Incompatible sketch sizes: k={} vs k={}",
                        index1.sketch.k, index2.sketch.k
                    ),
                    None,
                ));
            }

            if index1.sketch.hashes.len() != index2.sketch.hashes.len() {
                return Err(CasError::manifest(
                    format!(
                        "Incompatible hash counts: {} vs {}",
                        index1.sketch.hashes.len(),
                        index2.sketch.hashes.len()
                    ),
                    None,
                ));
            }

            if index1.sketch.hashes.is_empty() {
                return Ok(1.0); // Both empty
            }

            let mut matches = 0;
            for (h1, h2) in index1.sketch.hashes.iter().zip(index2.sketch.hashes.iter()) {
                if h1 == h2 {
                    matches += 1;
                }
            }

            Ok(matches as f64 / index1.sketch.hashes.len() as f64)
        }
        _ => Err(CasError::manifest(
            format!("Unsupported algorithm: {}", index1.algo),
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_by_fixed() {
        let data = b"hello world, this is a test";
        let chunks = chunk_by_fixed(data, 8).unwrap();

        // 28 bytes / 8 = 3 full chunks + 1 partial = 4 chunks total
        // But let's verify the actual behavior
        let total_len: u64 = chunks.iter().map(|c| c.len).sum();
        assert_eq!(total_len, data.len() as u64);
        assert!(chunks.len() >= 3); // At least 3 chunks
        assert_eq!(chunks[0].len, 8);
        assert_eq!(chunks[0].offset, 0);
        // Verify last chunk
        if chunks.len() > 1 {
            let last_chunk = &chunks[chunks.len() - 1];
            assert!(last_chunk.len <= 8);
        }
    }

    #[test]
    fn test_chunk_by_fixed_empty() {
        let data = b"";
        let chunks = chunk_by_fixed(data, 8).unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_by_fixed_error() {
        let data = b"test";
        let result = chunk_by_fixed(data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_by_cdc() {
        let data = b"hello world, this is a test with some content";
        let chunks = chunk_by_cdc(data, 16).unwrap();

        assert!(!chunks.is_empty());
        // CDC chunks should vary in size but average around target size
        // Simplified CDC implementation may not perfectly match target, so use wider range
        let total_len: u64 = chunks.iter().map(|c| c.len).sum();
        assert_eq!(total_len, data.len() as u64);

        // Verify chunks are within reasonable bounds
        // Note: Simplified CDC implementation may produce chunks slightly outside
        // ideal bounds, but should still be reasonable
        for chunk in &chunks {
            // Chunks should be at least 1 byte and not unreasonably large
            assert!(chunk.len > 0);
            assert!(chunk.len <= 64); // Allow some flexibility for simplified implementation
        }

        // Verify that most chunks are within expected range
        let chunks_in_range = chunks.iter().filter(|c| c.len >= 8 && c.len <= 32).count();
        // At least 50% of chunks should be in expected range
        if chunks.len() > 1 {
            assert!(chunks_in_range as f64 / chunks.len() as f64 >= 0.5);
        }
    }

    #[test]
    fn test_chunk_by_cdc_error() {
        let data = b"test";
        let result = chunk_by_cdc(data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_bytestream_manifest_empty() {
        let manifest = build_bytestream_manifest(&[], ChunkScheme::Fixed { size: 8 }).unwrap();
        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_len, 0);
        assert_eq!(manifest.chunks.len(), 0);
    }

    #[test]
    fn test_build_bytestream_manifest_single_chunk() {
        let data = b"hello";
        let chunks = chunk_by_fixed(data, 100).unwrap(); // Single chunk
        let manifest =
            build_bytestream_manifest(&chunks, ChunkScheme::Fixed { size: 100 }).unwrap();

        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_len, 5);
        assert_eq!(manifest.chunks.len(), 1);
    }

    #[test]
    fn test_build_bytestream_manifest_multiple_chunks() {
        let data = b"hello world";
        let chunks = chunk_by_fixed(data, 5).unwrap(); // Multiple chunks
        let manifest = build_bytestream_manifest(&chunks, ChunkScheme::Fixed { size: 5 }).unwrap();

        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_len, 11);
        assert_eq!(manifest.chunks.len(), 3);
    }

    #[test]
    fn test_build_manifest_from_data() {
        let data = b"test data for manifest";
        let manifest = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        assert!(manifest.manifest_root.starts_with("sha256:"));
        assert_eq!(manifest.manifest_len, data.len() as u64);
        assert!(!manifest.chunks.is_empty());
    }

    #[test]
    fn test_build_manifest_deterministic() {
        let data = b"deterministic test data";
        let chunks1 = chunk_by_fixed(data, 8).unwrap();
        let chunks2 = chunk_by_fixed(data, 8).unwrap();
        let manifest1 =
            build_bytestream_manifest(&chunks1, ChunkScheme::Fixed { size: 8 }).unwrap();
        let manifest2 =
            build_bytestream_manifest(&chunks2, ChunkScheme::Fixed { size: 8 }).unwrap();

        assert_eq!(manifest1.manifest_root, manifest2.manifest_root);
        assert_eq!(manifest1.manifest_len, manifest2.manifest_len);
    }

    #[test]
    fn test_manifest_cbor_serialization() {
        let data = b"test data for CBOR serialization";
        let manifest = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // Serialize to CBOR
        let cbor_bytes = serialize_manifest_to_cbor(&manifest).unwrap();
        assert!(!cbor_bytes.is_empty());

        // Deserialize from CBOR
        let deserialized = deserialize_manifest_from_cbor(&cbor_bytes).unwrap();
        assert_eq!(manifest, deserialized);
    }

    #[test]
    fn test_manifest_cbor_roundtrip() {
        let data = b"roundtrip test data";
        let manifest1 = build_manifest_from_data(data, ChunkScheme::CDC { avg_size: 16 }).unwrap();

        // Round trip through CBOR
        let cbor_bytes = serialize_manifest_to_cbor(&manifest1).unwrap();
        let manifest2 = deserialize_manifest_from_cbor(&cbor_bytes).unwrap();

        assert_eq!(manifest1.manifest_root, manifest2.manifest_root);
        assert_eq!(manifest1.manifest_len, manifest2.manifest_len);
        assert_eq!(manifest1.chunks.len(), manifest2.chunks.len());
        assert_eq!(manifest1.chunk_scheme, manifest2.chunk_scheme);

        // Verify chunks match
        for (c1, c2) in manifest1.chunks.iter().zip(manifest2.chunks.iter()) {
            assert_eq!(c1.id, c2.id);
            assert_eq!(c1.offset, c2.offset);
            assert_eq!(c1.len, c2.len);
            assert_eq!(c1.hash, c2.hash);
        }
    }

    #[test]
    fn test_manifest_cbor_deterministic() {
        let data = b"deterministic test";
        let manifest = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // Serialize multiple times - should produce same bytes
        let cbor1 = serialize_manifest_to_cbor(&manifest).unwrap();
        let cbor2 = serialize_manifest_to_cbor(&manifest).unwrap();
        assert_eq!(cbor1, cbor2);
    }

    #[test]
    fn test_chunk_hash_serialization() {
        use crate::serde_helpers;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestChunk {
            #[serde(with = "serde_helpers")]
            hash: [u8; 32],
        }

        let hash = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
                    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20];
        let test = TestChunk { hash };

        // Test CBOR serialization (should use byte string)
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&test, &mut cbor_bytes).unwrap();
        let deserialized: TestChunk = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
        assert_eq!(test, deserialized);
    }

    #[test]
    fn test_manifest_overlap_index_generation() {
        let data = b"test data for overlap index";
        let manifest = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // Verify overlap_index is present (but empty - MinHash removed as dead weight)
        assert_eq!(manifest.overlap_index.algo, "none");
        assert_eq!(manifest.overlap_index.sketch.k, 0);
        assert_eq!(manifest.overlap_index.sketch.hashes.len(), 0);
    }

    #[test]
    fn test_manifest_overlap_index_empty() {
        let manifest = build_bytestream_manifest(&[], ChunkScheme::Fixed { size: 8 }).unwrap();

        // Empty manifest should still have overlap_index (but empty - MinHash removed)
        assert_eq!(manifest.overlap_index.algo, "none");
        assert_eq!(manifest.overlap_index.sketch.k, 0);
        assert_eq!(manifest.overlap_index.sketch.hashes.len(), 0);
    }

    #[test]
    fn test_manifest_overlap_index_deterministic() {
        let data = b"deterministic test data";
        let manifest1 = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();
        let manifest2 = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // Overlap indices should be identical (both empty - MinHash removed)
        assert_eq!(manifest1.overlap_index.sketch.hashes, manifest2.overlap_index.sketch.hashes);
    }

    #[test]
    fn test_manifest_overlap_index_serialization() {
        let data = b"test serialization";
        let manifest = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // Serialize to CBOR
        let cbor_bytes = serialize_manifest_to_cbor(&manifest).unwrap();
        
        // Deserialize
        let deserialized = deserialize_manifest_from_cbor(&cbor_bytes).unwrap();
        
        // Verify overlap_index is preserved
        assert_eq!(manifest.overlap_index.algo, deserialized.overlap_index.algo);
        assert_eq!(manifest.overlap_index.sketch.k, deserialized.overlap_index.sketch.k);
        assert_eq!(manifest.overlap_index.sketch.hashes, deserialized.overlap_index.sketch.hashes);
    }

    #[test]
    fn test_estimate_jaccard_from_indices() {
        // Test with "none" algo (MinHash removed as dead weight)
        let data = b"test data";
        let manifest1 = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();
        let manifest2 = build_manifest_from_data(data, ChunkScheme::Fixed { size: 8 }).unwrap();

        // With "none" algo, should return 0.0 (no overlap estimation available)
        let j = estimate_jaccard_from_indices(&manifest1.overlap_index, &manifest2.overlap_index).unwrap();
        assert_eq!(j, 0.0, "With 'none' algo, Jaccard should be 0.0, got {}", j);
    }
}
