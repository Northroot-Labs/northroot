//! Storage traits and query types.

use crate::error::StorageError;
use northroot_receipts::{EncryptedLocatorRef, Receipt};
use uuid::Uuid;

/// Storage backend for receipts and manifests.
///
/// This trait provides a unified interface for storing and retrieving
/// receipts and manifests with support for content-addressed lookup.
pub trait ReceiptStore: Send + Sync {
    /// Store a receipt (immutable, permanent).
    ///
    /// Receipts are stored with their canonical CBOR representation
    /// and indexed by RID, PAC, epoch, policy, and timestamp.
    fn store_receipt(&self, r: &Receipt) -> Result<(), StorageError>;

    /// Retrieve receipt by RID.
    fn get_receipt(&self, rid: &Uuid) -> Result<Option<Receipt>, StorageError>;

    /// Query receipts by criteria (PAC, epoch, policy, timestamp range).
    fn query_receipts(&self, q: ReceiptQuery) -> Result<Vec<Receipt>, StorageError>;

    /// Store manifest (compressed, with TTL).
    ///
    /// Manifests are compressed with zstd before storage and can have
    /// an expiration time for garbage collection.
    fn put_manifest(
        &self,
        hash: &[u8; 32],
        data: &[u8],
        meta: &ManifestMeta,
    ) -> Result<(), StorageError>;

    /// Retrieve manifest by hash.
    ///
    /// Returns decompressed manifest data.
    fn get_manifest(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError>;

    /// Get previous execution receipt for reuse decision.
    ///
    /// Looks up the most recent execution receipt with matching PAC and trace_id.
    fn get_previous_execution(
        &self,
        pac: &[u8; 32],
        trace_id: &str,
    ) -> Result<Option<Receipt>, StorageError>;

    /// Garbage collect expired manifests.
    ///
    /// Removes manifests with expires_at < before timestamp.
    /// Returns the number of manifests removed.
    fn gc_manifests(&self, before: i64) -> Result<usize, StorageError>;

    // --- Phase 4: Encrypted Locator Storage ---

    /// Store encrypted locator reference for an execution receipt.
    ///
    /// Associates an encrypted locator with an execution receipt RID.
    /// Used for privacy-preserving artifact resolution.
    ///
    /// # Arguments
    ///
    /// * `execution_rid` - Execution receipt ID
    /// * `locator_ref` - Encrypted locator reference
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails or execution_rid doesn't exist
    fn store_encrypted_locator(
        &self,
        execution_rid: &Uuid,
        locator_ref: &EncryptedLocatorRef,
    ) -> Result<(), StorageError>;

    /// Retrieve encrypted locator reference for an execution receipt.
    ///
    /// # Arguments
    ///
    /// * `execution_rid` - Execution receipt ID
    ///
    /// # Returns
    ///
    /// Some(EncryptedLocatorRef) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn get_encrypted_locator(
        &self,
        execution_rid: &Uuid,
    ) -> Result<Option<EncryptedLocatorRef>, StorageError>;

    // --- Phase 4: Output Digest Storage ---

    /// Query receipts by output digest for fast exact-hit cache lookup.
    ///
    /// Returns all execution receipts with matching output_digest.
    /// Used for finding receipts that produced the same output.
    ///
    /// # Arguments
    ///
    /// * `output_digest` - Output digest in format "sha256:<64hex>"
    ///
    /// # Returns
    ///
    /// Vector of receipts with matching output digest
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn query_by_output_digest(&self, output_digest: &str) -> Result<Vec<Receipt>, StorageError>;

    /// Get output information for an execution receipt.
    ///
    /// Returns output digest and encrypted locator reference if available.
    ///
    /// # Arguments
    ///
    /// * `execution_rid` - Execution receipt ID
    ///
    /// # Returns
    ///
    /// Some((output_digest, locator_ref)) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn get_output_info(
        &self,
        execution_rid: &Uuid,
    ) -> Result<Option<(String, EncryptedLocatorRef)>, StorageError>;

    // --- Phase 4: Manifest Summary Storage ---

    /// Store manifest summary for fast overlap computation.
    ///
    /// Manifest summaries contain MinHash sketches, HLL cardinality estimates,
    /// and Bloom filters for efficient overlap detection without loading full manifests.
    ///
    /// # Arguments
    ///
    /// * `manifest_hash` - Hash of the manifest (32 bytes)
    /// * `pac` - Proof-addressable cache key (32 bytes)
    /// * `chunk_count` - Number of chunks in manifest
    /// * `minhash_sketch` - MinHash sketch bytes
    /// * `hll_cardinality` - HyperLogLog cardinality estimate (optional)
    /// * `bloom_filter` - Bloom filter bytes (optional)
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn store_manifest_summary(
        &self,
        manifest_hash: &[u8; 32],
        pac: &[u8; 32],
        chunk_count: u64,
        minhash_sketch: &[u8],
        hll_cardinality: Option<u64>,
        bloom_filter: Option<&[u8]>,
    ) -> Result<(), StorageError>;

    /// Retrieve manifest summary by manifest hash.
    ///
    /// # Arguments
    ///
    /// * `manifest_hash` - Hash of the manifest (32 bytes)
    ///
    /// # Returns
    ///
    /// Some(ManifestSummary) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if storage operation fails
    fn get_manifest_summary(
        &self,
        manifest_hash: &[u8; 32],
    ) -> Result<Option<ManifestSummary>, StorageError>;
}

/// Query criteria for searching receipts.
#[derive(Debug, Clone, Default)]
pub struct ReceiptQuery {
    /// Search by PAC key (exact match)
    pub pac: Option<[u8; 32]>,
    /// Search by change epoch ID
    pub change_epoch_id: Option<String>,
    /// Search by policy reference
    pub policy_ref: Option<String>,
    /// Search by trace ID
    pub trace_id: Option<String>,
    /// Search by workload ID (method_id from ExecutionPayload)
    pub workload_id: Option<String>,
    /// Search receipts created after this timestamp (Unix epoch seconds)
    pub timestamp_from: Option<i64>,
    /// Search receipts created before this timestamp (Unix epoch seconds)
    pub timestamp_to: Option<i64>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

/// Metadata for manifest storage.
#[derive(Debug, Clone)]
pub struct ManifestMeta {
    /// PAC key associated with this manifest
    pub pac: [u8; 32],
    /// Change epoch ID (optional)
    pub change_epoch_id: Option<String>,
    /// Encoding format ("zstd" or "raw")
    pub encoding: String,
    /// Uncompressed size in bytes
    pub size_uncompressed: usize,
    /// Expiration timestamp (Unix epoch seconds), None = never expire
    pub expires_at: Option<i64>,
}

/// Manifest summary for fast overlap computation.
///
/// Contains reduced manifest data (MinHash, HLL, Bloom filters) for
/// efficient overlap estimation without loading full manifests.
#[derive(Debug, Clone)]
pub struct ManifestSummary {
    /// Manifest hash (32 bytes)
    pub manifest_hash: [u8; 32],
    /// Proof-addressable cache key (32 bytes)
    pub pac: [u8; 32],
    /// Number of chunks in manifest
    pub chunk_count: u64,
    /// MinHash sketch bytes
    pub minhash_sketch: Vec<u8>,
    /// HyperLogLog cardinality estimate (optional)
    pub hll_cardinality: Option<u64>,
    /// Bloom filter bytes (optional)
    pub bloom_filter: Option<Vec<u8>>,
    /// Timestamp when summary was created (Unix epoch seconds)
    pub created_at: i64,
}
