//! Filesystem-based storage backend for receipts.
//!
//! This implementation provides a simple, local filesystem store for receipts.
//! Designed for v0.1 SDK use cases where simplicity and human-readability are priorities.
//!
//! ## Design
//!
//! - Receipts stored as JSON files (human-readable for debugging)
//! - Filename: `{rid}.json` (using receipt RID)
//! - Directory structure: `{base_path}/receipts/`
//! - Simple, no database dependencies

use crate::error::StorageError;
use crate::traits::{ManifestMeta, ReceiptQuery, ReceiptStore};
use northroot_receipts::{adapters::json::receipt_to_json, EncryptedLocatorRef, Receipt};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono;

/// Filesystem-based storage backend.
///
/// Stores receipts as JSON files in a directory structure.
/// Simple and human-readable, suitable for local development and v0.1 SDK.
#[derive(Clone)]
pub struct FilesystemStore {
    _base_path: PathBuf, // Reserved for future use (e.g., manifests subdirectory)
    receipts_dir: PathBuf,
    // Mutex for thread-safety (simple implementation)
    _lock: Arc<Mutex<()>>,
}

impl FilesystemStore {
    /// Create a new filesystem store.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Base directory for storage (will create `receipts/` subdirectory)
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if directory cannot be created
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self, StorageError> {
        let base_path = base_path.as_ref().to_path_buf();
        let receipts_dir = base_path.join("receipts");

        // Create receipts directory if it doesn't exist
        fs::create_dir_all(&receipts_dir).map_err(|e| {
            StorageError::IoError(format!(
                "Failed to create receipts directory {}: {}",
                receipts_dir.display(),
                e
            ))
        })?;

        Ok(Self {
            _base_path: base_path,
            receipts_dir,
            _lock: Arc::new(Mutex::new(())),
        })
    }

    /// Get the path to a receipt file.
    fn receipt_path(&self, rid: &Uuid) -> PathBuf {
        self.receipts_dir.join(format!("{}.json", rid))
    }

