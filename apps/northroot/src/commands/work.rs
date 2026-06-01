//! Work ledger command implementation.

use crate::path;
use clap::Subcommand;
use northroot_canonical::{
    compute_blob_digest, compute_event_id, Canonicalizer, Digest, ProfileId,
};
use northroot_journal::{verify_event_id, JournalReader, JournalWriter, ReadMode, WriteOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

const CANONICAL_PROFILE_ID: &str = "northroot-canonical-v1";
const WORK_LEDGER_SCHEMA: &str = "northroot.work_ledger.v0";
const SNAPSHOT_SCHEMA: &str = "northroot.workspace_snapshot.v0";
const SNAPSHOT_MANIFEST_SCHEMA: &str = "northroot.snapshot_manifest.v0";
const SNAPSHOT_LINEAGE_SCHEMA: &str = "northroot.snapshot_lineage.v0";
const SNAPSHOT_DOMAIN_SEPARATOR: &[u8] = b"northroot:snapshot:v0\0";
const MISSING_TIMESTAMP_SENTINEL: &str = "1970-01-01T00:00:00Z";

/// Work ledger subcommands.
#[derive(Subcommand)]
pub enum WorkCommand {
    /// Ingest local Codex session JSONL into a Northroot work ledger
    IngestCodex {
        /// File or directory containing Codex session JSONL files
        #[arg(long)]
        sessions: String,
        /// Target .nrj journal path
        #[arg(long, default_value = ".northroot/work-ledger.nrj")]
        journal: String,
        /// Maximum number of new events to append
        #[arg(long)]
        max_events: Option<usize>,
        /// Include redacted text previews in artifact events
        #[arg(long, default_value_t = false)]
        include_text: bool,
        /// Stop on malformed source records
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Append duplicate events instead of skipping known event_ids
        #[arg(long, default_value_t = false)]
        no_dedupe: bool,
        /// Sync journal data to disk after each append
        #[arg(long, default_value_t = false)]
        sync: bool,
        /// Write malformed source records as redacted JSONL quarantine
        #[arg(long)]
        malformed_out: Option<String>,
    },
    /// Project a readable work/run/evidence summary from a work ledger
    Project {
        /// Source .nrj journal path
        #[arg(long)]
        journal: String,
        /// Output path, or '-' for stdout
        #[arg(long, default_value = "-")]
        out: String,
    },
    /// Verify work ledger event identities
    Verify {
        /// Source .nrj journal path
        #[arg(long)]
        journal: String,
    },
    /// Create, verify, restore, or list work ledger snapshots
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
}

/// Work ledger snapshot subcommands.
#[derive(Subcommand)]
pub enum SnapshotCommand {
    /// Create a content-addressed snapshot from a work ledger
    Create {
        /// Source .nrj journal path
        #[arg(long)]
        journal: String,
        /// Snapshot output directory
        #[arg(long, default_value = ".northroot/snapshots")]
        out: String,
    },
    /// Verify a snapshot manifest and payload against a journal
    Verify {
        /// Snapshot manifest path, digest, sha256:digest, or snapshot:sha256:digest
        #[arg(long)]
        snapshot: String,
        /// Source .nrj journal path
        #[arg(long)]
        journal: String,
    },
    /// Restore a projection from a snapshot and replay remaining journal events
    Restore {
        /// Snapshot manifest path, digest, sha256:digest, or snapshot:sha256:digest
        #[arg(long)]
        snapshot: String,
        /// Source .nrj journal path
        #[arg(long)]
        journal: String,
        /// Output path, or '-' for stdout
        #[arg(long, default_value = ".northroot/projections/work-ledger.json")]
        out: String,
    },
    /// List content-addressed snapshots in a directory
    List {
        /// Snapshot directory
        #[arg(long, default_value = ".northroot/snapshots")]
        dir: String,
    },
}

/// Runs a work ledger subcommand.
pub fn run(command: WorkCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        WorkCommand::IngestCodex {
            sessions,
            journal,
            max_events,
            include_text,
            strict,
            no_dedupe,
            sync,
            malformed_out,
        } => ingest_codex(
            &PathBuf::from(sessions),
            &PathBuf::from(journal),
            IngestOptions {
                max_events,
                include_text,
                strict,
                dedupe: !no_dedupe,
                sync,
                malformed_out: malformed_out.as_deref().map(PathBuf::from),
            },
        ),
        WorkCommand::Project { journal, out } => project(&PathBuf::from(journal), &out),
        WorkCommand::Verify { journal } => verify(&PathBuf::from(journal)),
        WorkCommand::Snapshot { command } => run_snapshot(command),
    }
}

#[derive(Debug, Default)]
struct IngestOptions {
    max_events: Option<usize>,
    include_text: bool,
    strict: bool,
    dedupe: bool,
    sync: bool,
    malformed_out: Option<PathBuf>,
}

