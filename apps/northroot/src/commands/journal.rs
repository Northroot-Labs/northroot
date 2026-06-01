//! Structural journal command implementation.

use clap::Subcommand;
use northroot_canonical::{compute_blob_digest, Canonicalizer, Digest, ProfileId};
use northroot_journal::{verify_event_id, JournalReader, ReadMode};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const CANONICAL_PROFILE_ID: &str = "northroot-canonical-v1";
const MANIFEST_SCHEMA: &str = "northroot.segmented_journal_manifest.v0";
const CHECKPOINT_SCHEMA: &str = "northroot.journal_checkpoint.v0";

/// Structural journal subcommands.
#[derive(Subcommand)]
pub enum JournalCommand {
    /// Verify an ordered set of .nrj segments and emit a structural manifest
    VerifySegments {
        /// Directory containing .nrj files or a segments/ subdirectory
        #[arg(long)]
        dir: String,
    },
    /// Emit a regenerable structural manifest for .nrj segments
    Manifest {
        /// Directory containing .nrj files or a segments/ subdirectory
        #[arg(long)]
        dir: String,
        /// Output path, or '-' for stdout
        #[arg(long, default_value = "-")]
        out: String,
    },
    /// Emit a structural checkpoint at the verified journal tip
    Checkpoint {
        /// Directory containing .nrj files or a segments/ subdirectory
        #[arg(long)]
        dir: String,
        /// Output path, or '-' for stdout
        #[arg(long, default_value = "-")]
        out: String,
    },
}

/// Runs a structural journal subcommand.
pub fn run(command: JournalCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        JournalCommand::VerifySegments { dir } => {
            let report = verify_segments(&PathBuf::from(dir))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report["valid"].as_bool().unwrap_or(false) {
                return Err("segmented journal verification failed".into());
            }
            Ok(())
        }
        JournalCommand::Manifest { dir, out } => {
            let report = verify_segments(&PathBuf::from(dir))?;
            if !report["valid"].as_bool().unwrap_or(false) {
                return Err("cannot emit manifest for invalid segmented journal".into());
            }
            let manifest = report
                .get("manifest")
                .cloned()
                .ok_or("segmented journal report missing manifest")?;
            write_json_output(&manifest, &out)
        }
        JournalCommand::Checkpoint { dir, out } => {
            let report = verify_segments(&PathBuf::from(dir))?;
            if !report["valid"].as_bool().unwrap_or(false) {
                return Err("cannot checkpoint invalid segmented journal".into());
            }
            let checkpoint = checkpoint_from_report(&report)?;
            write_json_output(&checkpoint, &out)
        }
    }
}

#[derive(Debug, Serialize)]
struct SegmentReport {
    segment_ordinal: u64,
    path: String,
    first_event_ordinal: Option<u64>,
    last_event_ordinal: Option<u64>,
    event_count: u64,
    kernel_valid_event_count: u64,
    invalid_event_count: u64,
    verified_prefix_event_count: u64,
    first_event_id: Option<Value>,
    last_event_id: Option<Value>,
    verified_prefix_last_event_id: Option<Value>,
    byte_len: u64,
    digest: Value,
    valid: bool,
    error: Option<String>,
}

fn verify_segments(dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let root = existing_dir(dir)?;
    let segment_dir = if root.join("segments").is_dir() {
        root.join("segments")
    } else {
        root.clone()
    };
    let segment_paths = collect_segments(&segment_dir)?;
    let canonicalizer = canonicalizer()?;

    let mut segments = Vec::new();
    let mut event_count = 0u64;
    let mut kernel_valid_event_count = 0u64;
    let mut invalid_event_count = 0u64;
    let mut verified_prefix_event_count = 0u64;
    let mut prefix_closed = false;
    let mut tip_event_id = None;
    let mut verified_prefix_tip_event_id = None;

    for (index, path) in segment_paths.iter().enumerate() {
        let segment = verify_segment(
            &root,
            path,
            index as u64 + 1,
            event_count + 1,
            &canonicalizer,
        )?;
        event_count += segment.event_count;
        kernel_valid_event_count += segment.kernel_valid_event_count;
        invalid_event_count += segment.invalid_event_count;
        if !prefix_closed {
            verified_prefix_event_count += segment.verified_prefix_event_count;
            verified_prefix_tip_event_id = segment.verified_prefix_last_event_id.clone();
            if !segment.valid || segment.verified_prefix_event_count != segment.event_count {
                prefix_closed = true;
            }
        }
        if let Some(last_event_id) = segment.last_event_id.clone() {
            tip_event_id = Some(last_event_id);
        }
        segments.push(segment);
    }

    let valid = !segments.is_empty() && invalid_event_count == 0 && segments.iter().all(|s| s.valid);
    let segment_values = serde_json::to_value(&segments)?;
    let prefix_digest = digest_value(&json!({
        "schema": "northroot.segmented_journal_prefix.v0",
        "segments": segment_values,
        "event_count": event_count
    }))?;
    let manifest = json!({
        "schema": MANIFEST_SCHEMA,
        "journal_dir": root.display().to_string(),
        "segment_dir": segment_dir.strip_prefix(&root).unwrap_or(&segment_dir).display().to_string(),
        "segment_count": segments.len(),
        "event_count": event_count,
        "kernel_valid_event_count": kernel_valid_event_count,
        "invalid_event_count": invalid_event_count,
        "verified_prefix_event_count": verified_prefix_event_count,
        "verified_prefix_tip_event_id": verified_prefix_tip_event_id,
        "tip_event_id": tip_event_id,
        "prefix_digest": digest_ref(&prefix_digest),
        "segments": segments
    });

    Ok(json!({
        "schema": "northroot.segmented_journal_verification.v0",
        "valid": valid,
        "manifest": manifest
    }))
}

