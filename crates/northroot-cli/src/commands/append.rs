//! Append command implementation.

use crate::path;
use northroot_store::{JournalBackendWriter, StoreWriter, WriteOptions};
use serde_json::Value;
use std::io::Read;

/// Append an event to a journal.
///
/// Reads event JSON from stdin or a file, validates it, and appends it to the journal.
pub fn run(
    journal: String,
    event_json: Option<String>,
    stdin: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate and normalize journal path
    let journal_path = path::validate_journal_path(&journal, false)
        .map_err(|e| format!("Invalid journal path: {}", e))?;

    // Read event JSON
    let event_str = if stdin {
        // Read from stdin
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Failed to read from stdin: {}", e))?;
        buffer
    } else if let Some(json) = event_json {
        json
    } else {
        return Err("Either --event or --stdin must be provided".into());
    };

    // Parse JSON
    let event: Value =
        serde_json::from_str(&event_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Open writer (WriteOptions defaults to append: true)
    let mut writer =
        JournalBackendWriter::open(&journal_path, WriteOptions::default()).map_err(|e| {
            let sanitized = path::sanitize_path_for_error(&journal_path);
            format!("Failed to open journal for writing: {} ({})", sanitized, e)
        })?;

    // Append event
    writer
        .append(&event)
        .map_err(|e| format!("Failed to append event: {}", e))?;

    // Finish writing
    writer
        .finish()
        .map_err(|e| format!("Failed to finish writing: {}", e))?;

    Ok(())
}