    /// Check if a receipt matches the query criteria.
    fn matches_query(&self, receipt: &Receipt, q: &ReceiptQuery) -> bool {
        // Filter by trace_id
        if let Some(ref trace_id) = q.trace_id {
            if let northroot_receipts::Payload::Execution(ref exec) = receipt.payload {
                if exec.trace_id != *trace_id {
                    return false;
                }
            } else {
                // Non-execution receipts don't have trace_id
                return false;
            }
        }

        // Filter by workload_id (method_id from ExecutionPayload)
        if let Some(ref workload_id) = q.workload_id {
            if let northroot_receipts::Payload::Execution(ref exec) = receipt.payload {
                if exec.method_ref.method_id != *workload_id {
                    return false;
                }
            } else {
                // Non-execution receipts don't have workload_id
                return false;
            }
        }

        // Filter by timestamp range
        if q.timestamp_from.is_some() || q.timestamp_to.is_some() {
            // Parse RFC3339 timestamp to Unix epoch seconds
            match chrono::DateTime::parse_from_rfc3339(&receipt.ctx.timestamp) {
                Ok(dt) => {
                    let unix_secs = dt.timestamp();
                    if let Some(from) = q.timestamp_from {
                        if unix_secs < from {
                            return false;
                        }
                    }
                    if let Some(to) = q.timestamp_to {
                        if unix_secs > to {
                            return false;
                        }
                    }
                }
                Err(_) => {
                    // Invalid timestamp - skip this receipt
                    return false;
                }
            }
        }

        // Filter by policy_ref
        if let Some(ref policy_ref) = q.policy_ref {
            if receipt.ctx.policy_ref.as_ref() != Some(policy_ref) {
                return false;
            }
        }

        // Filter by change_epoch_id (from ExecutionPayload)
        if let Some(ref epoch_id) = q.change_epoch_id {
            if let northroot_receipts::Payload::Execution(ref exec) = receipt.payload {
                if exec.change_epoch.as_ref() != Some(epoch_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filter by PAC (from ExecutionPayload)
        if let Some(ref pac) = q.pac {
            if let northroot_receipts::Payload::Execution(ref exec) = receipt.payload {
                if exec.pac.as_ref() != Some(pac) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl ReceiptStore for FilesystemStore {
    fn store_receipt(&self, r: &Receipt) -> Result<(), StorageError> {
        let _lock = self._lock.lock().unwrap();

        // Serialize receipt to JSON (human-readable for v0.1)
        let json_str = receipt_to_json(r).map_err(|e| {
            StorageError::SerializationError(format!("JSON serialization failed: {}", e))
        })?;

        // Write to file
        let path = self.receipt_path(&r.rid);
        fs::write(&path, json_str).map_err(|e| {
            StorageError::IoError(format!(
                "Failed to write receipt to {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn get_receipt(&self, rid: &Uuid) -> Result<Option<Receipt>, StorageError> {
        let _lock = self._lock.lock().unwrap();

        let path = self.receipt_path(rid);

        if !path.exists() {
            return Ok(None);
        }

        // Read JSON file
        let json_str = fs::read_to_string(&path).map_err(|e| {
            StorageError::IoError(format!("Failed to read receipt from {}: {}", path.display(), e))
        })?;

        // Deserialize from JSON
        let receipt = northroot_receipts::adapters::json::receipt_from_json(&json_str)
            .map_err(|e| {
                StorageError::SerializationError(format!("JSON deserialization failed: {}", e))
            })?;

        Ok(Some(receipt))
    }

    fn query_receipts(&self, q: ReceiptQuery) -> Result<Vec<Receipt>, StorageError> {
        let _lock = self._lock.lock().unwrap();

        let mut receipts = Vec::new();

        // Read all JSON files in receipts directory
        let entries = fs::read_dir(&self.receipts_dir).map_err(|e| {
            StorageError::IoError(format!(
                "Failed to read receipts directory {}: {}",
                self.receipts_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                StorageError::IoError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json_str = fs::read_to_string(&path).map_err(|e| {
                    StorageError::IoError(format!("Failed to read receipt file: {}", e))
                })?;

                match northroot_receipts::adapters::json::receipt_from_json(&json_str) {
                    Ok(receipt) => {
                        // Apply filters
                        if !self.matches_query(&receipt, &q) {
                            continue;
                        }
                        receipts.push(receipt);
                    }
                    Err(e) => {
                        // Log error but continue (skip corrupted files)
                        eprintln!("Warning: Failed to parse receipt file {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Apply limit
        if let Some(limit) = q.limit {
            receipts.truncate(limit);
        }

        Ok(receipts)
    }

    fn put_manifest(
        &self,
        _hash: &[u8; 32],
        _data: &[u8],
        _meta: &ManifestMeta,
    ) -> Result<(), StorageError> {
        // For v0.1 minimal SDK, manifests are not stored
        // This can be implemented later if needed
        Ok(())
    }

    fn get_manifest(&self, _hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError> {
        // For v0.1 minimal SDK, manifests are not stored
        Ok(None)
    }

    fn get_previous_execution(
        &self,
        _pac: &[u8; 32],
        _trace_id: &str,
    ) -> Result<Option<Receipt>, StorageError> {
        // For v0.1 minimal SDK, simple implementation: not supported
        // Can be enhanced later with indexing
        Ok(None)
    }

    fn gc_manifests(&self, _before: i64) -> Result<usize, StorageError> {
        // No manifests stored, nothing to GC
        Ok(0)
    }

    fn store_encrypted_locator(
        &self,
        _execution_rid: &Uuid,
        _locator_ref: &EncryptedLocatorRef,
    ) -> Result<(), StorageError> {
        // For v0.1 minimal SDK, encrypted locators are not stored
        Ok(())
    }

    fn get_encrypted_locator(
        &self,
        _execution_rid: &Uuid,
    ) -> Result<Option<EncryptedLocatorRef>, StorageError> {
        // For v0.1 minimal SDK, encrypted locators are not stored
        Ok(None)
    }

    fn query_by_output_digest(
        &self,
        _output_digest: &str,
    ) -> Result<Vec<Receipt>, StorageError> {
        // For v0.1 minimal SDK, simple implementation: not supported
        // Can be enhanced later with indexing
        Ok(Vec::new())
    }

    fn get_output_info(
        &self,
        _execution_rid: &Uuid,
    ) -> Result<Option<(String, EncryptedLocatorRef)>, StorageError> {
        // For v0.1 minimal SDK, output info is not stored
        Ok(None)
    }

    fn store_manifest_summary(
        &self,
        _manifest_hash: &[u8; 32],
        _pac: &[u8; 32],
        _chunk_count: u64,
        _minhash_sketch: &[u8],
        _hll_cardinality: Option<u64>,
        _bloom_filter: Option<&[u8]>,
    ) -> Result<(), StorageError> {
        // For v0.1 minimal SDK, manifest summaries are not stored
        Ok(())
    }

    fn get_manifest_summary(
        &self,
        _manifest_hash: &[u8; 32],
    ) -> Result<Option<crate::traits::ManifestSummary>, StorageError> {
        // For v0.1 minimal SDK, manifest summaries are not stored
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_engine::api::record_work;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_filesystem_store_basic() {
        let temp_dir = TempDir::new().unwrap();
        let store = FilesystemStore::new(temp_dir.path()).unwrap();

        // Create a test receipt
        let receipt = record_work(
            "test-workload",
            json!({"input": "test"}),
            vec![],
            None,
            None,
        )
        .unwrap();

        // Store receipt
        store.store_receipt(&receipt).unwrap();

        // Retrieve receipt
        let retrieved = store.get_receipt(&receipt.rid).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.rid, receipt.rid);
        assert_eq!(retrieved.hash, receipt.hash);

        // Verify file exists
        let receipt_file = store.receipt_path(&receipt.rid);
        assert!(receipt_file.exists());
    }

    #[test]
    fn test_filesystem_store_query_all() {
        let temp_dir = TempDir::new().unwrap();
        let store = FilesystemStore::new(temp_dir.path()).unwrap();

        // Create multiple receipts
        let receipt1 = record_work("workload-1", json!({"data": 1}), vec![], None, None).unwrap();
        let receipt2 = record_work("workload-2", json!({"data": 2}), vec![], None, None).unwrap();

        store.store_receipt(&receipt1).unwrap();
        store.store_receipt(&receipt2).unwrap();

        // Query all receipts
        let query = ReceiptQuery {
            pac: None,
            change_epoch_id: None,
            policy_ref: None,
            trace_id: None,
            timestamp_from: None,
            timestamp_to: None,
            limit: None,
        };

        let receipts = store.query_receipts(query).unwrap();
        assert_eq!(receipts.len(), 2);
    }

    #[test]
    fn test_filesystem_store_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FilesystemStore::new(temp_dir.path()).unwrap();

        // Generate a UUID that doesn't exist
        let rid = {
            use uuid::Uuid;
            let mut bytes = [0u8; 16];
            use rand::RngCore;
            rand::thread_rng().fill_bytes(&mut bytes);
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            Uuid::from_bytes(bytes)
        };
        let result = store.get_receipt(&rid).unwrap();
        assert!(result.is_none());
    }
}

