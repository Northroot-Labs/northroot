//! Storage traits and query types.

use crate::error::StorageError;
use northroot_receipts::Receipt;
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
    fn put_manifest(&self, hash: &[u8; 32], data: &[u8], meta: &ManifestMeta) -> Result<(), StorageError>;

    /// Retrieve manifest by hash.
    ///
    /// Returns decompressed manifest data.
    fn get_manifest(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError>;

    /// Get previous execution receipt for reuse decision.
    ///
    /// Looks up the most recent execution receipt with matching PAC and trace_id.
    fn get_previous_execution(&self, pac: &[u8; 32], trace_id: &str) -> Result<Option<Receipt>, StorageError>;

    /// Garbage collect expired manifests.
    ///
    /// Removes manifests with expires_at < before timestamp.
    /// Returns the number of manifests removed.
    fn gc_manifests(&self, before: i64) -> Result<usize, StorageError>;
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