fn ingest_codex(
    sessions: &Path,
    journal: &Path,
    options: IngestOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_files = collect_session_files(sessions)?;
    let journal_path = prepare_journal_path(journal)?;
    let canonicalizer = canonicalizer()?;
    let mut existing_event_ids = if options.dedupe {
        read_event_ids(&journal_path)?
    } else {
        HashSet::new()
    };

    let mut writer = JournalWriter::open(
        &journal_path,
        WriteOptions {
            sync: options.sync,
            create: true,
            append: true,
        },
    )?;

    let mut appended = 0usize;
    let mut skipped_duplicates = 0usize;
    let mut malformed_records = 0usize;
    let mut quarantined_records = Vec::new();
    let mut warnings = Vec::new();

    for session_file in session_files {
        if options.max_events.is_some_and(|limit| appended >= limit) {
            break;
        }

        let records = read_source_records(
            &session_file,
            options.strict,
            &mut malformed_records,
            &mut quarantined_records,
            &mut warnings,
        )?;
        if records.is_empty() {
            continue;
        }

        let session_id = discover_session_id(&session_file, &records);
        let first_timestamp = records
            .iter()
            .find_map(|record| record_timestamp(&record.value))
            .unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string());
        let work_id = stable_id("work", &[&session_id])?;
        let work_event = signed_event(
            json!({
                "event_type": "work.observed",
                "event_version": "0",
                "occurred_at": first_timestamp,
                "principal_id": "agent:codex",
                "canonical_profile_id": CANONICAL_PROFILE_ID,
                "schema": WORK_LEDGER_SCHEMA,
                "work_id": work_id,
                "source": source_ref(&session_file, 0),
                "source_system": "codex_session",
                "observation": {
                    "session_id": session_id,
                    "record_count": records.len()
                }
            }),
            &canonicalizer,
        )?;
        append_if_new(
            &mut writer,
            work_event,
            &mut existing_event_ids,
            &mut appended,
            &mut skipped_duplicates,
            options.max_events,
        )?;

        let mut turn_index = 0usize;
        let mut current_run_id = stable_id("run", &[&session_id, "turn:0"])?;
        let mut open_run = false;

        for record in records {
            if options.max_events.is_some_and(|limit| appended >= limit) {
                break;
            }

            let event_type = record
                .value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("");
            if event_type == "turn_context" {
                turn_index += 1;
                current_run_id = stable_id("run", &[&session_id, &format!("turn:{turn_index}")])?;
                open_run = true;
                let run_event = signed_event(
                    json!({
                        "event_type": "run.observed",
                        "event_version": "0",
                        "occurred_at": record_timestamp(&record.value).unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string()),
                        "principal_id": "agent:codex",
                        "canonical_profile_id": CANONICAL_PROFILE_ID,
                        "schema": WORK_LEDGER_SCHEMA,
                        "work_id": work_id,
                        "run_id": current_run_id,
                        "source": source_ref(&session_file, record.line),
                        "source_system": "codex_session",
                        "observation": turn_observation(&record.value)
                    }),
                    &canonicalizer,
                )?;
                append_if_new(
                    &mut writer,
                    run_event,
                    &mut existing_event_ids,
                    &mut appended,
                    &mut skipped_duplicates,
                    options.max_events,
                )?;
                continue;
            }

            if let Some(artifact) =
                artifact_observation(&session_file, &record, options.include_text)?
            {
                let artifact_event = signed_event(
                    json!({
                        "event_type": "artifact.observed",
                        "event_version": "0",
                        "occurred_at": record_timestamp(&record.value).unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string()),
                        "principal_id": "agent:codex",
                        "canonical_profile_id": CANONICAL_PROFILE_ID,
                        "schema": WORK_LEDGER_SCHEMA,
                        "work_id": work_id,
                        "run_id": current_run_id,
                        "source": source_ref(&session_file, record.line),
                        "source_system": "codex_session",
                        "artifact": artifact
                    }),
                    &canonicalizer,
                )?;
                append_if_new(
                    &mut writer,
                    artifact_event,
                    &mut existing_event_ids,
                    &mut appended,
                    &mut skipped_duplicates,
                    options.max_events,
                )?;
            }

            if let Some(status) = terminal_status(&record.value) {
                let terminal_event_type = if status == "blocked" {
                    "run.blocked"
                } else {
                    "run.completed"
                };
                let terminal_event = signed_event(
                    json!({
                        "event_type": terminal_event_type,
                        "event_version": "0",
                        "occurred_at": record_timestamp(&record.value).unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string()),
                        "principal_id": "agent:codex",
                        "canonical_profile_id": CANONICAL_PROFILE_ID,
                        "schema": WORK_LEDGER_SCHEMA,
                        "work_id": work_id,
                        "run_id": current_run_id,
                        "source": source_ref(&session_file, record.line),
                        "source_system": "codex_session",
                        "status": status,
                        "reason": terminal_reason(&record.value)
                    }),
                    &canonicalizer,
                )?;
                append_if_new(
                    &mut writer,
                    terminal_event,
                    &mut existing_event_ids,
                    &mut appended,
                    &mut skipped_duplicates,
                    options.max_events,
                )?;
                open_run = false;
            }
        }

        if open_run && can_append_more(options.max_events, appended) {
            let observed_event = signed_event(
                json!({
                    "event_type": "run.blocked",
                    "event_version": "0",
                    "occurred_at": MISSING_TIMESTAMP_SENTINEL,
                    "principal_id": "agent:codex",
                    "canonical_profile_id": CANONICAL_PROFILE_ID,
                    "schema": WORK_LEDGER_SCHEMA,
                    "work_id": work_id,
                    "run_id": current_run_id,
                    "source": source_ref(&session_file, 0),
                    "source_system": "codex_session",
                    "status": "blocked",
                    "reason": "no terminal record observed"
                }),
                &canonicalizer,
            )?;
            append_if_new(
                &mut writer,
                observed_event,
                &mut existing_event_ids,
                &mut appended,
                &mut skipped_duplicates,
                options.max_events,
            )?;
        }

        if options.max_events.is_some_and(|limit| appended >= limit) {
            warnings.push(format!(
                "max_events reached after {}; session import may be partial",
                session_file.display()
            ));
        }
    }

    writer.finish()?;
    if let Some(path) = options.malformed_out.as_ref() {
        write_malformed_quarantine(path, &quarantined_records)?;
    }

    let summary = json!({
        "journal": journal_path.display().to_string(),
        "events_appended": appended,
        "duplicates_skipped": skipped_duplicates,
        "malformed_records": malformed_records,
        "malformed_quarantine": options.malformed_out.as_ref().map(|path| path.display().to_string()),
        "warnings": warnings
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn project(journal: &Path, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let projection = read_projection(journal)?;
    let output = projection_output(journal, projection)?;

    write_json_output(&output, out)?;

    Ok(())
}

fn run_snapshot(command: SnapshotCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        SnapshotCommand::Create { journal, out } => {
            create_snapshot(&PathBuf::from(journal), &PathBuf::from(out))
        }
        SnapshotCommand::Verify { snapshot, journal } => {
            verify_snapshot(&snapshot, &PathBuf::from(journal)).map(|report| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("snapshot report is JSON")
                );
            })
        }
        SnapshotCommand::Restore {
            snapshot,
            journal,
            out,
        } => restore_snapshot(&snapshot, &PathBuf::from(journal), &out),
        SnapshotCommand::List { dir } => list_snapshots(&PathBuf::from(dir)),
    }
}

fn create_snapshot(journal: &Path, out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let projection = read_projection(journal)?;
    let projection_output = projection_output(journal, projection)?;
    let journal_digest = file_digest(journal)?;
    let generated_at = latest_activity(&projection_output)
        .unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string());
    let event_count = projection_output
        .get("event_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let tip_event_id = projection_output
        .get("tip_event_id")
        .cloned()
        .unwrap_or(Value::Null);
    let journal_byte_offset = fs::metadata(journal)?.len();
    let projection_digest = snapshot_digest(&projection_output, &canonicalizer)?;
    let workspace_id = workspace_id_for_journal(journal)?;
    let lineage = json!({
        "schema": SNAPSHOT_LINEAGE_SCHEMA,
        "parent_snapshot_digest": Value::Null,
        "covered_tip_event_id": tip_event_id,
        "generated_by": {
            "kind": "northroot_cli",
            "command": "work snapshot create"
        }
    });
    let payload = json!({
        "schema": SNAPSHOT_SCHEMA,
        "workspace_id": workspace_id,
        "generated_at": generated_at,
        "covered_event_count": event_count,
        "covered_tip_event_id": projection_output.get("tip_event_id").cloned().unwrap_or(Value::Null),
        "state": projection_output,
        "projection_sections": {
            "work_ledger": {
                "digest": digest_ref(&projection_digest),
                "authoritative": false
            }
        },
        "evidence_index_sections": {},
        "lineage": lineage
    });
    let payload_digest = snapshot_digest(&payload, &canonicalizer)?;
    let payload_digest_hex = snapshot_digest_hex(&payload, &canonicalizer)?;
    let snapshot_dir = out_dir.join("sha256");
    fs::create_dir_all(&snapshot_dir)?;
    let payload_path = snapshot_dir.join(format!("{payload_digest_hex}.snapshot.json"));
    let manifest_path = snapshot_dir.join(format!("{payload_digest_hex}.manifest.json"));
    let manifest = json!({
        "schema": SNAPSHOT_MANIFEST_SCHEMA,
        "workspace_id": workspace_id,
        "generated_at": generated_at,
        "covered_journal_ref": {
            "path": journal.display().to_string(),
            "digest": digest_ref(&journal_digest)
        },
        "covered_event_count": event_count,
        "covered_tip_event_id": payload.get("covered_tip_event_id").cloned().unwrap_or(Value::Null),
        "covered_journal_byte_offset": journal_byte_offset,
        "payload_digest": digest_ref(&payload_digest),
        "payload_digest_hex": payload_digest_hex,
        "payload_path": payload_path.display().to_string(),
        "projection_digests": {
            "work_ledger": digest_ref(&projection_digest)
        },
        "redaction_profile": "derived-state-only",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "lineage": payload.get("lineage").cloned().unwrap_or(Value::Null)
    });

    write_json_file(&payload_path, &payload)?;
    write_json_file(&manifest_path, &manifest)?;
    append_snapshot_generated_event(journal, &manifest, &canonicalizer)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "snapshot": format!("snapshot:sha256:{payload_digest_hex}"),
            "payload": payload_path.display().to_string(),
            "manifest": manifest_path.display().to_string(),
            "covered_event_count": event_count
        }))?
    );
    Ok(())
}

