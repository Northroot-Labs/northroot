//! Storage backend traits.

use crate::error::StoreError;
use northroot_journal::EventJson;

/// Trait for writing events to a store.
pub trait StoreWriter {
    /// Append a canonical event to the store.
    fn append(&mut self, event: &EventJson) -> Result<(), StoreError>;
    /// Flush buffered writes (no-op if unbuffered).
    fn flush(&mut self) -> Result<(), StoreError>;
    /// Finish writing and release resources.
    fn finish(self) -> Result<(), StoreError>;
}

/// Trait for reading events from a store.
pub trait StoreReader {
    /// Read next event, None at EOF.
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError>;
}

