//! Verify command implementation.

use crate::path;
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_core::{VerificationVerdict, Verifier};
use northroot_store::{parse_event, JournalBackendReader, ReadMode, StoreReader, TypedEvent};
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

    let _reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}", sanitized)
    })?;

    let profile = ProfileId::parse("northroot-canonical-v1")
        .map_err(|e| format!("Invalid profile ID: {}", e))?;
    let canonicalizer = Canonicalizer::new(profile);
    let verifier = Verifier::new(canonicalizer);

    let mut all_ok = true;
    let mut results = Vec::new();

    // First pass: build auth map only (streaming, O(n) auth events, not all events)
    let mut auth_map =
        std::collections::HashMap::<String, northroot_core::AuthorizationEvent>::new();
    let mut event_count: u64 = 0;

    let mut temp_reader =
        JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
            let sanitized = path::sanitize_path_for_error(&journal_path);
            format!("Failed to open journal file: {}", sanitized)
        })?;
    while let Some(event_json) = temp_reader.read_next()? {
        // Check max_events limit
        if let Some(max) = max_events {
            if event_count >= max {
                break;
            }
        }

        if let Ok(TypedEvent::Authorization(auth)) = parse_event(&event_json) {
            auth_map.insert(auth.event_id.b64.clone(), auth);
        }
        event_count += 1;
    }

    // Second pass: stream events, verify, and emit results immediately
    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}", sanitized)
    })?;
    event_count = 0;
    while let Some(event_json) = reader.read_next()? {
        // Check max_events limit
        if let Some(max) = max_events {
            if event_count >= max {
                break;
            }
        }
        event_count += 1;
        let event_id_str = event_json
            .get("event_id")
            .and_then(|v| v.get("b64"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let event_id_str_clone = event_id_str.clone();

        match parse_event(&event_json)? {
            TypedEvent::Authorization(auth) => match verifier.verify_authorization(&auth) {
                Ok((_, verdict)) => {
                    all_ok = all_ok && verdict == VerificationVerdict::Ok;
                    results.push((event_id_str, "authorization".to_string(), verdict));
                }
                Err(e) => {
                    all_ok = false;
                    results.push((
                        event_id_str,
                        "authorization".to_string(),
                        VerificationVerdict::Invalid,
                    ));
                    if !json_output {
                        eprintln!(
                            "Error verifying authorization {}: {}",
                            event_id_str_clone, e
                        );
                    }
                }
            },
            TypedEvent::Execution(exec) => {
                // Find linked authorization
                let auth = auth_map
                    .get(&exec.auth_event_id.b64)
                    .ok_or_else(|| format!("Authorization {} not found", exec.auth_event_id.b64))?;

                match verifier.verify_execution(&exec, auth, None) {
                    Ok((_, verdict)) => {
                        all_ok = all_ok && verdict == VerificationVerdict::Ok;
                        results.push((event_id_str, "execution".to_string(), verdict));
                    }
                    Err(e) => {
                        all_ok = false;
                        results.push((
                            event_id_str,
                            "execution".to_string(),
                            VerificationVerdict::Invalid,
                        ));
                        if !json_output {
                            eprintln!("Error verifying execution {}: {}", event_id_str_clone, e);
                        }
                    }
                }
            }
            TypedEvent::Checkpoint(check) => match verifier.verify_checkpoint(&check) {
                Ok((_, verdict)) => {
                    all_ok = all_ok && verdict == VerificationVerdict::Ok;
                    results.push((event_id_str, "checkpoint".to_string(), verdict));
                }
                Err(e) => {
                    all_ok = false;
                    results.push((
                        event_id_str,
                        "checkpoint".to_string(),
                        VerificationVerdict::Invalid,
                    ));
                    if !json_output {
                        eprintln!("Error verifying checkpoint {}: {}", event_id_str_clone, e);
                    }
                }
            },
            TypedEvent::Attestation(attest) => match verifier.verify_attestation(&attest) {
                Ok((_, verdict)) => {
                    all_ok = all_ok && verdict == VerificationVerdict::Ok;
                    results.push((event_id_str, "attestation".to_string(), verdict));
                }
                Err(e) => {
                    all_ok = false;
                    results.push((
                        event_id_str,
                        "attestation".to_string(),
                        VerificationVerdict::Invalid,
                    ));
                    if !json_output {
                        eprintln!("Error verifying attestation {}: {}", event_id_str_clone, e);
                    }
                }
            },
            TypedEvent::Unknown(_) => {
                if !json_output {
                    eprintln!("Unknown event type: {}", event_id_str_clone);
                }
            }
        }
    }

    // Output results
    if json_output {
        let json_results: Vec<_> = results
            .into_iter()
            .map(|(id, ty, verdict)| {
                json!({
                    "event_id": id,
                    "event_type": ty,
                    "verdict": format!("{:?}", verdict)
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        #[allow(clippy::print_literal)]
        {
            println!("{:<44} {:<15} {}", "EVENT_ID", "TYPE", "VERDICT");
        }
        println!("{}", "-".repeat(70));
        for (id, ty, verdict) in results {
            println!("{:<44} {:<15} {:?}", truncate(&id, 44), ty, verdict);
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