fn verify_segment(
    root: &Path,
    path: &Path,
    segment_ordinal: u64,
    next_event_ordinal: u64,
    canonicalizer: &Canonicalizer,
) -> Result<SegmentReport, Box<dyn std::error::Error>> {
    let byte_len = fs::metadata(path)?.len();
    let digest = file_digest(path)?;
    let relative_path = path.strip_prefix(root).unwrap_or(path).display().to_string();
    let mut reader = match JournalReader::open(path, ReadMode::Strict) {
        Ok(reader) => reader,
        Err(err) => {
            return Ok(SegmentReport {
                segment_ordinal,
                path: relative_path,
                first_event_ordinal: None,
                last_event_ordinal: None,
                event_count: 0,
                kernel_valid_event_count: 0,
                invalid_event_count: 1,
                verified_prefix_event_count: 0,
                first_event_id: None,
                last_event_id: None,
                verified_prefix_last_event_id: None,
                byte_len,
                digest: digest_ref(&digest),
                valid: false,
                error: Some(err.to_string()),
            });
        }
    };

    let mut event_count = 0u64;
    let mut kernel_valid_event_count = 0u64;
    let mut invalid_event_count = 0u64;
    let mut verified_prefix_event_count = 0u64;
    let mut prefix_closed = false;
    let mut first_event_id = None;
    let mut last_event_id = None;
    let mut verified_prefix_last_event_id = None;
    let mut error = None;

    loop {
        match reader.read_event() {
            Ok(Some(event)) => {
                event_count += 1;
                let event_id = event.get("event_id").cloned().unwrap_or(Value::Null);
                if first_event_id.is_none() {
                    first_event_id = Some(event_id.clone());
                }
                last_event_id = Some(event_id);
                match verify_event_id(&event, canonicalizer) {
                    Ok(true) => {
                        kernel_valid_event_count += 1;
                        if !prefix_closed {
                            verified_prefix_event_count += 1;
                            verified_prefix_last_event_id = last_event_id.clone();
                        }
                    }
                    Ok(false) => {
                        invalid_event_count += 1;
                        prefix_closed = true;
                    }
                    Err(err) => {
                        invalid_event_count += 1;
                        prefix_closed = true;
                        if error.is_none() {
                            error = Some(err.to_string());
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(err) => {
                invalid_event_count += 1;
                error = Some(err.to_string());
                break;
            }
        }
    }

    let first_event_ordinal = if event_count == 0 {
        None
    } else {
        Some(next_event_ordinal)
    };
    let last_event_ordinal = if event_count == 0 {
        None
    } else {
        Some(next_event_ordinal + event_count - 1)
    };

    Ok(SegmentReport {
        segment_ordinal,
        path: relative_path,
        first_event_ordinal,
        last_event_ordinal,
        event_count,
        kernel_valid_event_count,
        invalid_event_count,
        verified_prefix_event_count,
        first_event_id,
        last_event_id,
        verified_prefix_last_event_id,
        byte_len,
        digest: digest_ref(&digest),
        valid: invalid_event_count == 0,
        error,
    })
}

fn checkpoint_from_report(report: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let manifest = report
        .get("manifest")
        .ok_or("segmented journal report missing manifest")?;
    let segments = manifest
        .get("segments")
        .and_then(Value::as_array)
        .ok_or("segmented journal manifest missing segments")?;
    let last_segment = segments.last();
    let frame_position = last_segment
        .map(|segment| {
            json!({
                "segment_ordinal": segment.get("segment_ordinal").cloned().unwrap_or(Value::Null),
                "byte_offset": segment.get("byte_len").cloned().unwrap_or(Value::Null)
            })
        })
        .unwrap_or(Value::Null);
    let mut checkpoint = json!({
        "schema": CHECKPOINT_SCHEMA,
        "journal_dir": manifest.get("journal_dir").cloned().unwrap_or(Value::Null),
        "segment_count": manifest.get("segment_count").cloned().unwrap_or(Value::Null),
        "event_ordinal": manifest.get("event_count").cloned().unwrap_or(Value::Null),
        "verified_prefix_event_ordinal": manifest.get("verified_prefix_event_count").cloned().unwrap_or(Value::Null),
        "frame_position": frame_position,
        "tip_event_id": manifest.get("tip_event_id").cloned().unwrap_or(Value::Null),
        "verified_prefix_tip_event_id": manifest.get("verified_prefix_tip_event_id").cloned().unwrap_or(Value::Null),
        "prefix_digest": manifest.get("prefix_digest").cloned().unwrap_or(Value::Null)
    });
    let checkpoint_digest = digest_value(&checkpoint)?;
    checkpoint["checkpoint_id"] = digest_ref(&checkpoint_digest);
    Ok(checkpoint)
}

fn collect_segments(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut segments = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "nrj") {
            segments.push(path);
        }
    }
    segments.sort();
    Ok(segments)
}

fn existing_dir(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let canonical = path.canonicalize()?;
    if !canonical.is_dir() {
        return Err(format!("not a directory: {}", path.display()).into());
    }
    Ok(canonical)
}

fn write_json_output(value: &Value, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rendered = serde_json::to_string_pretty(value)?;
    if out == "-" {
        println!("{rendered}");
    } else {
        let path = PathBuf::from(out);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, format!("{rendered}\n"))?;
    }
    Ok(())
}

fn digest_value(value: &Value) -> Result<Digest, Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let canonical = canonicalizer.canonicalize(value)?;
    Ok(compute_blob_digest(&canonical.bytes)?)
}

