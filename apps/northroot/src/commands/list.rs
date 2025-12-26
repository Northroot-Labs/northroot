//! List command implementation.

use crate::output;
use crate::path;
use northroot_journal::{JournalReader, ReadMode};
use serde_json;

pub fn run(
    journal: String,
    json: bool,
    max_events: Option<u64>,
    max_size: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate and normalize journal path
    let journal_path = path::validate_journal_path(&journal, false)
        .map_err(|e| format!("Invalid journal path: {}", e))?;

    // Check journal size if limit is set
    if let Some(max_bytes) = max_size {
        let metadata = std::fs::metadata(&journal_path)?;
        if metadata.len() > max_bytes {
            return Err(format!(
                "Journal size {} exceeds maximum {} bytes",
                metadata.len(),
                max_bytes
            )
            .into());
        }
    }

    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).map_err(|e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}: {}", sanitized, e)
    })?;

    // Output header if table format
    if !json {
        output::print_table_header();
    }

    let mut event_count: u64 = 0;
    while let Some(event) = reader.read_event()? {
        // Check max_events limit
        if let Some(max) = max_events {
            if event_count >= max {
                break;
            }
        }

        if json {
            println!("{}", serde_json::to_string(&event)?);
        } else {
            println!("{}", output::format_table_row(&event));
        }
        event_count += 1;
    }

    Ok(())
}
