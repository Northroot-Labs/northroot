//! Record stream import/export command implementation.

use clap::Subcommand;
use northroot_journal::WriteOptions;
use northroot_record::{
    export_nrj_records_to_jsonl_segment, import_jsonl_segment_to_nrj_records,
    verify_jsonl_segment, verify_nrj_record_stream, SegmentSeal, SourceJournalReport,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Record stream subcommands.
#[derive(Subcommand)]
pub enum RecordCommand {
    /// Import a canonical JSONL record segment into an authoritative .nrj stream
    ImportJsonl {
        /// Input canonical JSONL segment
        #[arg(long)]
        input: String,
        /// Target .nrj record stream
        #[arg(long)]
        journal: String,
        /// Sync file to disk after each append
        #[arg(long)]
        sync: bool,
        /// Output import report as JSON
        #[arg(long)]
        json: bool,
    },
    /// Export a verified .nrj record stream to a sealed canonical JSONL segment
    ExportJsonl {
        /// Source .nrj record stream
        #[arg(long)]
        journal: String,
        /// Output canonical JSONL segment path
        #[arg(long)]
        out: String,
        /// Output export report as JSON
        #[arg(long)]
        json: bool,
    },
    /// Verify an authoritative .nrj record stream
    VerifyNrj {
        /// Source .nrj record stream
        #[arg(long)]
        journal: String,
        /// Output verification report as JSON
        #[arg(long)]
        json: bool,
    },
    /// Verify a sealed canonical JSONL record segment
    VerifyJsonl {
        /// Input canonical JSONL segment
        #[arg(long)]
        input: String,
        /// Fail if the seal names a source .nrj journal that is unavailable or mismatched
        #[arg(long)]
        require_source: bool,
        /// Output verification report as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Runs a record stream subcommand.
pub fn run(command: RecordCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RecordCommand::ImportJsonl {
            input,
            journal,
            sync,
            json,
        } => import_jsonl(&PathBuf::from(input), &PathBuf::from(journal), sync, json),
        RecordCommand::ExportJsonl { journal, out, json } => {
            export_jsonl(&PathBuf::from(journal), &PathBuf::from(out), json)
        }
        RecordCommand::VerifyNrj { journal, json } => verify_nrj(&PathBuf::from(journal), json),
        RecordCommand::VerifyJsonl {
            input,
            require_source,
            json,
        } => verify_jsonl(&PathBuf::from(input), require_source, json),
    }
}

fn import_jsonl(
    input: &Path,
    journal: &Path,
    sync: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = import_jsonl_segment_to_nrj_records(
        input,
        journal,
        WriteOptions {
            sync,
            create: true,
            append: true,
        },
    )?;

    let report = ImportReport {
        schema: "northroot.record_jsonl_import.v0",
        input: input.display().to_string(),
        journal: journal.display().to_string(),
        imported_record_count: summary.imported_record_count,
        input_first_seq: summary.input_first_seq,
        input_last_seq: summary.input_last_seq,
        output_first_seq: summary.output_first_seq,
        output_last_seq: summary.output_last_seq,
        input_seal: summary.input_seal,
    };
    emit_report(&report, json)?;
    Ok(())
}

fn export_jsonl(
    journal: &Path,
    out: &Path,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let seal = export_nrj_records_to_jsonl_segment(journal, out)?;
    let report = ExportReport {
        schema: "northroot.record_jsonl_export.v0",
        journal: journal.display().to_string(),
        out: out.display().to_string(),
        seal,
    };
    emit_report(&report, json)?;
    Ok(())
}

fn verify_nrj(journal: &Path, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (valid, summary, error) = match verify_nrj_record_stream(journal) {
        Ok(summary) => (true, summary, None),
        Err(err) => (false, Default::default(), Some(err.to_string())),
    };
    let report = VerifyNrjReport {
        schema: "northroot.record_nrj_verify.v0",
        journal: journal.display().to_string(),
        valid,
        record_count: summary.record_count,
        first_seq: summary.first_seq,
        last_seq: summary.last_seq,
        error,
    };
    emit_report(&report, json)?;
    if !valid {
        return Err("NRJ record stream verification failed".into());
    }
    Ok(())
}

fn verify_jsonl(
    input: &Path,
    require_source: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let verification = verify_jsonl_segment(input, require_source)?;
    let report = VerifyReport {
        schema: "northroot.record_jsonl_verify.v0",
        input: input.display().to_string(),
        valid: verification.valid,
        seal: verification.seal,
        source: verification.source,
    };
    emit_report(&report, json)?;
    if !report.valid {
        return Err("JSONL segment verification failed".into());
    }
    Ok(())
}

fn emit_report<T: Serialize>(report: &T, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{}", serde_json::to_string(report)?);
    }
    Ok(())
}

#[derive(Serialize)]
struct ImportReport {
    schema: &'static str,
    input: String,
    journal: String,
    imported_record_count: u64,
    input_first_seq: Option<u64>,
    input_last_seq: Option<u64>,
    output_first_seq: Option<u64>,
    output_last_seq: Option<u64>,
    input_seal: SegmentSeal,
}

#[derive(Serialize)]
struct ExportReport {
    schema: &'static str,
    journal: String,
    out: String,
    seal: SegmentSeal,
}

#[derive(Serialize)]
struct VerifyNrjReport {
    schema: &'static str,
    journal: String,
    valid: bool,
    record_count: u64,
    first_seq: Option<u64>,
    last_seq: Option<u64>,
    error: Option<String>,
}

#[derive(Serialize)]
struct VerifyReport {
    schema: &'static str,
    input: String,
    valid: bool,
    seal: SegmentSeal,
    source: SourceJournalReport,
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
    use northroot_journal::{JournalWriter, ReadMode};
    use northroot_record::{
        compute_record_id, seal_segment, Context, JsonlSegmentReader, JsonlSegmentWriter,
        NrjRecordReader, NrjRecordWriter, Record, RecordRefs, RecordRole, Statement,
    };
    use serde_json::json;

    fn record() -> Record {
        let mut record = Record::new(
            RecordRole::Event,
            Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: "resource.classified".to_string(),
                object: "resource:document:seed-report".to_string(),
            },
            Context {
                node_id: Some("node:ag_demo_2026".to_string()),
                time: Some("2026-06-14T18:00:00Z".to_string()),
                ..Context::default()
            },
            RecordRefs {
                inputs: vec!["resource:document:seed-report".to_string()],
                outputs: vec!["resource:classification:xyz".to_string()],
                causes: vec![
                    "event:sha256:1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                ],
                ..RecordRefs::default()
            },
            json!({}),
        );
        record.id = compute_record_id(&record).unwrap();
        record
    }

    #[test]
    fn imports_jsonl_segment_to_nrj_record_stream() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("records.jsonl");
        let nrj = dir.path().join("records.nrj");
        let mut writer = JsonlSegmentWriter::create(&jsonl, 1).unwrap();
        writer.append(record()).unwrap();
        writer.flush().unwrap();
        seal_segment(&jsonl).unwrap();

        import_jsonl(&jsonl, &nrj, false, true).unwrap();

        let mut reader = NrjRecordReader::open(&nrj, ReadMode::Strict).unwrap();
        let entry = reader.read_next().unwrap().unwrap();
        assert_eq!(entry.seq, 1);
        assert_eq!(entry.record.statement.predicate, "resource.classified");
        assert!(reader.read_next().unwrap().is_none());
    }

    #[test]
    fn verifies_nrj_record_stream() {
        let dir = tempfile::tempdir().unwrap();
        let nrj = dir.path().join("records.nrj");

        let mut writer = NrjRecordWriter::open(&nrj, WriteOptions::default()).unwrap();
        writer.append(record()).unwrap();
        writer.finish().unwrap();

        verify_nrj(&nrj, true).unwrap();
    }

    #[test]
    fn verify_nrj_rejects_non_contiguous_record_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let nrj = dir.path().join("records.nrj");
        let mut writer = JournalWriter::open(&nrj, WriteOptions::default()).unwrap();
        let canonicalizer =
            Canonicalizer::new(ProfileId::parse("northroot-canonical-v1").unwrap());
        for seq in [1, 3] {
            let mut event = json!({
                "event_type": "northroot.record.appended",
                "event_version": "0",
                "canonical_profile_id": "northroot-canonical-v1",
                "seq": seq,
                "record": record(),
            });
            let event_id = compute_event_id(&event, &canonicalizer).unwrap();
            event["event_id"] = serde_json::to_value(event_id).unwrap();
            writer.append_event(&event).unwrap();
        }
        writer.finish().unwrap();

        assert!(verify_nrj(&nrj, true).is_err());
    }

    #[test]
    fn import_jsonl_rejects_unsealed_segments() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl = dir.path().join("records.jsonl");
        let nrj = dir.path().join("records.nrj");
        let mut writer = JsonlSegmentWriter::create(&jsonl, 1).unwrap();
        writer.append(record()).unwrap();
        writer.flush().unwrap();

        assert!(import_jsonl(&jsonl, &nrj, false, true).is_err());
        assert!(!nrj.exists());
    }

    #[test]
    fn exports_nrj_record_stream_to_jsonl_segment() {
        let dir = tempfile::tempdir().unwrap();
        let nrj = dir.path().join("records.nrj");
        let jsonl = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj, WriteOptions::default()).unwrap();
        writer.append(record()).unwrap();
        writer.finish().unwrap();

        export_jsonl(&nrj, &jsonl, true).unwrap();

        let mut reader = JsonlSegmentReader::open(&jsonl).unwrap();
        let entry = reader.read_next().unwrap().unwrap();
        assert_eq!(entry.seq, 1);
        assert_eq!(entry.record.statement.predicate, "resource.classified");
        assert!(reader.read_next().unwrap().is_none());
        assert!(jsonl.with_extension("jsonl.seal.json").exists());
    }

    #[test]
    fn verifies_jsonl_segment_and_source_binding() {
        let dir = tempfile::tempdir().unwrap();
        let nrj = dir.path().join("records.nrj");
        let jsonl = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj, WriteOptions::default()).unwrap();
        writer.append(record()).unwrap();
        writer.finish().unwrap();
        export_jsonl(&nrj, &jsonl, true).unwrap();

        verify_jsonl(&jsonl, true, true).unwrap();
    }

    #[test]
    fn verify_jsonl_requires_source_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let nrj = dir.path().join("records.nrj");
        let jsonl = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj, WriteOptions::default()).unwrap();
        writer.append(record()).unwrap();
        writer.finish().unwrap();
        export_jsonl(&nrj, &jsonl, true).unwrap();
        std::fs::remove_file(&nrj).unwrap();

        assert!(verify_jsonl(&jsonl, true, true).is_err());
        assert!(verify_jsonl(&jsonl, false, true).is_ok());
    }
}