fn file_digest(path: &Path) -> Result<Digest, Box<dyn std::error::Error>> {
    Ok(compute_blob_digest(&fs::read(path)?)?)
}

fn digest_ref(digest: &Digest) -> Value {
    serde_json::to_value(digest).expect("digest serializes")
}

fn canonicalizer() -> Result<Canonicalizer, Box<dyn std::error::Error>> {
    let profile = ProfileId::parse(CANONICAL_PROFILE_ID)?;
    Ok(Canonicalizer::new(profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_canonical::compute_event_id;
    use northroot_journal::{JournalWriter, WriteOptions};
    use serde_json::json;
    use tempfile::TempDir;

    fn signed_event(name: &str) -> Value {
        let canonicalizer = canonicalizer().unwrap();
        let mut event = json!({
            "event_type": name,
            "event_version": "1",
            "occurred_at": "2026-06-01T00:00:00Z",
            "principal_id": "service:test",
            "canonical_profile_id": CANONICAL_PROFILE_ID
        });
        let event_id = compute_event_id(&event, &canonicalizer).unwrap();
        event["event_id"] = serde_json::to_value(event_id).unwrap();
        event
    }

    fn write_segment(path: &Path, events: &[Value]) {
        let mut writer = JournalWriter::open(path, WriteOptions::default()).unwrap();
        for event in events {
            writer.append_event(event).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn segmented_manifest_and_checkpoint_are_structural() {
        let temp = TempDir::new().unwrap();
        let segments = temp.path().join("segments");
        fs::create_dir(&segments).unwrap();
        write_segment(&segments.join("0000000000000001.nrj"), &[signed_event("a")]);
        write_segment(
            &segments.join("0000000000000002.nrj"),
            &[signed_event("b"), signed_event("c")],
        );

        let report = verify_segments(temp.path()).unwrap();
        assert_eq!(report["valid"], true);
        assert_eq!(report["manifest"]["segment_count"], 2);
        assert_eq!(report["manifest"]["event_count"], 3);
        assert_eq!(report["manifest"]["verified_prefix_event_count"], 3);
        assert_eq!(
            report["manifest"]["segments"][1]["first_event_ordinal"],
            2
        );
        assert_eq!(report["manifest"]["segments"][1]["last_event_ordinal"], 3);

        let checkpoint = checkpoint_from_report(&report).unwrap();
        assert_eq!(checkpoint["schema"], CHECKPOINT_SCHEMA);
        assert_eq!(checkpoint["event_ordinal"], 3);
        assert_eq!(checkpoint["verified_prefix_event_ordinal"], 3);
        assert_eq!(checkpoint["frame_position"]["segment_ordinal"], 2);
        assert!(checkpoint.get("checkpoint_id").is_some());
    }

    #[test]
    fn segmented_verify_rejects_invalid_event_identity() {
        let temp = TempDir::new().unwrap();
        let segments = temp.path().join("segments");
        fs::create_dir(&segments).unwrap();
        let mut event = signed_event("a");
        event["event_type"] = Value::String("tampered".to_string());
        write_segment(&segments.join("0000000000000001.nrj"), &[event]);

        let report = verify_segments(temp.path()).unwrap();
        assert_eq!(report["valid"], false);
        assert_eq!(report["manifest"]["invalid_event_count"], 1);
        assert_eq!(report["manifest"]["verified_prefix_event_count"], 0);
    }
}