fn verify_snapshot(snapshot: &str, journal: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let (manifest_path, manifest) = load_snapshot_manifest(snapshot)?;
    let payload_path = manifest_payload_path(&manifest_path, &manifest)?;
    let payload: Value = serde_json::from_str(&fs::read_to_string(&payload_path)?)?;
    let computed_payload_digest = snapshot_digest(&payload, &canonicalizer)?;
    let computed_payload_hex = snapshot_digest_hex(&payload, &canonicalizer)?;
    let manifest_payload_digest = manifest
        .get("payload_digest")
        .ok_or("snapshot manifest missing payload_digest")?;
    let payload_digest_ok = manifest_payload_digest == &digest_ref(&computed_payload_digest);

    let projection = read_projection(journal)?;
    let output = projection_output(journal, projection)?;
    let covered_count = manifest
        .get("covered_event_count")
        .and_then(Value::as_u64)
        .ok_or("snapshot manifest missing covered_event_count")?;
    let journal_event_count = output
        .get("event_count")
        .and_then(Value::as_u64)
        .ok_or("projection missing event_count")?;
    let covered_tip = manifest
        .get("covered_tip_event_id")
        .cloned()
        .unwrap_or(Value::Null);
    let journal_tip_at_position = event_id_at_position(journal, covered_count as usize)?;
    let tip_ok = covered_count == 0
        || journal_tip_at_position
            .as_ref()
            .is_some_and(|event_id| *event_id == covered_tip);
    let hex_ok = manifest
        .get("payload_digest_hex")
        .and_then(Value::as_str)
        .is_some_and(|digest| digest == computed_payload_hex);
    let valid = payload_digest_ok && hex_ok && tip_ok && covered_count <= journal_event_count;

    let report = json!({
        "manifest": manifest_path.display().to_string(),
        "payload": payload_path.display().to_string(),
        "computed_snapshot": format!("snapshot:sha256:{computed_payload_hex}"),
        "payload_digest_ok": payload_digest_ok,
        "payload_digest_hex_ok": hex_ok,
        "covered_tip_ok": tip_ok,
        "covered_event_count": covered_count,
        "journal_event_count": journal_event_count,
        "valid": valid
    });
    if !valid {
        return Err(format!("snapshot verification failed: {report}").into());
    }
    Ok(report)
}

fn restore_snapshot(
    snapshot: &str,
    journal: &Path,
    out: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    verify_snapshot(snapshot, journal)?;
    let (_, manifest) = load_snapshot_manifest(snapshot)?;
    let covered_count = manifest
        .get("covered_event_count")
        .and_then(Value::as_u64)
        .ok_or("snapshot manifest missing covered_event_count")? as usize;
    let (manifest_path, _) = load_snapshot_manifest(snapshot)?;
    let payload_path = manifest_payload_path(&manifest_path, &manifest)?;
    let payload: Value = serde_json::from_str(&fs::read_to_string(payload_path)?)?;
    let state = payload
        .get("state")
        .ok_or("snapshot payload missing state")?;
    let mut projection = WorkLedgerProjection::from_output(state)?;
    replay_tail(journal, covered_count, &mut projection)?;
    let output = projection_output(journal, projection)?;
    write_json_output(&output, out)?;
    Ok(())
}

fn list_snapshots(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let snapshot_dir = dir.join("sha256");
    let mut snapshots = Vec::new();
    if snapshot_dir.exists() {
        for entry in fs::read_dir(snapshot_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".manifest.json"))
            {
                let manifest: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
                snapshots.push(json!({
                    "manifest": path.display().to_string(),
                    "snapshot": manifest.get("payload_digest_hex").and_then(Value::as_str).map(|digest| format!("snapshot:sha256:{digest}")),
                    "workspace_id": manifest.get("workspace_id").cloned().unwrap_or(Value::Null),
                    "covered_event_count": manifest.get("covered_event_count").cloned().unwrap_or(Value::Null),
                    "covered_tip_event_id": manifest.get("covered_tip_event_id").cloned().unwrap_or(Value::Null),
                    "generated_at": manifest.get("generated_at").cloned().unwrap_or(Value::Null)
                }));
            }
        }
    }
    snapshots.sort_by_key(|item| {
        item.get("covered_event_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "snapshots": snapshots }))?
    );
    Ok(())
}

fn read_projection(journal: &Path) -> Result<WorkLedgerProjection, Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let mut reader = JournalReader::open(journal, ReadMode::Strict)?;
    let mut projection = WorkLedgerProjection::default();

    while let Some(event) = reader.read_event()? {
        if !verify_event_id(&event, &canonicalizer)? {
            return Err("journal contains event_id mismatch".into());
        }
        let profile_errors = validate_work_ledger_profile_event(&event);
        if !profile_errors.is_empty() {
            return Err(format!(
                "journal contains profile-invalid work ledger event: {}",
                profile_errors.join("; ")
            )
            .into());
        }
        projection.apply(&event);
        projection.record_metadata(&event);
    }
    Ok(projection)
}

fn replay_tail(
    journal: &Path,
    skip_events: usize,
    projection: &mut WorkLedgerProjection,
) -> Result<(), Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let mut reader = JournalReader::open(journal, ReadMode::Strict)?;
    let mut index = 0usize;
    while let Some(event) = reader.read_event()? {
        index += 1;
        if index <= skip_events {
            continue;
        }
        if !verify_event_id(&event, &canonicalizer)? {
            return Err("journal contains event_id mismatch".into());
        }
        let profile_errors = validate_work_ledger_profile_event(&event);
        if !profile_errors.is_empty() {
            return Err(format!(
                "journal contains profile-invalid work ledger event: {}",
                profile_errors.join("; ")
            )
            .into());
        }
        projection.apply(&event);
        projection.record_metadata(&event);
    }
    Ok(())
}

fn projection_output(
    journal: &Path,
    projection: WorkLedgerProjection,
) -> Result<Value, Box<dyn std::error::Error>> {
    let journal_digest = file_digest(journal)?;
    Ok(projection.into_output(journal, journal_digest))
}

fn event_id_at_position(
    journal: &Path,
    position: usize,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    if position == 0 {
        return Ok(None);
    }
    let mut reader = JournalReader::open(journal, ReadMode::Strict)?;
    let mut count = 0usize;
    while let Some(event) = reader.read_event()? {
        count += 1;
        if count == position {
            return Ok(event.get("event_id").cloned());
        }
    }
    Ok(None)
}

