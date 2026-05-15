//! Primitive validation, hashing, and receipt verification commands.

use northroot_actor::Actor;
use northroot_core::{
    canonical_sha256, file_sha256, jsonl_sha256, read_json, NorthrootError, ValidatePrimitive,
};
use northroot_cost::CostAttribution;
use northroot_event::ExecutionEvent;
use northroot_obligation::Obligation;
use northroot_policy::Policy;
use northroot_receipt::Receipt;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub fn validate(input: String) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(&input);
    let value = read_json(&path)?;
    let kind = validate_value(value)?;
    println!("validated {} {}", kind, input);
    Ok(())
}

pub fn hash(input: String) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(&input);
    let digest = hash_path(&path)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "path": input,
            "hash": digest
        }))?
    );
    Ok(())
}

pub fn verify_receipt(
    input: String,
    base_dir: Option<String>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let receipt_path = PathBuf::from(&input);
    let receipt: Receipt = serde_json::from_value(read_json(&receipt_path)?)?;
    receipt.validate()?;

    let base = base_dir
        .map(PathBuf::from)
        .or_else(|| receipt_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut checked = Vec::new();
    let mut all_ok = true;
    for evidence in &receipt.evidence {
        if let Some(path) = evidence.uri.strip_prefix("file://") {
            let resolved = base.join(path);
            let actual = hash_path(&resolved)?;
            let valid = actual == evidence.hash;
            all_ok &= valid;
            checked.push(json!({
                "kind": evidence.kind,
                "uri": evidence.uri,
                "expected": evidence.hash,
                "actual": actual,
                "valid": valid
            }));
        } else {
            checked.push(json!({
                "kind": evidence.kind,
                "uri": evidence.uri,
                "expected": evidence.hash,
                "actual": null,
                "valid": null
            }));
        }
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "receipt_id": receipt.receipt_id,
                "obligation_id": receipt.obligation_id,
                "claim": receipt.claim,
                "result": receipt.result,
                "valid": all_ok,
                "evidence": checked
            }))?
        );
    } else {
        println!("receipt: {}", receipt.receipt_id);
        println!("obligation: {}", receipt.obligation_id);
        println!("claim: {}", receipt.claim);
        println!("valid: {}", all_ok);
        for item in checked {
            println!(
                "evidence {} {}",
                item["kind"].as_str().unwrap_or("?"),
                item["valid"]
            );
        }
    }

    if !all_ok {
        return Err("receipt evidence verification failed".into());
    }
    Ok(())
}

fn validate_value(value: Value) -> Result<&'static str, Box<dyn std::error::Error>> {
    if value.get("authority_scope").is_some() && value.get("display_name").is_some() {
        let primitive: Actor = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("actor");
    }
    if value.get("objective").is_some() && value.get("verification").is_some() {
        let primitive: Obligation = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("obligation");
    }
    if value.get("rules").is_some() && value.get("applies_to").is_some() {
        let primitive: Policy = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("policy");
    }
    if value.get("event_type").is_some() && value.get("obligation_id").is_some() {
        let primitive: ExecutionEvent = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("execution_event");
    }
    if value.get("receipt_id").is_some() && value.get("evidence").is_some() {
        let primitive: Receipt = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("receipt");
    }
    if value.get("provider").is_some() && value.get("attributed_to").is_some() {
        let primitive: CostAttribution = serde_json::from_value(value)?;
        primitive.validate()?;
        return Ok("cost_attribution");
    }
    if value.get("files").is_some() && value.get("bundle").is_some() {
        return Ok("manifest");
    }
    Err(Box::new(NorthrootError::InvalidValue {
        field: "primitive",
        value: "unrecognized JSON shape".to_string(),
    }))
}

fn hash_path(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if path.is_dir() {
        let mut entries = Vec::new();
        collect_files(path, path, &mut entries)?;
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let manifest = json!({
            "bundle": path.file_name().and_then(|name| name.to_str()).unwrap_or("."),
            "files": entries
                .into_iter()
                .map(|(relative_path, hash)| json!({
                    "path": relative_path,
                    "hash": hash
                }))
                .collect::<Vec<_>>()
        });
        return Ok(canonical_sha256(&manifest)?);
    }

    if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
        || path.extension().and_then(|ext| ext.to_str()) == Some("ndjson")
    {
        return Ok(jsonl_sha256(&fs::read_to_string(path)?)?);
    }
    Ok(file_sha256(path)?)
}

fn collect_files(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, entries)?;
        } else {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push((relative, hash_path(&path)?));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn jsonl_hash_is_format_stable() {
        let first = jsonl_sha256("{\"b\":2,\"a\":1}\n").unwrap();
        let second = jsonl_sha256("{\"a\":1,\"b\":2}\n").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn receipt_verifies_file_evidence() {
        let temp = tempfile::TempDir::new().unwrap();
        let events = temp.path().join("events.jsonl");
        let mut file = fs::File::create(&events).unwrap();
        writeln!(file, "{{\"event_type\":\"x\",\"obligation_id\":\"obl_demo\",\"actor_id\":\"actor:northroot:agent:repo_inspector\",\"event_version\":\"1\",\"timestamp\":\"2026-05-15T12:00:00Z\"}}").unwrap();
        let hash = hash_path(&events).unwrap();
        let receipt = json!({
            "schema_version": "northroot.receipt.v0",
            "receipt_id": "rcpt_demo",
            "obligation_id": "obl_demo",
            "claim": "demo",
            "actor_id": "actor:northroot:agent:repo_inspector",
            "evidence": [{"kind": "event_log", "uri": "file://events.jsonl", "hash": hash}],
            "result": "passed",
            "created_at": "2026-05-15T12:01:00Z"
        });
        let receipt_path = temp.path().join("receipt.json");
        fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt).unwrap(),
        )
        .unwrap();
        verify_receipt(receipt_path.display().to_string(), None, true).unwrap();
    }
}
