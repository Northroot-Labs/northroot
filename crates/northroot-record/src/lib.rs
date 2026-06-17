//! Tiny Northroot record contract plus deterministic validators.
//!
//! This crate owns the Core V0 record shape:
//! `Record`, `Statement`, `Context`, `Refs`, and `Payload`.
//! It deliberately avoids domain policy, execution behavior, or application
//! vocabulary. Profiles and applications constrain records in higher crates.

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
    seal_segment, verify_segment_seal, JournalError, JsonlSegmentReader, JsonlSegmentWriter,
    SegmentEntry, SegmentSeal,
};
pub use record::{
    Authority, Context, Method, MethodKind, Record, RecordRefs, RecordRole, Scope, Statement,
    RECORD_SCHEMA_V0,
};
pub use validate::{validate_record, ValidationError};
