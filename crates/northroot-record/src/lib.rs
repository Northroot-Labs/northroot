//! Tiny Northroot record contract plus deterministic validators.
//!
//! This crate owns the Core V0 record shape:
//! `Record`, `Statement`, `Context`, `Refs`, and `Payload`.
//! It deliberately avoids domain policy, execution behavior, or application
//! vocabulary. Profiles and applications constrain records in higher crates.
//!
//! ## Authoritative `.nrj`, boring JSONL interchange
//!
//! Applications should define ordinary record payloads. The SDK can persist
//! those records to an authoritative `.nrj` stream and export sealed JSONL
//! segments for line-oriented tools, review, fixtures, and interchange.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use northroot_journal::WriteOptions;
//! use northroot_record::{
//!     compute_record_id, export_nrj_records_to_jsonl_segment,
//!     import_jsonl_segment_to_nrj_records, verify_jsonl_segment,
//!     verify_nrj_record_stream, Context, NrjRecordWriter, Record, RecordRefs,
//!     RecordRole, Statement,
//! };
//! use serde_json::json;
//!
//! let mut record = Record::new(
//!     RecordRole::Event,
//!     Statement {
//!         subject: "entity:node:local".to_string(),
//!         predicate: "artifact.observed".to_string(),
//!         object: "resource:artifact:seed".to_string(),
//!     },
//!     Context {
//!         node_id: Some("node:local".to_string()),
//!         time: Some("2026-06-14T18:00:00Z".to_string()),
//!         ..Context::default()
//!     },
//!     RecordRefs::default(),
//!     json!({ "size": 42 }),
//! );
//! record.id = compute_record_id(&record)?;
//!
//! let nrj_path = std::env::temp_dir().join("northroot-records.nrj");
//! let jsonl_path = std::env::temp_dir().join("northroot-records.jsonl");
//! let imported_path = std::env::temp_dir().join("northroot-records-imported.nrj");
//!
//! let mut writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default())?;
//! writer.append(record)?;
//! writer.finish()?;
//!
//! let summary = verify_nrj_record_stream(&nrj_path)?;
//! assert_eq!(summary.record_count, 1);
//!
//! export_nrj_records_to_jsonl_segment(&nrj_path, &jsonl_path)?;
//! let verification = verify_jsonl_segment(&jsonl_path, true)?;
//! assert!(verification.valid);
//!
//! let import_summary = import_jsonl_segment_to_nrj_records(
//!     &jsonl_path,
//!     &imported_path,
//!     WriteOptions::default(),
//! )?;
//! assert_eq!(import_summary.imported_record_count, 1);
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]

mod id;
mod journal;
mod record;
mod validate;

pub use id::{
    compute_record_id, is_content_id, record_canonical_bytes, verify_record_id, ContentId,
    RecordIdError,
};
pub use journal::{
    export_nrj_records_to_jsonl_segment, import_jsonl_segment_to_nrj_records, seal_segment,
    verify_jsonl_segment, verify_nrj_record_stream, verify_segment_seal, JournalError,
    JsonlImportSummary, JsonlSegmentReader, JsonlSegmentVerification, JsonlSegmentWriter,
    NrjRecordReader, NrjRecordWriter, RecordStreamSummary, SegmentEntry, SegmentSeal,
    SourceJournalReport,
};
pub use record::{
    Authority, Context, Method, MethodKind, Record, RecordRefs, RecordRole, Scope, Statement,
    RECORD_SCHEMA_V0,
};
pub use validate::{validate_record, ValidationError};
