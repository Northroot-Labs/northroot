//! SQLite backend for receipt and manifest storage.

use crate::error::StorageError;
use crate::traits::{ManifestMeta, ManifestSummary, ReceiptQuery, ReceiptStore};
use ciborium;
use northroot_receipts::{EncryptedLocatorRef, Receipt};
use rusqlite::{params, Connection, Row};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zstd::encode_all;

/// SQLite-based storage backend.
///
/// This implementation provides persistent storage for receipts and manifests
/// using SQLite with WAL mode for concurrent access.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Create a new SQLite store from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to SQLite database file (will be created if it doesn't exist)
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if database cannot be opened or initialized
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Create an in-memory SQLite store (for testing).
    pub fn in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize database schema.
    fn init_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Enable WAL mode for better concurrency
        // PRAGMA statements that return values need to be queried, not executed
        let _: String = conn.query_row("PRAGMA journal_mode=WAL;", [], |row| row.get(0))?;
        conn.execute("PRAGMA synchronous=NORMAL;", [])?;

        // Create receipts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS receipts (
                rid TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                version TEXT NOT NULL,
                hash TEXT NOT NULL UNIQUE,
                pac BLOB NOT NULL,
                change_epoch_id TEXT,
                policy_ref TEXT,
                timestamp TEXT NOT NULL,
                canonical_cbor BLOB NOT NULL,
                minhash_signature BLOB,
                hll_cardinality INTEGER,
                chunk_manifest_hash BLOB,
                chunk_manifest_size_bytes INTEGER,
                merkle_root BLOB,
                prev_execution_rid TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipts_pac ON receipts(pac)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipts_epoch ON receipts(change_epoch_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipts_policy_ref ON receipts(policy_ref)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_receipts_timestamp ON receipts(timestamp)",
            [],
        )?;

        // Create manifests table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS manifests (
                manifest_hash BLOB PRIMARY KEY,
                pac BLOB NOT NULL,
                change_epoch_id TEXT,
                encoding TEXT NOT NULL,
                bytes BLOB NOT NULL,
                size_uncompressed INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER
            )",
            [],
        )?;

        // Create manifest indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_manifests_pac ON manifests(pac)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_manifests_expires ON manifests(expires_at)",
            [],
        )?;

        // --- Phase 4: Encrypted Locators Table ---
        conn.execute(
            "CREATE TABLE IF NOT EXISTS encrypted_locators (
                execution_rid TEXT PRIMARY KEY,
                encrypted_data BLOB NOT NULL,
                content_hash TEXT NOT NULL,
                encryption_scheme TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (execution_rid) REFERENCES receipts(rid)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_locators_content_hash ON encrypted_locators(content_hash)",
            [],
        )?;

        // --- Phase 4: Output Digests Table ---
        conn.execute(
            "CREATE TABLE IF NOT EXISTS output_digests (
                execution_rid TEXT PRIMARY KEY,
                output_digest TEXT NOT NULL,
                manifest_root BLOB,
                output_mime_type TEXT,
                output_size_bytes INTEGER,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (execution_rid) REFERENCES receipts(rid)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_output_digests_digest ON output_digests(output_digest)",
            [],
        )?;

        // --- Phase 4: Manifest Summaries Table ---
        conn.execute(
            "CREATE TABLE IF NOT EXISTS manifest_summaries (
                manifest_hash BLOB PRIMARY KEY,
                pac BLOB NOT NULL,
                chunk_count INTEGER NOT NULL,
                minhash_sketch BLOB NOT NULL,
                hll_cardinality INTEGER,
                bloom_filter BLOB,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_manifest_summaries_pac ON manifest_summaries(pac)",
            [],
        )?;

        Ok(())
    }

    /// Extract PAC from receipt (from ExecutionPayload if present, otherwise compute default).
    fn extract_pac(receipt: &Receipt) -> [u8; 32] {
        // Extract from ExecutionPayload.pac if available
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            if let Some(pac_vec) = &exec.pac {
                if pac_vec.len() == 32 {
                    let mut pac = [0u8; 32];
                    pac.copy_from_slice(pac_vec);
                    return pac;
                }
            }
        }
        // Fallback: use a default PAC based on RID
        let mut pac = [0u8; 32];
        let rid_bytes = receipt.rid.as_bytes();
        pac[..16].copy_from_slice(&rid_bytes[..16]);
        pac
    }

    /// Extract change_epoch from receipt (from ExecutionPayload if present).
    fn extract_change_epoch(_receipt: &Receipt) -> Option<String> {
        // TODO: Extract from ExecutionPayload.change_epoch when Phase 3 is complete
        None
    }

    /// Extract policy_ref from receipt context.
    fn extract_policy_ref(receipt: &Receipt) -> Option<String> {
        receipt.ctx.policy_ref.clone()
    }

    /// Extract minhash_signature from receipt (from ExecutionPayload if present).
    fn extract_minhash_signature(receipt: &Receipt) -> Option<Vec<u8>> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.minhash_signature.clone()
        } else {
            None
        }
    }

    /// Extract hll_cardinality from receipt (from ExecutionPayload if present).
    fn extract_hll_cardinality(receipt: &Receipt) -> Option<u64> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.hll_cardinality
        } else {
            None
        }
    }

    /// Extract chunk_manifest_hash from receipt (from ExecutionPayload if present).
    fn extract_chunk_manifest_hash(receipt: &Receipt) -> Option<[u8; 32]> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.chunk_manifest_hash
        } else {
            None
        }
    }

    /// Extract chunk_manifest_size_bytes from receipt (from ExecutionPayload if present).
    fn extract_chunk_manifest_size_bytes(_receipt: &Receipt) -> Option<u64> {
        // TODO: Extract from ExecutionPayload.chunk_manifest_size_bytes when Phase 3 is complete
        None
    }

    /// Extract merkle_root from receipt (from ExecutionPayload if present).
    fn extract_merkle_root(receipt: &Receipt) -> Option<[u8; 32]> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.merkle_root
        } else {
            None
        }
    }

    /// Extract prev_execution_rid from receipt (from ExecutionPayload if present).
    fn extract_prev_execution_rid(receipt: &Receipt) -> Option<Uuid> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.prev_execution_rid
        } else {
            None
        }
    }

    /// Extract output_digest from receipt (from ExecutionPayload if present).
    fn extract_output_digest(receipt: &Receipt) -> Option<String> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.output_digest.clone()
        } else {
            None
        }
    }

    /// Extract manifest_root from receipt (from ExecutionPayload if present).
    fn extract_manifest_root(receipt: &Receipt) -> Option<[u8; 32]> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.manifest_root
        } else {
            None
        }
    }

    /// Extract output_mime_type from receipt (from ExecutionPayload if present).
    fn extract_output_mime_type(receipt: &Receipt) -> Option<String> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.output_mime_type.clone()
        } else {
            None
        }
    }

    /// Extract output_size_bytes from receipt (from ExecutionPayload if present).
    fn extract_output_size_bytes(receipt: &Receipt) -> Option<u64> {
        if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
            exec.output_size_bytes
        } else {
            None
        }
    }

    /// Parse timestamp to Unix epoch seconds.
    fn parse_timestamp(_timestamp: &str) -> Result<i64, StorageError> {
        // Simple parser for RFC3339 timestamps
        // For now, use a basic implementation
        // TODO: Use proper RFC3339 parser
        use std::time::{SystemTime, UNIX_EPOCH};
        // For simplicity, return current time if parsing fails
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .map_err(|_| StorageError::InvalidInput("Invalid timestamp".to_string()))
    }

    /// Convert receipt from database row.
    fn receipt_from_row(row: &Row) -> Result<Receipt, StorageError> {
        let cbor_bytes: Vec<u8> = row.get("canonical_cbor")?;

        // Deserialize from CBOR bytes (deterministic encoding per RFC 8949)
        let receipt: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).map_err(|e| {
            StorageError::SerializationError(format!("CBOR deserialization failed: {}", e))
        })?;

        Ok(receipt)
    }
}