fn append_snapshot_generated_event(
    journal: &Path,
    manifest: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_digest = snapshot_digest(manifest, canonicalizer)?;
    let manifest_hex = snapshot_digest_hex(manifest, canonicalizer)?;
    let work_id = stable_id("work", &["snapshot", &manifest_hex])?;
    let event = signed_event(
        json!({
            "event_type": "snapshot.generated",
            "event_version": "0",
            "occurred_at": manifest.get("generated_at").and_then(Value::as_str).unwrap_or(MISSING_TIMESTAMP_SENTINEL),
            "principal_id": "agent:northroot-cli",
            "canonical_profile_id": CANONICAL_PROFILE_ID,
            "schema": WORK_LEDGER_SCHEMA,
            "work_id": work_id,
            "source_system": "northroot_cli",
            "source": {
                "kind": "snapshot_manifest",
                "path": manifest.get("payload_path").and_then(Value::as_str).unwrap_or(""),
                "line": 0
            },
            "snapshot": {
                "manifest_digest": digest_ref(&manifest_digest),
                "payload_digest": manifest.get("payload_digest").cloned().unwrap_or(Value::Null),
                "payload_digest_hex": manifest.get("payload_digest_hex").cloned().unwrap_or(Value::Null),
                "covered_event_count": manifest.get("covered_event_count").cloned().unwrap_or(Value::Null),
                "covered_tip_event_id": manifest.get("covered_tip_event_id").cloned().unwrap_or(Value::Null)
            }
        }),
        canonicalizer,
    )?;
    let mut writer = JournalWriter::open(
        journal,
        WriteOptions {
            sync: true,
            create: true,
            append: true,
        },
    )?;
    writer.append_event(&event)?;
    writer.finish()?;
    Ok(())
}

fn load_snapshot_manifest(snapshot: &str) -> Result<(PathBuf, Value), Box<dyn std::error::Error>> {
    let path = validate_existing_path(&snapshot_manifest_path(snapshot))?;
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok((path, manifest))
}

fn snapshot_manifest_path(snapshot: &str) -> PathBuf {
    let path = PathBuf::from(snapshot);
    if path.exists() || snapshot.ends_with(".json") {
        return path;
    }
    let digest = snapshot
        .strip_prefix("snapshot:sha256:")
        .or_else(|| snapshot.strip_prefix("sha256:"))
        .unwrap_or(snapshot);
    PathBuf::from(".northroot/snapshots/sha256").join(format!("{digest}.manifest.json"))
}

fn manifest_payload_path(
    manifest_path: &Path,
    manifest: &Value,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let raw = manifest
        .get("payload_path")
        .and_then(Value::as_str)
        .ok_or("snapshot manifest missing payload_path")?;
    let path = PathBuf::from(raw);
    if path.is_absolute() || path.exists() {
        Ok(validate_existing_path(&path)?)
    } else {
        let candidate = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path);
        Ok(validate_existing_path(&candidate)?)
    }
}

fn validate_existing_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let canonical = path.canonicalize().map_err(|err| {
        format!(
            "invalid path {}: {err}",
            path::sanitize_path_for_error(path)
        )
    })?;
    if canonical.to_string_lossy().contains("..") {
        return Err(format!(
            "path contains traversal sequences: {}",
            path::sanitize_path_for_error(&canonical)
        )
        .into());
    }
    Ok(canonical)
}

fn snapshot_digest(
    value: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<Digest, Box<dyn std::error::Error>> {
    let bytes = snapshot_hash_input(value, canonicalizer)?;
    Ok(compute_blob_digest(&bytes)?)
}

fn snapshot_digest_hex(
    value: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = snapshot_hash_input(value, canonicalizer)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn snapshot_hash_input(
    value: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = SNAPSHOT_DOMAIN_SEPARATOR.to_vec();
    bytes.extend(canonicalizer.canonicalize(value)?.bytes);
    Ok(bytes)
}

fn file_digest(path: &Path) -> Result<Digest, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(compute_blob_digest(&bytes)?)
}

fn digest_ref(digest: &Digest) -> Value {
    serde_json::to_value(digest).expect("digest serializes")
}

fn workspace_id_for_journal(journal: &Path) -> Result<String, Box<dyn std::error::Error>> {
    stable_id("workspace", &[&journal.display().to_string()])
}

fn latest_activity(output: &Value) -> Option<String> {
    output
        .get("works")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|work| work.get("last_activity").and_then(Value::as_str))
        .max()
        .map(str::to_string)
}

fn write_json_output(value: &Value, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rendered = serde_json::to_string_pretty(value)?;
    if out == "-" {
        println!("{rendered}");
    } else {
        write_json_file(&PathBuf::from(out), value)?;
    }
    Ok(())
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

fn verify(journal: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let canonicalizer = canonicalizer()?;
    let mut reader = JournalReader::open(journal, ReadMode::Strict)?;
    let mut checked = 0usize;
    let mut invalid_events = 0usize;
    let mut kernel_valid_events = 0usize;
    let mut profile_valid_events = 0usize;
    let mut profile_invalid_events = 0usize;
    let mut profile_errors = Vec::new();

    while let Some(event) = reader.read_event()? {
        checked += 1;
        if !verify_event_id(&event, &canonicalizer)? {
            invalid_events += 1;
            continue;
        }
        kernel_valid_events += 1;

        let errors = validate_work_ledger_profile_event(&event);
        if errors.is_empty() {
            profile_valid_events += 1;
        } else {
            profile_invalid_events += 1;
            if profile_errors.len() < 20 {
                profile_errors.push(json!({
                    "event_index": checked,
                    "event_id": event.get("event_id").cloned().unwrap_or(Value::Null),
                    "errors": errors
                }));
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "journal": journal.display().to_string(),
            "events_checked": checked,
            "kernel_valid_events": kernel_valid_events,
            "profile_valid_events": profile_valid_events,
            "invalid_events": invalid_events,
            "profile_invalid_events": profile_invalid_events,
            "profile_errors": profile_errors,
            "valid": invalid_events == 0 && profile_invalid_events == 0
        }))?
    );

    if invalid_events > 0 || profile_invalid_events > 0 {
        return Err("work ledger verification failed".into());
    }

    Ok(())
}

fn validate_work_ledger_profile_event(event: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if !event.is_object() {
        return vec!["event must be a JSON object".to_string()];
    }

    let event_type = require_string(event, "event_type", &mut errors);
    validate_digest_ref(event.get("event_id"), "event_id", &mut errors);
    require_string(event, "occurred_at", &mut errors);
    require_string(event, "principal_id", &mut errors);
    require_string(event, "source_system", &mut errors);

    if require_string(event, "event_version", &mut errors) != Some("0") {
        errors.push("event_version must be 0".to_string());
    }
    if require_string(event, "canonical_profile_id", &mut errors) != Some(CANONICAL_PROFILE_ID) {
        errors.push(format!(
            "canonical_profile_id must be {CANONICAL_PROFILE_ID}"
        ));
    }
    if require_string(event, "schema", &mut errors) != Some(WORK_LEDGER_SCHEMA) {
        errors.push(format!("schema must be {WORK_LEDGER_SCHEMA}"));
    }
    if let Some(work_id) = require_string(event, "work_id", &mut errors) {
        if !work_id.starts_with("work:") {
            errors.push("work_id must start with work:".to_string());
        }
    }

    validate_source_ref(event.get("source"), &mut errors);

    let Some(event_type) = event_type else {
        return errors;
    };

    match event_type {
        "work.observed" => {}
        "run.observed" | "run.completed" | "run.blocked" => {
            require_run_id(event, &mut errors);
        }
        "artifact.observed" => {
            require_run_id(event, &mut errors);
            require_object(event, "artifact", &mut errors);
        }
        "snapshot.generated" | "snapshot.restored" => {
            require_object(event, "snapshot", &mut errors);
        }
        "backup.receipt.observed" => {
            require_object(event, "backup_receipt", &mut errors);
        }
        other => errors.push(format!("event_type is not in work-ledger vocabulary: {other}")),
    }

    errors
}

