//! List command implementation.

use crate::output;
use crate::path;
use northroot_canonical::Timestamp;
use northroot_store::{
    AndFilter, EventFilter, EventTypeFilter, FilteredReader, JournalBackendReader, PrincipalFilter,
    ReadMode, StoreReader, TimeRangeFilter,
};
use serde_json;

#[allow(clippy::too_many_arguments)]
pub fn run(
    journal: String,
    event_type: Option<String>,
    principal: Option<String>,
    after: Option<String>,
    before: Option<String>,
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

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}", sanitized)
    })?;

    // Build composite filter
    let mut filters: Vec<Box<dyn EventFilter>> = Vec::new();

    if let Some(et) = event_type {
        filters.push(Box::new(EventTypeFilter { event_type: et }));
    }

    if let Some(pid) = principal {
        filters.push(Box::new(PrincipalFilter { principal_id: pid }));
    }

    if after.is_some() || before.is_some() {
        let after_ts = after
            .map(|s| Timestamp::parse(s).map_err(|e| format!("Invalid 'after' timestamp: {}", e)))
            .transpose()?;
        let before_ts = before
            .map(|s| Timestamp::parse(s).map_err(|e| format!("Invalid 'before' timestamp: {}", e)))
            .transpose()?;

        filters.push(Box::new(TimeRangeFilter {
            after: after_ts,
            before: before_ts,
        }));
    }

    // Output header if table format
    if !json {
        output::print_table_header();
    }

    // Apply filters and iterate
    let mut event_count: u64 = 0;
    if filters.is_empty() {
        // No filters, use reader directly
        let mut reader = reader;
        while let Some(event) = reader.read_next()? {
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
    } else {
        // Apply filters - combine with AND
        let and_filter = AndFilter { filters };
        let mut filtered = FilteredReader::new(reader, and_filter);
        while let Some(event) = filtered.read_next()? {
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
    }

    Ok(())
}