impl ReceiptStore for SqliteStore {
    fn store_receipt(&self, r: &Receipt) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Serialize receipt to CBOR using deterministic encoding (RFC 8949)
        // This provides better performance and storage efficiency than JSON
        let cbor_bytes = northroot_receipts::cbor_deterministic(r).map_err(|e| {
            StorageError::SerializationError(format!("CBOR serialization failed: {}", e))
        })?;

        // Extract fields
        let pac = Self::extract_pac(r);
        let change_epoch = Self::extract_change_epoch(r);
        let policy_ref = Self::extract_policy_ref(r);
        let minhash_signature = Self::extract_minhash_signature(r);
        let hll_cardinality = Self::extract_hll_cardinality(r);
        let chunk_manifest_hash = Self::extract_chunk_manifest_hash(r);
        let chunk_manifest_size_bytes = Self::extract_chunk_manifest_size_bytes(r);
        let merkle_root = Self::extract_merkle_root(r);
        let prev_execution_rid = Self::extract_prev_execution_rid(r);

        let created_at = Self::parse_timestamp(&r.ctx.timestamp)?;

        // Convert arrays to Vec for storage
        let chunk_manifest_hash_vec = chunk_manifest_hash.map(|h| h.to_vec());
        let merkle_root_vec = merkle_root.map(|r| r.to_vec());