fn require_string<'a>(event: &'a Value, field: &str, errors: &mut Vec<String>) -> Option<&'a str> {
    match event.get(field).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => Some(value),
        _ => {
            errors.push(format!("{field} must be a non-empty string"));
            None
        }
    }
}

fn require_object(event: &Value, field: &str, errors: &mut Vec<String>) {
    if !event.get(field).is_some_and(Value::is_object) {
        errors.push(format!("{field} must be an object"));
    }
}

fn require_run_id(event: &Value, errors: &mut Vec<String>) {
    if let Some(run_id) = require_string(event, "run_id", errors) {
        if !run_id.starts_with("run:") {
            errors.push("run_id must start with run:".to_string());
        }
    }
}

fn validate_source_ref(source: Option<&Value>, errors: &mut Vec<String>) {
    let Some(source) = source else {
        errors.push("source must be an object".to_string());
        return;
    };
    if !source.is_object() {
        errors.push("source must be an object".to_string());
        return;
    }
    for field in ["kind", "path"] {
        match source.get(field).and_then(Value::as_str) {
            Some(value) if !value.is_empty() => {}
            _ => errors.push(format!("source.{field} must be a non-empty string")),
        }
    }
    if !source.get("line").is_some_and(Value::is_u64) {
        errors.push("source.line must be a non-negative integer".to_string());
    }
}

fn validate_digest_ref(value: Option<&Value>, field: &str, errors: &mut Vec<String>) {
    let Some(value) = value else {
        errors.push(format!("{field} must be a digest object"));
        return;
    };
    if !value.is_object() {
        errors.push(format!("{field} must be a digest object"));
        return;
    }
    match value.get("alg").and_then(Value::as_str) {
        Some("sha-256") => {}
        _ => errors.push(format!("{field}.alg must be sha-256")),
    }
    match value.get("b64").and_then(Value::as_str) {
        Some(b64) if !b64.is_empty() => {}
        _ => errors.push(format!("{field}.b64 must be a non-empty string")),
    }
}

#[derive(Debug)]
struct SourceRecord {
    line: usize,
    value: Value,
}

#[derive(Debug, Serialize)]
struct MalformedRecord {
    source_path: String,
    line: usize,
    error: String,
    raw_digest: Value,
    redacted_preview: String,
}

fn collect_session_files(path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else {
        collect_session_files_recursive(path, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_session_files_recursive(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_session_files_recursive(&entry_path, files)?;
        } else if entry_path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn prepare_journal_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(path.to_path_buf())
}

fn read_source_records(
    path: &Path,
    strict: bool,
    malformed_records: &mut usize,
    quarantined_records: &mut Vec<MalformedRecord>,
    warnings: &mut Vec<String>,
) -> Result<Vec<SourceRecord>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line_result) in reader.lines().enumerate() {
        let line_no = index + 1;
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(&line) {
            Ok(value) => records.push(SourceRecord {
                line: line_no,
                value,
            }),
            Err(err) if strict => {
                return Err(format!("{}:{line_no}: malformed JSON: {err}", path.display()).into());
            }
            Err(err) => {
                *malformed_records += 1;
                quarantined_records.push(MalformedRecord {
                    source_path: path.display().to_string(),
                    line: line_no,
                    error: err.to_string(),
                    raw_digest: digest_text(&line)?,
                    redacted_preview: truncate(&redact_text(&line), 240),
                });
                warnings.push(format!(
                    "{}:{line_no}: malformed JSON: {err}",
                    path.display()
                ));
            }
        }
    }

    Ok(records)
}

