//! Verify command implementation.

use crate::path;
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_journal::{JournalReader, ReadMode, verify_event_id};
use serde_json::json;

pub fn run(
    journal: String,
    strict: bool,
    json_output: bool,
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

    let profile = ProfileId::parse("northroot-canonical-v1")
        .map_err(|e| format!("Invalid profile ID: {}", e))?;
    let canonicalizer = Canonicalizer::new(profile);

    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).map_err(|e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}: {}", sanitized, e)
    })?;

    let mut all_ok = true;
    let mut results = Vec::new();
    let mut event_count: u64 = 0;

    while let Some(event) = reader.read_event()? {
        // Check max_events limit
        if let Some(max) = max_events {
            if event_count >= max {
                break;
            }
        }
        event_count += 1;

        let event_id_str = event
            .get("event_id")
            .and_then(|v| v.get("b64"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();

        match verify_event_id(&event, &canonicalizer) {
            Ok(true) => {
                results.push((event_id_str.clone(), true, None));
            }
            Ok(false) => {
                all_ok = false;
                results.push((event_id_str.clone(), false, Some("event_id mismatch".to_string())));
            }
            Err(e) => {
                all_ok = false;
                results.push((event_id_str.clone(), false, Some(e.to_string())));
            }
        }
    }

    // Output results
    if json_output {
        let json_results: Vec<_> = results
            .into_iter()
            .map(|(id, valid, error)| {
                json!({
                    "event_id": id,
                    "valid": valid,
                    "error": error
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        println!("{:<44} {:<10} {}", "EVENT_ID", "VALID", "ERROR");
        println!("{}", "-".repeat(80));
        for (id, valid, error_opt) in results {
            let error_str = error_opt.as_deref().unwrap_or("");
            println!("{:<44} {:<10} {}", truncate(&id, 44), if valid { "✓" } else { "✗" }, error_str);
        }
    }

    if strict && !all_ok {
        std::process::exit(1);
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
