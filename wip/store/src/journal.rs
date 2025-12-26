//! Journal-backed storage implementation.

use crate::error::StoreError;
use crate::traits::{StoreReader, StoreWriter};
use northroot_journal::{EventJson, JournalReader, JournalWriter, ReadMode, WriteOptions};
use std::path::Path;

/// Journal-backed writer implementation.
pub struct JournalBackendWriter {
    writer: JournalWriter,
}

impl JournalBackendWriter {
    /// Open a journal file for writing.
    pub fn open<P: AsRef<Path>>(path: P, options: WriteOptions) -> Result<Self, StoreError> {
        let writer = JournalWriter::open(path, options)?;
        Ok(Self { writer })
    }
}

impl StoreWriter for JournalBackendWriter {
    fn append(&mut self, event: &EventJson) -> Result<(), StoreError> {
        // Check payload size before appending
        let json_bytes = serde_json::to_vec(event)
            .map_err(|e| StoreError::Other(format!("JSON serialization error: {}", e)))?;
        const MAX_PAYLOAD_SIZE: usize = 16 * 1024 * 1024; // 16 MiB
        if json_bytes.len() > MAX_PAYLOAD_SIZE {
            return Err(StoreError::PayloadTooLarge);
        }
        self.writer.append_event(event)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StoreError> {
        // JournalWriter flushes on each append, but we can call it explicitly
        // The file is already flushed, but this is a no-op for compatibility
        Ok(())
    }

    fn finish(self) -> Result<(), StoreError> {
        self.writer.finish()?;
        Ok(())
    }
}

/// Journal-backed reader implementation.
pub struct JournalBackendReader {
    reader: JournalReader,
}

impl JournalBackendReader {
    /// Open a journal file for reading.
    pub fn open<P: AsRef<Path>>(path: P, mode: ReadMode) -> Result<Self, StoreError> {
        let reader = JournalReader::open(path, mode)?;
        Ok(Self { reader })
    }
}

impl StoreReader for JournalBackendReader {
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError> {
        self.reader.read_event().map_err(StoreError::from)
    }
}