fn write_malformed_quarantine(
    path: &Path,
    records: &[MalformedRecord],
) -> Result<(), Box<dyn std::error::Error>> {
    if records.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = File::create(path)?;
    for record in records {
        file.write_all(serde_json::to_string(record)?.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn can_append_more(max_events: Option<usize>, appended: usize) -> bool {
    max_events.is_none_or(|limit| appended < limit)
}

fn discover_session_id(path: &Path, records: &[SourceRecord]) -> String {
    records
        .iter()
        .find_map(|record| {
            let payload = record.value.get("payload")?;
            if record.value.get("type").and_then(Value::as_str) == Some("session_meta") {
                payload
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown-session")
                .to_string()
        })
}

fn record_timestamp(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/payload/timestamp").and_then(Value::as_str))
        .map(str::to_string)
}

fn source_ref(path: &Path, line: usize) -> Value {
    json!({
        "kind": "codex_session_jsonl",
        "path": path.display().to_string(),
        "line": line
    })
}

fn turn_observation(record: &Value) -> Value {
    let payload = record.get("payload").unwrap_or(&Value::Null);
    json!({
        "turn_id": payload.get("turn_id").and_then(Value::as_str),
        "cwd": payload.get("cwd").and_then(Value::as_str),
        "model": payload.get("model").and_then(Value::as_str)
    })
}

fn artifact_observation(
    path: &Path,
    record: &SourceRecord,
    include_text: bool,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let record_type = record
        .value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("");
    let payload = record.value.get("payload").unwrap_or(&Value::Null);

    match record_type {
        "response_item" => response_item_artifact(path, record.line, payload, include_text),
        "event_msg" => event_msg_artifact(path, record.line, payload, include_text),
        _ => Ok(None),
    }
}

fn response_item_artifact(
    path: &Path,
    line: usize,
    payload: &Value,
    include_text: bool,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match payload.get("type").and_then(Value::as_str).unwrap_or("") {
        "function_call" => {
            let name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let arguments = payload
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("");
            let parsed_arguments: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
            let command = parsed_arguments.get("cmd").and_then(Value::as_str);
            let redacted_command = command.map(redact_text);
            Ok(Some(json!({
                "artifact_ref": text_ref("function_call", path, line, arguments)?,
                "kind": "tool_call",
                "tool_name": name,
                "arguments_digest": digest_text(arguments)?,
                "command": redacted_command,
                "touched_paths": command.map(extract_path_tokens).unwrap_or_default()
            })))
        }
        "function_call_output" => {
            let output = payload.get("output").and_then(Value::as_str).unwrap_or("");
            Ok(Some(json!({
                "artifact_ref": text_ref("function_call_output", path, line, output)?,
                "kind": "tool_output",
                "call_id": payload.get("call_id").and_then(Value::as_str),
                "exit_code": parse_exit_code(output),
                "output_digest": digest_text(output)?,
                "output_len": output.len()
            })))
        }
        "message" => {
            let text = message_text(payload);
            if text.is_empty() {
                return Ok(None);
            }
            Ok(Some(text_artifact(
                "message",
                path,
                line,
                payload.get("role").and_then(Value::as_str),
                &text,
                include_text,
            )?))
        }
        _ => Ok(None),
    }
}

fn event_msg_artifact(
    path: &Path,
    line: usize,
    payload: &Value,
    include_text: bool,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match payload.get("type").and_then(Value::as_str).unwrap_or("") {
        "user_message" | "agent_message" => {
            let text = payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                return Ok(None);
            }
            Ok(Some(text_artifact(
                payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("message"),
                path,
                line,
                None,
                &text,
                include_text,
            )?))
        }
        "token_count" => Ok(Some(json!({
            "artifact_ref": text_ref("token_count", path, line, &payload.to_string())?,
            "kind": "token_count",
            "usage": payload.get("info").and_then(|info| info.get("last_token_usage")).cloned().unwrap_or(Value::Null)
        }))),
        _ => Ok(None),
    }
}

fn text_artifact(
    kind: &str,
    path: &Path,
    line: usize,
    role: Option<&str>,
    text: &str,
    include_text: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let redacted = redact_text(text);
    let mut artifact = Map::new();
    artifact.insert(
        "artifact_ref".to_string(),
        text_ref(kind, path, line, &redacted)?,
    );
    artifact.insert("kind".to_string(), Value::String(kind.to_string()));
    artifact.insert("text_digest".to_string(), digest_text(&redacted)?);
    artifact.insert("text_len".to_string(), Value::from(redacted.len()));
    if let Some(role) = role {
        artifact.insert("role".to_string(), Value::String(role.to_string()));
    }
    if include_text {
        artifact.insert("redacted_text".to_string(), Value::String(redacted));
    }
    Ok(Value::Object(artifact))
}

fn message_text(payload: &Value) -> String {
    payload
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("text")
                        .or_else(|| item.get("input_text"))
                        .or_else(|| item.get("output_text"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn terminal_status(record: &Value) -> Option<&'static str> {
    let payload = record.get("payload")?;
    let payload_type = payload.get("type").and_then(Value::as_str);

    match (record.get("type").and_then(Value::as_str), payload_type) {
        (Some("event_msg"), Some("task_complete")) => Some("completed"),
        (Some("event_msg"), Some("agent_message"))
            if payload.get("phase").and_then(Value::as_str) == Some("final_answer") =>
        {
            let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
            if message.to_lowercase().contains("blocked") {
                Some("blocked")
            } else {
                Some("completed")
            }
        }
        _ => None,
    }
}

fn terminal_reason(record: &Value) -> String {
    record
        .pointer("/payload/message")
        .and_then(Value::as_str)
        .map(|message| truncate(&redact_text(message), 240))
        .or_else(|| {
            record
                .pointer("/payload/last_agent_message")
                .and_then(Value::as_str)
                .map(|message| truncate(&redact_text(message), 240))
        })
        .unwrap_or_else(|| "terminal telemetry record observed".to_string())
}

fn parse_exit_code(output: &str) -> Option<i64> {
    let marker = "exited with code ";
    let start = output.find(marker)? + marker.len();
    let tail = &output[start..];
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();
    digits.parse::<i64>().ok()
}

fn extract_path_tokens(command: &str) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for token in command.split_whitespace() {
        let cleaned = token.trim_matches(|ch: char| {
            matches!(ch, '"' | '\'' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
        });
        if cleaned.contains('/') && !cleaned.contains("://") {
            paths.insert(cleaned.to_string());
        }
        if paths.len() >= 20 {
            break;
        }
    }
    paths.into_iter().collect()
}

fn redact_text(input: &str) -> String {
    let mut output = Vec::new();
    let mut redact_next_count = 0usize;
    for token in input.split_whitespace() {
        if redact_next_count > 0 {
            output.push("[REDACTED]");
            redact_next_count -= 1;
            continue;
        }

        let lower = token.to_ascii_lowercase();
        if should_redact_token(&lower) {
            output.push("[REDACTED]");
            redact_next_count = redacted_following_token_count(&lower);
        } else {
            output.push(token);
        }
    }
    output.join(" ")
}

fn should_redact_token(lower: &str) -> bool {
    lower.starts_with("sk-")
        || lower.contains("://") && lower.contains('@')
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("token=")
        || lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("authorization:")
        || lower.contains("\"api_key\"")
        || lower.contains("\"apikey\"")
        || lower.contains("\"token\"")
        || lower.contains("\"password\"")
        || lower.contains("\"secret\"")
        || lower == "bearer"
        || lower == "basic"
        || lower == "--token"
        || lower == "--password"
        || lower == "--secret"
        || lower == "--api-key"
}

fn redacted_following_token_count(lower: &str) -> usize {
    if lower.contains("authorization:") {
        2
    } else if lower == "bearer"
        || lower == "basic"
        || lower == "--token"
        || lower == "--password"
        || lower == "--secret"
        || lower == "--api-key"
    {
        1
    } else {
        0
    }
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len.saturating_sub(3)])
    }
}

fn text_ref(
    kind: &str,
    path: &Path,
    line: usize,
    text: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "kind": kind,
        "source_path": path.display().to_string(),
        "source_line": line,
        "digest": digest_text(text)?
    }))
}

fn digest_text(text: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(compute_blob_digest(text.as_bytes())?)?)
}

fn signed_event(
    mut event: Value,
    canonicalizer: &Canonicalizer,
) -> Result<Value, Box<dyn std::error::Error>> {
    let event_id = compute_event_id(&event, canonicalizer)?;
    event["event_id"] = serde_json::to_value(event_id)?;
    Ok(event)
}

fn append_if_new(
    writer: &mut JournalWriter,
    event: Value,
    known_event_ids: &mut HashSet<String>,
    appended: &mut usize,
    skipped_duplicates: &mut usize,
    max_events: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if max_events.is_some_and(|limit| *appended >= limit) {
        return Ok(());
    }

    let event_id = event_id_b64(&event).unwrap_or_default();
    if !event_id.is_empty() && known_event_ids.contains(&event_id) {
        *skipped_duplicates += 1;
        return Ok(());
    }

    writer.append_event(&event)?;
    if !event_id.is_empty() {
        known_event_ids.insert(event_id);
    }
    *appended += 1;
    Ok(())
}

fn read_event_ids(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let mut ids = HashSet::new();
    if !path.exists() {
        return Ok(ids);
    }
    let mut reader = JournalReader::open(path, ReadMode::Strict)?;
    while let Some(event) = reader.read_event()? {
        if let Some(event_id) = event_id_b64(&event) {
            ids.insert(event_id);
        }
    }
    Ok(ids)
}

