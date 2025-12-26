//! Get command implementation.

use crate::output;
use crate::path;
use northroot_canonical::{Digest, DigestAlg};
use northroot_store::{EventIdFilter, FilteredReader, JournalBackendReader, ReadMode, StoreReader};

pub fn run(journal: String, event_id: String) -> Result<(), Box<dyn std::error::Error>> {
    // Validate and normalize journal path
    let journal_path = path::validate_journal_path(&journal, false)
        .map_err(|e| format!("Invalid journal path: {}", e))?;

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}", sanitized)
    })?;

    // Parse event ID
    let digest =
        Digest::new(DigestAlg::Sha256, event_id).map_err(|e| format!("Invalid event ID: {}", e))?;

    // Create filtered reader
    let filter = EventIdFilter { event_id: digest };
    let mut filtered = FilteredReader::new(reader, filter);

    // Find the event
    match filtered.read_next()? {
        Some(event) => {
            println!("{}", output::format_json(&event));
            Ok(())
        }
        None => {
            eprintln!("Event not found");
            std::process::exit(1);
        }
    }
}