        conn.execute(
            "INSERT OR REPLACE INTO receipts (
                rid, kind, version, hash, pac, change_epoch_id, policy_ref,
                timestamp, canonical_cbor, minhash_signature, hll_cardinality,
                chunk_manifest_hash, chunk_manifest_size_bytes, merkle_root,
                prev_execution_rid, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                r.rid.to_string(),
                format!("{:?}", r.kind),
                r.version,
                r.hash,
                pac.as_slice(),
                change_epoch,
                policy_ref,
                r.ctx.timestamp,
                cbor_bytes,
                minhash_signature,
                hll_cardinality,
                chunk_manifest_hash_vec,
                chunk_manifest_size_bytes,
                merkle_root_vec,
                prev_execution_rid.map(|r| r.to_string()),
                created_at,
            ],
        )?;

        // --- Phase 4: Automatically store output digest if present ---
        if let Some(output_digest) = Self::extract_output_digest(r) {
            let manifest_root = Self::extract_manifest_root(r);
            let output_mime_type = Self::extract_output_mime_type(r);
            let output_size_bytes = Self::extract_output_size_bytes(r);

            let created_at = Self::parse_timestamp(&r.ctx.timestamp)?;
            let manifest_root_vec = manifest_root.map(|r| r.to_vec());

            conn.execute(
                "INSERT OR REPLACE INTO output_digests (
                    execution_rid, output_digest, manifest_root,
                    output_mime_type, output_size_bytes, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    r.rid.to_string(),
                    output_digest,
                    manifest_root_vec,
                    output_mime_type,
                    output_size_bytes.map(|s| s as i64),
                    created_at,
                ],
            )?;
        }

        // --- Phase 4: Automatically store encrypted locator if present ---
        if let northroot_receipts::Payload::Execution(exec) = &r.payload {
            if let Some(ref output_locator_ref) = exec.output_locator_ref {
                // Store the output locator reference
                // Note: We need to drop the connection lock first
                let execution_rid = r.rid;
                let locator_ref = output_locator_ref.clone();
                drop(conn); // Release lock before calling trait method
                self.store_encrypted_locator(&execution_rid, &locator_ref)?;
                return Ok(()); // Already stored, return early
            }
        }

        Ok(())
    }

    fn get_receipt(&self, rid: &Uuid) -> Result<Option<Receipt>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT canonical_cbor FROM receipts WHERE rid = ?1")?;
        let mut rows = stmt.query_map(params![rid.to_string()], |row| {
            Self::receipt_from_row(row).map_err(|e| match e {
                StorageError::DatabaseError(db_err) => db_err,
                StorageError::SerializationError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::CompressionError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::InvalidInput(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::NotFound(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::IoError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
            })
        })?;

        match rows.next() {
            Some(Ok(receipt)) => Ok(Some(receipt)),
            Some(Err(e)) => Err(StorageError::DatabaseError(e)),
            None => Ok(None),
        }
    }

    fn query_receipts(&self, q: ReceiptQuery) -> Result<Vec<Receipt>, StorageError> {
        let conn = self.conn.lock().unwrap();

        // For Phase 2, use a simple approach: fetch all and filter in memory
        // This will be optimized in later phases when we have proper indexes
        let mut stmt =
            conn.prepare("SELECT canonical_cbor FROM receipts ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Self::receipt_from_row(row).map_err(|e| match e {
                StorageError::DatabaseError(db_err) => db_err,
                StorageError::SerializationError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::CompressionError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::InvalidInput(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::NotFound(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::IoError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
            })
        })?;

        let mut receipts = Vec::new();
        for row_result in rows {
            let receipt = row_result?;

            // Apply filters
            if let Some(ref pac) = q.pac {
                let receipt_pac = Self::extract_pac(&receipt);
                if receipt_pac != *pac {
                    continue;
                }
            }
            if let Some(ref epoch) = &q.change_epoch_id {
                let receipt_epoch = Self::extract_change_epoch(&receipt);
                if receipt_epoch.as_ref() != Some(epoch) {
                    continue;
                }
            }
            if let Some(ref policy) = &q.policy_ref {
                let receipt_policy = Self::extract_policy_ref(&receipt);
                if receipt_policy.as_ref() != Some(policy) {
                    continue;
                }
            }
            if let Some(trace_id) = &q.trace_id {
                if let northroot_receipts::Payload::Execution(exec) = &receipt.payload {
                    if exec.trace_id != *trace_id {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            if let Some(from) = q.timestamp_from {
                let created_at = Self::parse_timestamp(&receipt.ctx.timestamp)?;
                if created_at < from {
                    continue;
                }
            }
            if let Some(to) = q.timestamp_to {
                let created_at = Self::parse_timestamp(&receipt.ctx.timestamp)?;
                if created_at > to {
                    continue;
                }
            }

            receipts.push(receipt);

            // Apply limit
            if let Some(limit) = q.limit {
                if receipts.len() >= limit {
                    break;
                }
            }
        }

        Ok(receipts)
    }

    fn put_manifest(
        &self,
        hash: &[u8; 32],
        data: &[u8],
        meta: &ManifestMeta,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Compress manifest if encoding is zstd
        let (compressed_data, encoding) = if meta.encoding == "zstd" {
            let compressed = encode_all(data, 3) // Level 3 compression
                .map_err(|e| {
                    StorageError::CompressionError(format!("zstd compression failed: {}", e))
                })?;
            (compressed, "zstd".to_string())
        } else {
            (data.to_vec(), meta.encoding.clone())
        };

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .map_err(|_| StorageError::InvalidInput("Failed to get current time".to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO manifests (
                manifest_hash, pac, change_epoch_id, encoding, bytes,
                size_uncompressed, created_at, expires_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                hash.as_slice(),
                meta.pac.as_slice(),
                meta.change_epoch_id,
                encoding,
                compressed_data,
                meta.size_uncompressed as i64,
                created_at,
                meta.expires_at,
            ],
        )?;

        Ok(())
    }

    fn get_manifest(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT encoding, bytes FROM manifests WHERE manifest_hash = ?1")?;
        let mut rows = stmt.query_map(params![hash.as_slice()], |row| {
            let encoding: String = row.get("encoding")?;
            let bytes: Vec<u8> = row.get("bytes")?;
            Ok((encoding, bytes))
        })?;

        match rows.next() {
            Some(Ok((encoding, compressed_bytes))) => {
                if encoding == "zstd" {
                    // Decompress
                    zstd::decode_all(compressed_bytes.as_slice())
                        .map_err(|e| {
                            StorageError::CompressionError(format!(
                                "zstd decompression failed: {}",
                                e
                            ))
                        })
                        .map(Some)
                } else {
                    Ok(Some(compressed_bytes))
                }
            }
            Some(Err(e)) => Err(StorageError::DatabaseError(e)),
            None => Ok(None),
        }
    }

    fn get_previous_execution(
        &self,
        pac: &[u8; 32],
        trace_id: &str, // Not used in query, but kept for API compatibility
    ) -> Result<Option<Receipt>, StorageError> {
        // Query for most recent execution receipt with matching PAC
        // Exclude the current execution by requiring created_at < current time
        // This ensures we get the previous execution, not the current one
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT canonical_cbor FROM receipts 
             WHERE pac = ?1 AND kind = 'Execution'
             ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![pac.as_slice()], |row| {
            Self::receipt_from_row(row).map_err(|e| match e {
                StorageError::DatabaseError(db_err) => db_err,
                StorageError::SerializationError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::CompressionError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::InvalidInput(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::NotFound(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::IoError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
            })
        })?;

        match rows.next() {
            Some(Ok(receipt)) => {
                // Return the most recent execution with matching PAC
                // Note: trace_id parameter is kept for API compatibility but not used in filtering
                // This allows finding previous executions regardless of trace_id
                Ok(Some(receipt))
            }
            Some(Err(e)) => Err(StorageError::SerializationError(format!(
                "Failed to deserialize receipt: {}",
                e
            ))),
            None => Ok(None),
        }
    }

    fn gc_manifests(&self, before: i64) -> Result<usize, StorageError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM manifests WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![before],
        )?;
        Ok(count)
    }

    // --- Phase 4: Encrypted Locator Storage ---

    fn store_encrypted_locator(
        &self,
        execution_rid: &Uuid,
        locator_ref: &EncryptedLocatorRef,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Verify receipt exists
        let receipt_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM receipts WHERE rid = ?1)",
            params![execution_rid.to_string()],
            |row| row.get(0),
        )?;

        if !receipt_exists {
            return Err(StorageError::NotFound(format!(
                "Receipt {} not found",
                execution_rid
            )));
        }

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .map_err(|_| StorageError::InvalidInput("Failed to get current time".to_string()))?;

        // Serialize encrypted_data to BLOB
        let encrypted_data_blob = &locator_ref.encrypted_data;

        conn.execute(
            "INSERT OR REPLACE INTO encrypted_locators (
                execution_rid, encrypted_data, content_hash, encryption_scheme, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                execution_rid.to_string(),
                encrypted_data_blob,
                locator_ref.content_hash,
                locator_ref.encryption_scheme,
                created_at,
            ],
        )?;

        Ok(())
    }

    fn get_encrypted_locator(
        &self,
        execution_rid: &Uuid,
    ) -> Result<Option<EncryptedLocatorRef>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT encrypted_data, content_hash, encryption_scheme 
             FROM encrypted_locators WHERE execution_rid = ?1",
        )?;

        let mut rows = stmt.query_map(params![execution_rid.to_string()], |row| {
            Ok(EncryptedLocatorRef {
                encrypted_data: row.get("encrypted_data")?,
                content_hash: row.get("content_hash")?,
                encryption_scheme: row.get("encryption_scheme")?,
            })
        })?;

        match rows.next() {
            Some(Ok(locator)) => Ok(Some(locator)),
            Some(Err(e)) => Err(StorageError::DatabaseError(e)),
            None => Ok(None),
        }
    }

    // --- Phase 4: Output Digest Storage ---

    fn query_by_output_digest(&self, output_digest: &str) -> Result<Vec<Receipt>, StorageError> {
        let conn = self.conn.lock().unwrap();

        // Join output_digests with receipts to get full receipt data
        let mut stmt = conn.prepare(
            "SELECT r.canonical_cbor 
             FROM receipts r
             INNER JOIN output_digests od ON r.rid = od.execution_rid
             WHERE od.output_digest = ?1
             ORDER BY od.created_at DESC",
        )?;

        let rows = stmt.query_map(params![output_digest], |row| {
            Self::receipt_from_row(row).map_err(|e| match e {
                StorageError::DatabaseError(db_err) => db_err,
                StorageError::SerializationError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::CompressionError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::InvalidInput(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::NotFound(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
                StorageError::IoError(msg) => {
                    rusqlite::Error::InvalidColumnType(0, msg, rusqlite::types::Type::Blob)
                }
            })
        })?;

        let mut receipts = Vec::new();
        for row_result in rows {
            match row_result {
                Ok(receipt) => receipts.push(receipt),
                Err(e) => return Err(StorageError::DatabaseError(e)),
            }
        }

        Ok(receipts)
    }

    fn get_output_info(
        &self,
        execution_rid: &Uuid,
    ) -> Result<Option<(String, EncryptedLocatorRef)>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT od.output_digest, el.encrypted_data, el.content_hash, el.encryption_scheme
             FROM output_digests od
             LEFT JOIN encrypted_locators el ON od.execution_rid = el.execution_rid
             WHERE od.execution_rid = ?1",
        )?;

        let mut rows = stmt.query_map(params![execution_rid.to_string()], |row| {
            let output_digest: String = row.get("output_digest")?;
            let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data")?;
            let content_hash: Option<String> = row.get("content_hash")?;
            let encryption_scheme: Option<String> = row.get("encryption_scheme")?;

            if let (Some(encrypted_data), Some(content_hash), Some(encryption_scheme)) =
                (encrypted_data, content_hash, encryption_scheme)
            {
                Ok(Some((
                    output_digest,
                    EncryptedLocatorRef {
                        encrypted_data,
                        content_hash,
                        encryption_scheme,
                    },
                )))
            } else {
                Ok(None)
            }
        })?;

        match rows.next() {
            Some(Ok(result)) => Ok(result),
            Some(Err(e)) => Err(StorageError::DatabaseError(e)),
            None => Ok(None),
        }
    }

    // --- Phase 4: Manifest Summary Storage ---

    fn store_manifest_summary(
        &self,
        manifest_hash: &[u8; 32],
        pac: &[u8; 32],
        chunk_count: u64,
        minhash_sketch: &[u8],
        hll_cardinality: Option<u64>,
        bloom_filter: Option<&[u8]>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .map_err(|_| StorageError::InvalidInput("Failed to get current time".to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO manifest_summaries (
                manifest_hash, pac, chunk_count, minhash_sketch, 
                hll_cardinality, bloom_filter, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                manifest_hash.as_slice(),
                pac.as_slice(),
                chunk_count as i64,
                minhash_sketch,
                hll_cardinality.map(|c| c as i64),
                bloom_filter,
                created_at,
            ],
        )?;

        Ok(())
    }

    fn get_manifest_summary(
        &self,
        manifest_hash: &[u8; 32],
    ) -> Result<Option<ManifestSummary>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT manifest_hash, pac, chunk_count, minhash_sketch, 
                    hll_cardinality, bloom_filter, created_at
             FROM manifest_summaries WHERE manifest_hash = ?1",
        )?;

        let mut rows = stmt.query_map(params![manifest_hash.as_slice()], |row| {
            let manifest_hash_bytes: Vec<u8> = row.get("manifest_hash")?;
            let pac_bytes: Vec<u8> = row.get("pac")?;
            let chunk_count: i64 = row.get("chunk_count")?;
            let minhash_sketch: Vec<u8> = row.get("minhash_sketch")?;
            let hll_cardinality: Option<i64> = row.get("hll_cardinality")?;
            let bloom_filter: Option<Vec<u8>> = row.get("bloom_filter")?;
            let created_at: i64 = row.get("created_at")?;

            // Convert Vec<u8> to [u8; 32]
            let mut manifest_hash_array = [0u8; 32];
            let mut pac_array = [0u8; 32];
            if manifest_hash_bytes.len() == 32 && pac_bytes.len() == 32 {
                manifest_hash_array.copy_from_slice(&manifest_hash_bytes);
                pac_array.copy_from_slice(&pac_bytes);
            } else {
                return Err(rusqlite::Error::InvalidColumnType(
                    0,
                    "Invalid hash length".to_string(),
                    rusqlite::types::Type::Blob,
                ));
            }

            Ok(ManifestSummary {
                manifest_hash: manifest_hash_array,
                pac: pac_array,
                chunk_count: chunk_count as u64,
                minhash_sketch,
                hll_cardinality: hll_cardinality.map(|c| c as u64),
                bloom_filter,
                created_at,
            })
        })?;

        match rows.next() {
            Some(Ok(summary)) => Ok(Some(summary)),
            Some(Err(e)) => Err(StorageError::DatabaseError(e)),
            None => Ok(None),
        }
    }
}