fn event_id_b64(event: &Value) -> Option<String> {
    event
        .get("event_id")
        .and_then(|value| value.get("b64"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn stable_id(prefix: &str, parts: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = prefix.to_string();
    for part in parts {
        input.push('\0');
        input.push_str(part);
    }
    let digest = compute_blob_digest(input.as_bytes())?;
    Ok(format!("{prefix}:{}", digest.b64))
}

fn canonicalizer() -> Result<Canonicalizer, Box<dyn std::error::Error>> {
    let profile = ProfileId::parse(CANONICAL_PROFILE_ID)?;
    Ok(Canonicalizer::new(profile))
}

#[derive(Debug, Default)]
struct WorkLedgerProjection {
    works: BTreeMap<String, WorkView>,
    event_count: usize,
    tip_event_id: Option<Value>,
    generated_at: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct WorkView {
    work_id: String,
    status: String,
    run_count: usize,
    observed_actors: BTreeSet<String>,
    evidence_refs: Vec<Value>,
    last_activity: Option<String>,
    reason: Option<String>,
}

impl WorkLedgerProjection {
    fn apply(&mut self, event: &Value) {
        let Some(work_id) = event.get("work_id").and_then(Value::as_str) else {
            return;
        };
        let entry = self
            .works
            .entry(work_id.to_string())
            .or_insert_with(|| WorkView {
                work_id: work_id.to_string(),
                status: "observed".to_string(),
                ..WorkView::default()
            });

        if let Some(actor) = event.get("principal_id").and_then(Value::as_str) {
            entry.observed_actors.insert(actor.to_string());
        }
        if let Some(timestamp) = event.get("occurred_at").and_then(Value::as_str) {
            entry.last_activity = Some(timestamp.to_string());
        }

        match event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("")
        {
            "run.observed" => {
                entry.run_count += 1;
                if entry.status == "observed" {
                    entry.status = "running".to_string();
                }
            }
            "artifact.observed" => {
                if let Some(ref_value) = event.pointer("/artifact/artifact_ref") {
                    entry.evidence_refs.push(ref_value.clone());
                }
            }
            "run.completed" => {
                entry.status = "completed".to_string();
                entry.reason = event
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            "run.blocked" => {
                entry.status = "blocked".to_string();
                entry.reason = event
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            _ => {}
        }
    }

    fn record_metadata(&mut self, event: &Value) {
        self.event_count += 1;
        self.tip_event_id = event.get("event_id").cloned();
        if let Some(timestamp) = event.get("occurred_at").and_then(Value::as_str) {
            self.generated_at = Some(timestamp.to_string());
        }
    }

    fn from_output(output: &Value) -> Result<Self, Box<dyn std::error::Error>> {
        let mut projection = WorkLedgerProjection {
            event_count: output
                .get("event_count")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            tip_event_id: output.get("tip_event_id").cloned(),
            generated_at: output
                .get("generated_at")
                .and_then(Value::as_str)
                .map(str::to_string),
            ..WorkLedgerProjection::default()
        };
        for work in output
            .get("works")
            .and_then(Value::as_array)
            .ok_or("projection output missing works array")?
        {
            let work_id = work
                .get("work_id")
                .and_then(Value::as_str)
                .ok_or("projection work missing work_id")?
                .to_string();
            let view: WorkView = serde_json::from_value(work.clone())?;
            projection.works.insert(work_id, view);
        }
        Ok(projection)
    }

    fn into_output(self, journal: &Path, journal_digest: Digest) -> Value {
        json!({
            "schema": "northroot.work_ledger_projection.v0",
            "source_journal": journal.display().to_string(),
            "source_journal_digest": digest_ref(&journal_digest),
            "event_count": self.event_count,
            "tip_event_id": self.tip_event_id,
            "generated_at": self.generated_at.unwrap_or_else(|| MISSING_TIMESTAMP_SENTINEL.to_string()),
            "works": self.works.into_values().collect::<Vec<_>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_journal::JournalReader;
    use tempfile::TempDir;

    fn default_ingest_options() -> IngestOptions {
        IngestOptions {
            dedupe: true,
            ..IngestOptions::default()
        }
    }

    #[test]
    fn redaction_removes_common_secret_shapes() {
        let cases = [
            ("token assignment", "token=abc123", "abc123"),
            ("password assignment", "password=abc123", "abc123"),
            ("api key assignment", "api_key=abc123", "abc123"),
            ("json token", r#"{"token":"abc123"}"#, "abc123"),
            ("json password", r#"{"password":"abc123"}"#, "abc123"),
            ("openai key", "sk-test-secret", "sk-test-secret"),
            (
                "authorization bearer",
                "Authorization: Bearer abc123",
                "abc123",
            ),
            (
                "url credentials",
                "https://user:abc123@example.com/path",
                "abc123",
            ),
            ("flag value", "--token abc123", "abc123"),
            ("multiline", "first\nsecret=abc123\nlast", "abc123"),
        ];
        for (name, input, forbidden) in cases {
            let redacted = redact_text(input);
            assert!(
                !redacted.contains(forbidden),
                "{name} leaked {forbidden}: {redacted}"
            );
            assert!(redacted.contains("[REDACTED]"), "{name} did not redact");
        }

        let false_positive = redact_text("tokenization and secretariat are normal words");
        assert_eq!(
            false_positive,
            "tokenization and secretariat are normal words"
        );
    }

    #[test]
    #[ignore = "seeded redaction variation gate for nightly/manual CI"]
    fn seeded_redaction_variations_do_not_leak_generated_values() {
        let mut seed = 0x5eed_u64;
        for index in 0..512 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let secret = format!("s{:016x}", seed);
            let shape = match index % 8 {
                0 => format!("token={secret}"),
                1 => format!("password={secret}"),
                2 => format!("Authorization: Bearer {secret}"),
                3 => format!("--api-key {secret}"),
                4 => format!(r#"{{"secret":"{secret}"}}"#),
                5 => format!("https://user:{secret}@example.com/path"),
                6 => format!("Bearer {secret}"),
                _ => format!("first\napi_key={secret}\nlast"),
            };
            let redacted = redact_text(&shape);
            assert!(
                !redacted.contains(&secret),
                "seeded redaction variation {index} leaked generated value"
            );
        }
    }

    #[test]
    fn stable_ids_are_deterministic() {
        let first = stable_id("work", &["codex", "session-a"]).unwrap();
        let second = stable_id("work", &["codex", "session-a"]).unwrap();
        let third = stable_id("work", &["codex", "session-b"]).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn ingest_codex_creates_valid_projectable_journal_and_dedupes() {
        let temp = TempDir::new().unwrap();
        let sessions = temp.path().join("sessions");
        fs::create_dir(&sessions).unwrap();
        let session = sessions.join("rollout-test.jsonl");
        fs::write(
            &session,
            [
                r#"{"timestamp":"2026-05-30T12:00:00Z","type":"session_meta","payload":{"id":"session-a","timestamp":"2026-05-30T12:00:00Z"}}"#,
                r#"{"timestamp":"2026-05-30T12:01:00Z","type":"turn_context","payload":{"turn_id":"turn-a","cwd":"/tmp/work","model":"gpt-test"}}"#,
                r#"{"timestamp":"2026-05-30T12:01:10Z","type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"cmd\":\"cargo test --manifest-path apps/northroot/Cargo.toml\"}"}}"#,
                r#"{"timestamp":"2026-05-30T12:01:11Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-a","output":"Process exited with code 0"}}"#,
                r#"{"timestamp":"2026-05-30T12:02:00Z","type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"Done. token=secret"}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let journal = temp.path().join(".northroot/work-ledger.nrj");
        ingest_codex(&sessions, &journal, default_ingest_options()).unwrap();
        let mut reader = JournalReader::open(&journal, ReadMode::Strict).unwrap();
        let mut event_count = 0usize;
        while reader.read_event().unwrap().is_some() {
            event_count += 1;
        }
        assert_eq!(event_count, 6);

        ingest_codex(&sessions, &journal, default_ingest_options()).unwrap();
        let mut reader = JournalReader::open(&journal, ReadMode::Strict).unwrap();
        let mut deduped_count = 0usize;
        while reader.read_event().unwrap().is_some() {
            deduped_count += 1;
        }
        assert_eq!(deduped_count, event_count);

        let projection_path = temp.path().join("projection.json");
        project(&journal, projection_path.to_str().unwrap()).unwrap();
        let projection: Value =
            serde_json::from_str(&fs::read_to_string(projection_path).unwrap()).unwrap();
        assert_eq!(projection["schema"], "northroot.work_ledger_projection.v0");
        assert_eq!(projection["works"][0]["status"], "completed");
        assert_eq!(projection["works"][0]["run_count"], 1);
        assert!(
            projection["works"][0]["evidence_refs"]
                .as_array()
                .unwrap()
                .len()
                >= 3
        );
    }

    #[test]
    fn work_ledger_profile_verifier_accepts_valid_event() {
        let canonicalizer = canonicalizer().unwrap();
        let event = signed_event(
            json!({
                "event_type": "work.observed",
                "event_version": "0",
                "occurred_at": "2026-05-30T12:00:00Z",
                "principal_id": "agent:test",
                "canonical_profile_id": CANONICAL_PROFILE_ID,
                "schema": WORK_LEDGER_SCHEMA,
                "work_id": "work:test",
                "source": {
                    "kind": "unit_test",
                    "path": "work.rs",
                    "line": 1
                },
                "source_system": "unit_test"
            }),
            &canonicalizer,
        )
        .unwrap();

        assert_eq!(validate_work_ledger_profile_event(&event), Vec::<String>::new());
    }

    #[test]
    fn work_verify_rejects_kernel_valid_profile_invalid_event() {
        let temp = TempDir::new().unwrap();
        let journal = temp.path().join("work.nrj");
        let canonicalizer = canonicalizer().unwrap();
        let event = signed_event(
            json!({
                "event_type": "work.observed",
                "event_version": "0",
                "occurred_at": "2026-05-30T12:00:00Z",
                "principal_id": "agent:test",
                "canonical_profile_id": CANONICAL_PROFILE_ID,
                "schema": WORK_LEDGER_SCHEMA,
                "work_id": "not-a-work-id",
                "source_system": "unit_test"
            }),
            &canonicalizer,
        )
        .unwrap();
        let mut writer = JournalWriter::open(
            &journal,
            WriteOptions {
                sync: false,
                create: true,
                append: true,
            },
        )
        .unwrap();
        writer.append_event(&event).unwrap();
        writer.finish().unwrap();

        assert!(verify(&journal).is_err());
        assert!(read_projection(&journal).is_err());
    }

    #[test]
    fn malformed_records_are_skipped_unless_strict() {
        let temp = TempDir::new().unwrap();
        let session = temp.path().join("bad.jsonl");
        fs::write(
            &session,
            [
                r#"{"timestamp":"2026-05-30T12:00:00Z","type":"session_meta","payload":{"id":"session-b"}}"#,
                "{not-json",
            ]
            .join("\n"),
        )
        .unwrap();

        let journal = temp.path().join("work.nrj");
        let quarantine = temp.path().join(".northroot/quarantine/malformed.jsonl");
        ingest_codex(
            &session,
            &journal,
            IngestOptions {
                malformed_out: Some(quarantine.clone()),
                ..default_ingest_options()
            },
        )
        .unwrap();
        let quarantine_text = fs::read_to_string(quarantine).unwrap();
        assert!(quarantine_text.contains("raw_digest"));
        assert!(ingest_codex(
            &session,
            &journal,
            IngestOptions {
                strict: true,
                ..default_ingest_options()
            }
        )
        .is_err());
    }

    #[test]
    fn snapshot_digest_is_stable_and_tamper_evident() {
        let canonicalizer = canonicalizer().unwrap();
        let payload = json!({
            "schema": SNAPSHOT_SCHEMA,
            "workspace_id": "workspace:test",
            "covered_event_count": 1
        });
        let first = snapshot_digest_hex(&payload, &canonicalizer).unwrap();
        let second = snapshot_digest_hex(&payload, &canonicalizer).unwrap();
        let changed = snapshot_digest_hex(
            &json!({
                "schema": SNAPSHOT_SCHEMA,
                "workspace_id": "workspace:test",
                "covered_event_count": 2
            }),
            &canonicalizer,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_ne!(first, changed);
    }

    #[test]
    fn snapshot_create_verify_restore_replays_tail() {
        let temp = TempDir::new().unwrap();
        let session = temp.path().join("session.jsonl");
        fs::write(
            &session,
            [
                r#"{"timestamp":"2026-05-30T12:00:00Z","type":"session_meta","payload":{"id":"session-c"}}"#,
                r#"{"timestamp":"2026-05-30T12:01:00Z","type":"turn_context","payload":{"turn_id":"turn-a"}}"#,
                r#"{"timestamp":"2026-05-30T12:02:00Z","type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"Done"}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let journal = temp.path().join(".northroot/work-ledger.nrj");
        ingest_codex(&session, &journal, default_ingest_options()).unwrap();
        let snapshot_dir = temp.path().join(".northroot/snapshots");
        create_snapshot(&journal, &snapshot_dir).unwrap();
        let manifests = fs::read_dir(snapshot_dir.join("sha256"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".manifest.json"))
            })
            .collect::<Vec<_>>();
        assert_eq!(manifests.len(), 1);
        let manifest_path = manifests[0].display().to_string();
        let report = verify_snapshot(&manifest_path, &journal).unwrap();
        assert_eq!(report["valid"], true);

        let restored = temp.path().join("restored.json");
        restore_snapshot(&manifest_path, &journal, restored.to_str().unwrap()).unwrap();
        let restored_projection: Value =
            serde_json::from_str(&fs::read_to_string(&restored).unwrap()).unwrap();
        assert_eq!(
            restored_projection["event_count"].as_u64().unwrap(),
            5,
            "restore should replay the snapshot.generated event appended after the snapshot"
        );
    }

    #[test]
    fn snapshot_verify_rejects_payload_tampering() {
        let temp = TempDir::new().unwrap();
        let session = temp.path().join("session.jsonl");
        fs::write(
            &session,
            r#"{"timestamp":"2026-05-30T12:00:00Z","type":"session_meta","payload":{"id":"session-d"}}"#,
        )
        .unwrap();
        let journal = temp.path().join(".northroot/work-ledger.nrj");
        ingest_codex(&session, &journal, default_ingest_options()).unwrap();
        let snapshot_dir = temp.path().join(".northroot/snapshots");
        create_snapshot(&journal, &snapshot_dir).unwrap();
        let manifest_path = fs::read_dir(snapshot_dir.join("sha256"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".manifest.json"))
            })
            .unwrap();
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let payload_path = manifest_payload_path(&manifest_path, &manifest).unwrap();
        let mut payload: Value =
            serde_json::from_str(&fs::read_to_string(&payload_path).unwrap()).unwrap();
        payload["state"]["schema"] = Value::String("tampered".to_string());
        write_json_file(&payload_path, &payload).unwrap();

        assert!(verify_snapshot(manifest_path.to_str().unwrap(), &journal).is_err());
    }
}
