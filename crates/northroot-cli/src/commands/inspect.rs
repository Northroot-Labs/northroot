//! Inspect command implementation.

use crate::path;
use northroot_canonical::{Digest, DigestAlg};
use northroot_store::{executions_for_auth, resolve_auth, JournalBackendReader, ReadMode};
use serde_json::json;

pub fn run(journal: String, auth_id: String) -> Result<(), Box<dyn std::error::Error>> {
    // Validate and normalize journal path
    let journal_path = path::validate_journal_path(&journal, false)
        .map_err(|e| format!("Invalid journal path: {}", e))?;

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
        let sanitized = path::sanitize_path_for_error(&journal_path);
        format!("Failed to open journal file: {}", sanitized)
    })?;

    // Parse auth event ID
    let digest = Digest::new(DigestAlg::Sha256, auth_id)
        .map_err(|e| format!("Invalid auth event ID: {}", e))?;

    // Resolve authorization
    let auth = match resolve_auth(&mut reader, &digest)? {
        Some(a) => a,
        None => {
            eprintln!("Authorization not found");
            std::process::exit(1);
        }
    };

    // Get linked executions
    let mut reader2 =
        JournalBackendReader::open(&journal_path, ReadMode::Strict).map_err(|_e| {
            let sanitized = path::sanitize_path_for_error(&journal_path);
            format!("Failed to open journal file: {}", sanitized)
        })?;
    let executions = executions_for_auth(&mut reader2, &digest)?;

    // Output structured view
    let output = json!({
        "authorization": {
            "event_id": auth.event_id.b64,
            "event_type": auth.event_type,
            "occurred_at": auth.occurred_at.as_ref(),
            "principal_id": auth.principal_id.as_ref(),
            "decision": format!("{:?}", auth.decision),
            "decision_code": auth.decision_code,
            "policy_id": auth.policy_id,
        },
        "executions": executions.iter().map(|e| json!({
            "event_id": e.event_id.b64,
            "occurred_at": e.occurred_at.as_ref(),
            "tool_name": e.tool_name.as_ref(),
            "outcome": format!("{:?}", e.outcome),
            "meter_used": e.meter_used.iter().map(|m| json!({
                "unit": m.unit,
                "amount": format!("{:?}", m.amount)
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "execution_count": executions.len()
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
