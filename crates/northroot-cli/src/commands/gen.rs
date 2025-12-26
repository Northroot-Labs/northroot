//! Generate command implementation.

use northroot_canonical::Timestamp;
use northroot_store::{JournalBackendWriter, StoreWriter, WriteOptions};
use serde_json::json;
use sha2::{Digest as Sha2Digest, Sha256};
use std::path::Path;

/// Generate a deterministic digest from seed, index, and type.
fn derive_digest(seed: u64, index: u32, event_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.to_le_bytes());
    hasher.update(index.to_le_bytes());
    hasher.update(event_type.as_bytes());
    let hash_bytes = hasher.finalize();

    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash_bytes)
}

/// Build a valid authorization event.
fn build_auth(index: u32, seed: u64, timestamp: &str) -> serde_json::Value {
    let b64 = derive_digest(seed, index, "authorization");

    json!({
        "event_id": { "alg": "sha-256", "b64": b64 },
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": timestamp,
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": b64 }
        },
        "policy_id": "test-policy",
        "policy_digest": { "alg": "sha-256", "b64": b64 },
        "decision": "allow",
        "decision_code": "ALLOW",
        "authorization": {
            "kind": "grant",
            "bounds": {
                "allowed_tools": ["test.tool"],
                "meter_caps": []
            }
        }
    })
}

/// Build a valid execution event referencing an auth.
fn build_exec_ok(index: u32, seed: u64, timestamp: &str, auth_id: &str) -> serde_json::Value {
    let b64_id = derive_digest(seed, index, "execution");

    json!({
        "event_id": { "alg": "sha-256", "b64": b64_id },
        "event_type": "execution",
        "event_version": "1",
        "occurred_at": timestamp,
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": b64_id }
        },
        "auth_event_id": { "alg": "sha-256", "b64": auth_id },
        "tool_name": "test.tool",
        "meter_used": [],
        "outcome": "success"
    })
}

/// Build an execution event referencing a non-existent auth (for deny testing).
fn build_exec_bad(index: u32, seed: u64, timestamp: &str) -> serde_json::Value {
    let b64_id = derive_digest(seed, index, "execution");
    // Use a digest that won't match any auth (seed + 99999)
    let fake_auth = derive_digest(seed.wrapping_add(99999), 0, "authorization");

    json!({
        "event_id": { "alg": "sha-256", "b64": b64_id },
        "event_type": "execution",
        "event_version": "1",
        "occurred_at": timestamp,
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": b64_id }
        },
        "auth_event_id": { "alg": "sha-256", "b64": fake_auth },
        "tool_name": "test.tool",
        "meter_used": [],
        "outcome": "success"
    })
}

/// Build a malformed event (missing required field).
fn build_malformed() -> serde_json::Value {
    json!({
        "event_id": { "alg": "sha-256", "b64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" },
        "event_type": "authorization",
        // Missing event_version, occurred_at, etc.
    })
}

/// Add milliseconds to an RFC3339 timestamp string.
/// Simple implementation: adds seconds (ms / 1000) to the timestamp.
fn add_milliseconds(ts: &str, ms: u64) -> Result<String, Box<dyn std::error::Error>> {
    let seconds_to_add = ms / 1000;
    if seconds_to_add == 0 {
        return Ok(ts.to_string());
    }

    // Parse timestamp - format is YYYY-MM-DDTHH:MM:SS[.fraction]Z
    let ts_str = ts.trim_end_matches('Z');
    let (base, fraction) = if let Some(dot_pos) = ts_str.find('.') {
        (ts_str[..dot_pos].to_string(), Some(&ts_str[dot_pos + 1..]))
    } else {
        (ts_str.to_string(), None)
    };

    // Parse base time: YYYY-MM-DDTHH:MM:SS
    let parts: Vec<&str> = base.split('T').collect();
    if parts.len() != 2 {
        return Err("Invalid timestamp format".into());
    }

    let date_part = parts[0];
    let time_part = parts[1];
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() != 3 {
        return Err("Invalid time format".into());
    }

    let mut hour: u64 = time_parts[0].parse()?;
    let mut min: u64 = time_parts[1].parse()?;
    let mut sec: u64 = time_parts[2].parse()?;

    // Add seconds
    sec += seconds_to_add;
    min += sec / 60;
    sec %= 60;
    hour += min / 60;
    min %= 60;
    // Note: we don't handle day overflow for simplicity

    // Reconstruct timestamp
    let new_time = format!("{:02}:{:02}:{:02}", hour, min, sec);
    let new_ts = if let Some(frac) = fraction {
        format!("{}T{}.{}Z", date_part, new_time, frac)
    } else {
        format!("{}T{}Z", date_part, new_time)
    };

    Ok(new_ts)
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    output: String,
    seed: u64,
    count_auth: u32,
    count_exec_ok: u32,
    count_exec_bad: u32,
    start_ts: String,
    ts_step_ms: u64,
    with_bad: bool,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if output file exists
    if Path::new(&output).exists() && !force {
        return Err(format!("File {} already exists. Use --force to overwrite", output).into());
    }

    // Validate start timestamp
    Timestamp::parse(&start_ts).map_err(|e| format!("Invalid start timestamp: {}", e))?;

    // Open writer
    let mut writer = JournalBackendWriter::open(&output, WriteOptions::default())
        .map_err(|e| format!("Failed to open journal for writing: {}", e))?;

    let mut event_count = 0;
    let mut current_ts = start_ts.clone();
    let mut auth_ids = Vec::new();

    // Generate auth events
    for i in 0..count_auth {
        let event = build_auth(i, seed, &current_ts);
        let auth_id = event["event_id"]["b64"].as_str().unwrap().to_string();
        auth_ids.push(auth_id.clone());
        writer.append(&event)?;
        event_count += 1;

        // Advance timestamp for next event
        current_ts = add_milliseconds(&current_ts, ts_step_ms)?;
    }

    // Generate valid execution events (paired with auths)
    for i in 0..count_exec_ok {
        let auth_idx = (i as usize) % auth_ids.len();
        let auth_id = &auth_ids[auth_idx];

        let event = build_exec_ok(i, seed, &current_ts, auth_id);
        writer.append(&event)?;
        event_count += 1;

        // Advance timestamp for next event
        current_ts = add_milliseconds(&current_ts, ts_step_ms)?;
    }

    // Generate bad execution events (orphan references)
    for i in 0..count_exec_bad {
        let event = build_exec_bad(count_exec_ok + i, seed, &current_ts);
        writer.append(&event)?;
        event_count += 1;

        // Advance timestamp for next event
        current_ts = add_milliseconds(&current_ts, ts_step_ms)?;
    }

    // Add malformed event if requested
    if with_bad {
        let event = build_malformed();
        writer.append(&event)?;
        event_count += 1;
    }

    writer.finish()?;

    println!("Generated {} events to {}", event_count, output);
    Ok(())
}
