use crate::id::sha256_hex;
use crate::{record_canonical_bytes, validate_record, Record, ValidationError};
use northroot_canonical::{compute_event_id, parse_json_strict, Canonicalizer, ProfileId};
use northroot_journal::{verify_event_id, JournalReader, JournalWriter, ReadMode, WriteOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const RECORD_APPENDED_EVENT_TYPE: &str = "northroot.record.appended";
const RECORD_APPENDED_EVENT_VERSION: &str = "0";
const CANONICAL_PROFILE_ID: &str = "northroot-canonical-v1";
const SEGMENT_SEAL_SCHEMA: &str = "northroot.segment-seal.v0";
const JSONL_SEGMENT_REPRESENTATION: &str = "canonical-jsonl-segment";

/// One record stream entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentEntry {
    /// Contiguous sequence number within the stream.
    pub seq: u64,
    /// Record at this sequence.
    pub record: Record,
}

/// Seal metadata for an immutable JSONL interchange segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentSeal {
    /// Seal schema identifier.
    pub schema: String,
    /// Segment representation.
    pub representation: String,
    /// Segment file reference.
    pub segment_ref: String,
    /// First sequence number in the segment.
    pub first_seq: u64,
    /// Last sequence number in the segment.
    pub last_seq: u64,
    /// Number of records in the segment.
    pub record_count: u64,
    /// SHA-256 digest of exact segment bytes.
    pub segment_digest: String,
    /// Source `.nrj` stream reference, when this segment was exported from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_journal_ref: Option<String>,
    /// SHA-256 digest of exact source `.nrj` bytes, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_journal_digest: Option<String>,
}

/// Verification summary for an authoritative `.nrj` record stream.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordStreamSummary {
    /// Number of verified records in the stream.
    pub record_count: u64,
    /// First verified record sequence number.
    pub first_seq: Option<u64>,
    /// Last verified record sequence number.
    pub last_seq: Option<u64>,
}

/// Import summary for a sealed JSONL segment imported into `.nrj`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonlImportSummary {
    /// Number of imported records.
    pub imported_record_count: u64,
    /// First imported JSONL sequence number.
    pub input_first_seq: Option<u64>,
    /// Last imported JSONL sequence number.
    pub input_last_seq: Option<u64>,
    /// First authoritative `.nrj` sequence number written by this import.
    pub output_first_seq: Option<u64>,
    /// Last authoritative `.nrj` sequence number written by this import.
    pub output_last_seq: Option<u64>,
    /// Verified input segment seal.
    pub input_seal: SegmentSeal,
}

/// Source `.nrj` binding report for a JSONL segment seal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceJournalReport {
    /// Source `.nrj` path recorded in the segment seal.
    pub source_journal_ref: Option<String>,
    /// Expected source `.nrj` digest recorded in the segment seal.
    pub expected_source_journal_digest: Option<String>,
    /// Actual source `.nrj` digest, when the source file is available.
    pub actual_source_journal_digest: Option<String>,
    /// Whether the source binding matched. `None` means no source binding was declared.
    pub valid: Option<bool>,
    /// Source binding error, when one was detected without aborting segment verification.
    pub error: Option<String>,
}

/// Verification report for a sealed JSONL segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonlSegmentVerification {
    /// Whether the verification satisfied the requested source-binding policy.
    pub valid: bool,
    /// Verified segment seal.
    pub seal: SegmentSeal,
    /// Optional source `.nrj` binding report.
    pub source: SourceJournalReport,
}

/// Append-only writer for authoritative `.nrj` record streams.
pub struct NrjRecordWriter {
    writer: JournalWriter,
    next_seq: u64,
}

impl NrjRecordWriter {
    /// Opens an authoritative `.nrj` record stream for appending.
    ///
    /// Existing streams are scanned to continue the next record sequence.
    ///
    /// # Errors
    ///
    /// Fails if the `.nrj` stream cannot be opened, read, verified, or written.
    pub fn open(path: impl AsRef<Path>, options: WriteOptions) -> Result<Self, JournalError> {
        let path = path.as_ref();
        let next_seq = if options.append
            && path.exists()
            && path.metadata()?.len() >= northroot_journal::JournalHeader::HEADER_SIZE as u64
        {
            let mut reader = NrjRecordReader::open(path, ReadMode::Strict)?;
            let mut next_seq = 1;
            while let Some(entry) = reader.read_next()? {
                next_seq = entry
                    .seq
                    .checked_add(1)
                    .ok_or(JournalError::SequenceOverflow)?;
            }
            next_seq
        } else {
            1
        };

        Ok(Self {
            writer: JournalWriter::open(path, options)?,
            next_seq,
        })
    }

    /// Appends a validated record to the `.nrj` stream.
    ///
    /// # Errors
    ///
    /// Fails when the record is invalid, event identity cannot be computed, or
    /// the backing `.nrj` append fails.
    pub fn append(&mut self, record: Record) -> Result<u64, JournalError> {
        validate_record(&record)?;
        let seq = self.next_seq;
        let event = record_appended_event(seq, &record)?;
        self.writer.append_event(&event)?;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or(JournalError::SequenceOverflow)?;
        Ok(seq)
    }

    /// Finishes writing the backing `.nrj` stream.
    ///
    /// # Errors
    ///
    /// Returns I/O or sync errors from the backing writer.
    pub fn finish(self) -> Result<(), JournalError> {
        self.writer.finish()?;
        Ok(())
    }
}

/// Reader for authoritative `.nrj` record streams.
pub struct NrjRecordReader {
    reader: JournalReader,
    next_seq: Option<u64>,
    canonicalizer: Canonicalizer,
}

impl NrjRecordReader {
    /// Opens an authoritative `.nrj` record stream.
    ///
    /// # Errors
    ///
    /// Returns errors if the backing journal cannot be opened.
    pub fn open(path: impl AsRef<Path>, mode: ReadMode) -> Result<Self, JournalError> {
        Ok(Self {
            reader: JournalReader::open(path, mode)?,
            next_seq: None,
            canonicalizer: canonicalizer()?,
        })
    }

    /// Reads and verifies the next record entry from the stream.
    ///
    /// Non-record events are skipped only after their kernel event identity has
    /// been verified.
    ///
    /// # Errors
    ///
    /// Fails on invalid event identity, invalid record payload, or
    /// non-contiguous record sequence numbers.
    pub fn read_next(&mut self) -> Result<Option<SegmentEntry>, JournalError> {
        loop {
            let Some(event) = self.reader.read_event()? else {
                return Ok(None);
            };
            if !verify_event_id(&event, &self.canonicalizer)? {
                return Err(JournalError::EventIdMismatch);
            }
            if event.get("event_type").and_then(Value::as_str) != Some(RECORD_APPENDED_EVENT_TYPE) {
                continue;
            }
            let entry = event_to_segment_entry(event)?;
            validate_record(&entry.record)?;
            validate_next_seq(self.next_seq, entry.seq)?;
            self.next_seq = Some(
                entry
                    .seq
                    .checked_add(1)
                    .ok_or(JournalError::SequenceOverflow)?,
            );
            return Ok(Some(entry));
        }
    }
}

/// Verifies an authoritative `.nrj` record stream and returns its summary.
///
/// Verification is stricter than generic journal verification: every frame is
/// read in strict mode, every event identity is checked, record wrapper events
/// must have the supported shape, records must validate, and record sequence
/// numbers must be contiguous.
///
/// # Errors
///
/// Fails when the stream cannot be read as `.nrj`, an event identity is invalid,
/// a record wrapper is malformed, a record is invalid, or record sequence
/// numbers are not contiguous.
pub fn verify_nrj_record_stream(
    path: impl AsRef<Path>,
) -> Result<RecordStreamSummary, JournalError> {
    let mut reader = NrjRecordReader::open(path, ReadMode::Strict)?;
    let mut summary = RecordStreamSummary::default();
    while let Some(entry) = reader.read_next()? {
        summary.record_count = summary
            .record_count
            .checked_add(1)
            .ok_or(JournalError::SequenceOverflow)?;
        summary.first_seq.get_or_insert(entry.seq);
        summary.last_seq = Some(entry.seq);
    }
    Ok(summary)
}

/// Imports a sealed JSONL interchange segment into an authoritative `.nrj` stream.
///
/// The adjacent `<segment>.seal.json` is verified before the `.nrj` writer is
/// opened. This is the SDK-level crossing from boring line-oriented interchange
/// into kernel-backed authoritative state.
///
/// # Errors
///
/// Fails if the input segment is unsealed, the seal does not match the segment,
/// any record is invalid, the segment sequence is non-contiguous, or the target
/// `.nrj` stream cannot be written.
pub fn import_jsonl_segment_to_nrj_records(
    segment_path: impl AsRef<Path>,
    nrj_path: impl AsRef<Path>,
    options: WriteOptions,
) -> Result<JsonlImportSummary, JournalError> {
    let segment_path = segment_path.as_ref();
    let nrj_path = nrj_path.as_ref();
    let seal = verify_segment_seal(segment_path)?;
    let mut reader = JsonlSegmentReader::open(segment_path)?;
    let mut writer = NrjRecordWriter::open(nrj_path, options)?;
    let mut imported_record_count = 0u64;
    let mut input_first_seq = None;
    let mut input_last_seq = None;
    let mut output_first_seq = None;
    let mut output_last_seq = None;

    while let Some(entry) = reader.read_next()? {
        input_first_seq.get_or_insert(entry.seq);
        input_last_seq = Some(entry.seq);
        let output_seq = writer.append(entry.record)?;
        output_first_seq.get_or_insert(output_seq);
        output_last_seq = Some(output_seq);
        imported_record_count = imported_record_count
            .checked_add(1)
            .ok_or(JournalError::SequenceOverflow)?;
    }
    writer.finish()?;

    Ok(JsonlImportSummary {
        imported_record_count,
        input_first_seq,
        input_last_seq,
        output_first_seq,
        output_last_seq,
        input_seal: seal,
    })
}

/// Exports an authoritative `.nrj` record stream to a canonical JSONL segment.
///
/// The JSONL segment is an interchange/debug representation. The returned seal
/// records the exact source `.nrj` byte digest so verifiers can bind the export
/// back to the kernel stream.
///
/// # Errors
///
/// Fails if the source stream cannot be read and verified, or if the segment
/// cannot be written and sealed.
pub fn export_nrj_records_to_jsonl_segment(
    nrj_path: impl AsRef<Path>,
    segment_path: impl AsRef<Path>,
) -> Result<SegmentSeal, JournalError> {
    let nrj_path = nrj_path.as_ref();
    let segment_path = segment_path.as_ref();
    let mut reader = NrjRecordReader::open(nrj_path, ReadMode::Strict)?;
    let first_entry = reader.read_next()?;
    let first_seq = first_entry.as_ref().map(|entry| entry.seq).unwrap_or(0);
    let mut writer = JsonlSegmentWriter::create(segment_path, first_seq)?;
    if let Some(entry) = first_entry {
        writer.append(entry.record)?;
    }
    while let Some(entry) = reader.read_next()? {
        writer.append(entry.record)?;
    }
    writer.flush()?;
    seal_segment_with_source(segment_path, Some(nrj_path))
}

/// Append-only writer for canonical JSONL interchange segments.
///
/// JSONL segments are portable exports over the kernel stream. They are not the
/// authoritative Northroot log when an `.nrj` source is available.
pub struct JsonlSegmentWriter {
    path: PathBuf,
    seal_path: PathBuf,
    writer: BufWriter<File>,
    next_seq: u64,
}

impl JsonlSegmentWriter {
    /// Creates a new unsealed JSONL interchange segment.
    ///
    /// # Errors
    ///
    /// Fails if the segment already exists or if an adjacent seal exists.
    pub fn create(path: impl AsRef<Path>, first_seq: u64) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let seal_path = seal_path_for(&path);
        if seal_path.exists() {
            return Err(JournalError::SealedSegment(path));
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            path,
            seal_path,
            writer: BufWriter::new(file),
            next_seq: first_seq,
        })
    }

    /// Appends a validated record with the next contiguous sequence number.
    ///
    /// # Errors
    ///
    /// Fails when the segment is sealed, the record is invalid, or writing fails.
    pub fn append(&mut self, record: Record) -> Result<u64, JournalError> {
        if self.seal_path.exists() {
            return Err(JournalError::SealedSegment(self.path.clone()));
        }
        validate_record(&record)?;
        let seq = self.next_seq;
        let entry = SegmentEntry { seq, record };
        let line = canonical_entry_line(&entry)?;
        self.writer.write_all(&line)?;
        self.writer.write_all(b"\n")?;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or(JournalError::SequenceOverflow)?;
        Ok(seq)
    }

    /// Flushes segment bytes to disk.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying writer.
    pub fn flush(&mut self) -> Result<(), JournalError> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Reader for canonical JSONL interchange segments.
pub struct JsonlSegmentReader {
    reader: BufReader<File>,
    next_seq: Option<u64>,
}

impl JsonlSegmentReader {
    /// Opens a JSONL interchange segment.
    ///
    /// # Errors
    ///
    /// Returns I/O errors if the segment cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        Ok(Self {
            reader: BufReader::new(File::open(path)?),
            next_seq: None,
        })
    }

    /// Reads and validates the next segment entry.
    ///
    /// # Errors
    ///
    /// Fails on invalid JSON, invalid record, or non-contiguous sequence.
    pub fn read_next(&mut self) -> Result<Option<SegmentEntry>, JournalError> {
        let mut line = String::new();
        let read = self.reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }
        let value = parse_json_strict(line.trim_end())
            .map_err(|err| JournalError::InvalidRecordEvent(err.to_string()))?;
        let entry: SegmentEntry = serde_json::from_value(value)?;
        validate_record(&entry.record)?;
        if let Some(expected) = self.next_seq {
            if entry.seq != expected {
                return Err(JournalError::NonContiguousSeq {
                    expected,
                    found: entry.seq,
                });
            }
        }
        self.next_seq = Some(
            entry
                .seq
                .checked_add(1)
                .ok_or(JournalError::SequenceOverflow)?,
        );
        Ok(Some(entry))
    }
}

/// Creates an adjacent seal file for a completed JSONL interchange segment.
///
/// # Errors
///
/// Fails if the segment cannot be read, contains invalid entries, or the seal
/// cannot be written.
pub fn seal_segment(path: impl AsRef<Path>) -> Result<SegmentSeal, JournalError> {
    seal_segment_with_source(path, None::<&Path>)
}

fn seal_segment_with_source(
    path: impl AsRef<Path>,
    source_journal_path: Option<impl AsRef<Path>>,
) -> Result<SegmentSeal, JournalError> {
    let path = path.as_ref();
    let mut reader = JsonlSegmentReader::open(path)?;
    let mut first_seq = None;
    let mut last_seq = 0;
    let mut count = 0;
    while let Some(entry) = reader.read_next()? {
        first_seq.get_or_insert(entry.seq);
        last_seq = entry.seq;
        count += 1;
    }
    let bytes = fs::read(path)?;
    let source_journal_ref = source_journal_path
        .as_ref()
        .map(|source| source.as_ref().to_string_lossy().to_string());
    let source_journal_digest = source_journal_path
        .as_ref()
        .map(|source| fs::read(source.as_ref()).map(|bytes| sha256_hex(&bytes)))
        .transpose()?;
    let seal = SegmentSeal {
        schema: SEGMENT_SEAL_SCHEMA.to_string(),
        representation: JSONL_SEGMENT_REPRESENTATION.to_string(),
        segment_ref: path.to_string_lossy().to_string(),
        first_seq: first_seq.unwrap_or(0),
        last_seq,
        record_count: count,
        segment_digest: sha256_hex(&bytes),
        source_journal_ref,
        source_journal_digest,
    };
    let seal_path = seal_path_for(path);
    let seal_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(seal_path)?;
    serde_json::to_writer_pretty(seal_file, &seal)?;
    Ok(seal)
}

/// Verifies that a sealed segment still matches its seal metadata.
///
/// # Errors
///
/// Fails if the segment or seal cannot be read, the segment is invalid, or the
/// digest/sequence metadata has changed.
pub fn verify_segment_seal(path: impl AsRef<Path>) -> Result<SegmentSeal, JournalError> {
    let path = path.as_ref();
    let seal_path = seal_path_for(path);
    let seal_bytes = fs::read_to_string(seal_path)?;
    let seal_value = parse_json_strict(&seal_bytes)
        .map_err(|err| JournalError::InvalidSegmentSeal(err.to_string()))?;
    let seal: SegmentSeal = serde_json::from_value(seal_value)?;
    if seal.schema != SEGMENT_SEAL_SCHEMA {
        return Err(JournalError::InvalidSegmentSeal(format!(
            "schema must be {SEGMENT_SEAL_SCHEMA}"
        )));
    }
    if seal.representation != JSONL_SEGMENT_REPRESENTATION {
        return Err(JournalError::InvalidSegmentSeal(format!(
            "representation must be {JSONL_SEGMENT_REPRESENTATION}"
        )));
    }
    verify_segment_ref(path, &seal)?;
    let mut reader = JsonlSegmentReader::open(path)?;
    let mut first_seq = None;
    let mut last_seq = 0;
    let mut count = 0;
    while let Some(entry) = reader.read_next()? {
        first_seq.get_or_insert(entry.seq);
        last_seq = entry.seq;
        count += 1;
    }
    let bytes = fs::read(path)?;
    let digest = sha256_hex(&bytes);
    if seal.segment_digest != digest
        || seal.first_seq != first_seq.unwrap_or(0)
        || seal.last_seq != last_seq
        || seal.record_count != count
    {
        return Err(JournalError::SealMismatch);
    }
    Ok(seal)
}

/// Verifies a sealed JSONL segment and optionally requires its source `.nrj` binding.
///
/// Segment integrity is always mandatory. Source `.nrj` binding status is
/// reported separately so tools can inspect detached JSONL segments without
/// pretending the source journal was available.
///
/// # Errors
///
/// Fails when the segment or seal cannot be read, the segment is invalid, or
/// the seal metadata mismatches the segment. Source binding failure is reported
/// as `valid: false` when `require_source` is true.
pub fn verify_jsonl_segment(
    path: impl AsRef<Path>,
    require_source: bool,
) -> Result<JsonlSegmentVerification, JournalError> {
    let seal = verify_segment_seal(path)?;
    let source = verify_source_journal_binding(&seal)?;
    let valid = !require_source || source.valid == Some(true);
    Ok(JsonlSegmentVerification {
        valid,
        seal,
        source,
    })
}

fn verify_source_journal_binding(seal: &SegmentSeal) -> Result<SourceJournalReport, JournalError> {
    let Some(source_ref) = seal.source_journal_ref.clone() else {
        return Ok(SourceJournalReport {
            source_journal_ref: None,
            expected_source_journal_digest: None,
            actual_source_journal_digest: None,
            valid: None,
            error: None,
        });
    };
    let expected = seal.source_journal_digest.clone();
    let source_path = PathBuf::from(&source_ref);
    if !source_path.exists() {
        return Ok(SourceJournalReport {
            source_journal_ref: Some(source_ref),
            expected_source_journal_digest: expected,
            actual_source_journal_digest: None,
            valid: Some(false),
            error: Some("source journal not found".to_string()),
        });
    }
    let actual = sha256_hex(&fs::read(&source_path)?);
    let valid = expected
        .as_ref()
        .is_some_and(|expected| expected == &actual);
    Ok(SourceJournalReport {
        source_journal_ref: Some(source_ref),
        expected_source_journal_digest: expected,
        actual_source_journal_digest: Some(actual),
        valid: Some(valid),
        error: None,
    })
}

fn verify_segment_ref(path: &Path, seal: &SegmentSeal) -> Result<(), JournalError> {
    let actual_path = path
        .canonicalize()
        .map_err(|err| JournalError::InvalidSegmentSeal(err.to_string()))?;
    let recorded_path = PathBuf::from(&seal.segment_ref);
    let recorded_path = if recorded_path.is_absolute() {
        recorded_path
    } else {
        seal_path_for(path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(recorded_path)
    };
    let recorded_path = recorded_path
        .canonicalize()
        .map_err(|err| JournalError::InvalidSegmentSeal(err.to_string()))?;
    if actual_path != recorded_path {
        return Err(JournalError::InvalidSegmentSeal(
            "segment_ref must reference verified segment".to_string(),
        ));
    }
    Ok(())
}

/// Journal segment error.
#[derive(thiserror::Error, Debug)]
pub enum JournalError {
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Validation failed.
    #[error("record validation failed: {0}")]
    Validation(#[from] ValidationError),
    /// Identifier computation failed.
    #[error("record id failed: {0}")]
    RecordId(#[from] crate::RecordIdError),
    /// Sequence numbers are not contiguous.
    #[error("non-contiguous sequence: expected {expected}, found {found}")]
    NonContiguousSeq {
        /// Expected sequence.
        expected: u64,
        /// Found sequence.
        found: u64,
    },
    /// Sequence overflowed.
    #[error("sequence overflow")]
    SequenceOverflow,
    /// Segment has already been sealed.
    #[error("segment is sealed: {0}")]
    SealedSegment(PathBuf),
    /// Segment no longer matches its seal.
    #[error("segment seal mismatch")]
    SealMismatch,
    /// Segment seal is not a supported Northroot segment seal.
    #[error("invalid segment seal: {0}")]
    InvalidSegmentSeal(String),
    /// Backing `.nrj` journal failed.
    #[error("nrj journal error: {0}")]
    Nrj(#[from] northroot_journal::JournalError),
    /// Record wrapper event did not match its claimed event identity.
    #[error("record wrapper event id mismatch")]
    EventIdMismatch,
    /// Record wrapper event has an invalid field.
    #[error("invalid record wrapper event: {0}")]
    InvalidRecordEvent(String),
    /// Event identifier computation failed.
    #[error("event id failed: {0}")]
    EventId(String),
}

fn record_appended_event(seq: u64, record: &Record) -> Result<Value, JournalError> {
    let canonicalizer = canonicalizer()?;
    let mut event = json!({
        "event_type": RECORD_APPENDED_EVENT_TYPE,
        "event_version": RECORD_APPENDED_EVENT_VERSION,
        "canonical_profile_id": CANONICAL_PROFILE_ID,
        "seq": seq,
        "record": record,
    });
    let event_id = compute_event_id(&event, &canonicalizer)
        .map_err(|err| JournalError::EventId(err.to_string()))?;
    event["event_id"] = serde_json::to_value(event_id)?;
    Ok(event)
}

fn event_to_segment_entry(event: Value) -> Result<SegmentEntry, JournalError> {
    if event.get("event_version").and_then(Value::as_str) != Some(RECORD_APPENDED_EVENT_VERSION) {
        return Err(JournalError::InvalidRecordEvent(
            "event_version must be 0".to_string(),
        ));
    }
    if event.get("canonical_profile_id").and_then(Value::as_str) != Some(CANONICAL_PROFILE_ID) {
        return Err(JournalError::InvalidRecordEvent(format!(
            "canonical_profile_id must be {CANONICAL_PROFILE_ID}"
        )));
    }
    let seq = event
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| JournalError::InvalidRecordEvent("seq must be a u64".to_string()))?;
    let record_value = event
        .get("record")
        .ok_or_else(|| JournalError::InvalidRecordEvent("record is required".to_string()))?
        .clone();
    let record = serde_json::from_value(record_value)?;
    Ok(SegmentEntry { seq, record })
}

fn validate_next_seq(expected: Option<u64>, found: u64) -> Result<(), JournalError> {
    if let Some(expected) = expected {
        if found != expected {
            return Err(JournalError::NonContiguousSeq { expected, found });
        }
    }
    Ok(())
}

fn canonicalizer() -> Result<Canonicalizer, JournalError> {
    let profile = ProfileId::parse(CANONICAL_PROFILE_ID)
        .map_err(|err| crate::RecordIdError::Serialization(err.to_string()))?;
    Ok(Canonicalizer::new(profile))
}

fn canonical_entry_line(entry: &SegmentEntry) -> Result<Vec<u8>, JournalError> {
    let value = serde_json::to_value(entry)?;
    let canonicalizer = canonicalizer()?;
    let _ = record_canonical_bytes(&entry.record)?;
    Ok(canonicalizer
        .canonicalize(&value)
        .map_err(crate::RecordIdError::from)?
        .bytes)
}

fn seal_path_for(path: &Path) -> PathBuf {
    let mut seal = path.as_os_str().to_os_string();
    seal.push(".seal.json");
    PathBuf::from(seal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compute_record_id, Context, Record, RecordRefs, RecordRole, Statement};
    use serde_json::json;

    fn event_record() -> Record {
        let cause_id =
            "event:sha256:1111111111111111111111111111111111111111111111111111111111111111";
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
                evidence: vec!["attestation:policy-decision-123".to_string()],
                causes: vec![cause_id.to_string()],
            },
            json!({}),
        );
        record.id = compute_record_id(&record).unwrap();
        record
    }

    #[test]
    fn nrj_record_stream_round_trips_and_exports_jsonl_segment() {
        let dir = tempfile::tempdir().unwrap();
        let nrj_path = dir.path().join("records.nrj");
        let jsonl_path = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default()).unwrap();
        writer.append(event_record()).unwrap();
        writer.finish().unwrap();

        let mut reader = NrjRecordReader::open(&nrj_path, ReadMode::Strict).unwrap();
        let entry = reader.read_next().unwrap().unwrap();
        assert_eq!(entry.seq, 1);
        assert_eq!(entry.record.statement.predicate, "resource.classified");
        assert!(reader.read_next().unwrap().is_none());

        let summary = verify_nrj_record_stream(&nrj_path).unwrap();
        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.first_seq, Some(1));
        assert_eq!(summary.last_seq, Some(1));

        let seal = export_nrj_records_to_jsonl_segment(&nrj_path, &jsonl_path).unwrap();
        assert_eq!(seal.first_seq, 1);
        assert_eq!(seal.last_seq, 1);
        assert_eq!(seal.record_count, 1);
        assert_eq!(
            seal.source_journal_ref.as_deref(),
            Some(nrj_path.to_str().unwrap())
        );
        assert!(seal.source_journal_digest.is_some());
        assert!(verify_segment_seal(&jsonl_path).is_ok());
    }

    #[test]
    fn nrj_record_stream_rejects_wrong_canonical_profile_id() {
        let dir = tempfile::tempdir().unwrap();
        let nrj_path = dir.path().join("records.nrj");
        let mut writer = JournalWriter::open(&nrj_path, WriteOptions::default()).unwrap();
        let canonicalizer = canonicalizer().unwrap();
        let mut event = json!({
            "event_type": RECORD_APPENDED_EVENT_TYPE,
            "event_version": RECORD_APPENDED_EVENT_VERSION,
            "canonical_profile_id": "northroot-other-v1",
            "seq": 1,
            "record": event_record(),
        });
        let event_id = compute_event_id(&event, &canonicalizer).unwrap();
        event["event_id"] = serde_json::to_value(event_id).unwrap();
        writer.append_event(&event).unwrap();
        writer.finish().unwrap();

        assert!(matches!(
            verify_nrj_record_stream(&nrj_path),
            Err(JournalError::InvalidRecordEvent(_))
        ));
    }

    #[test]
    fn jsonl_segment_round_trips_as_interchange_export() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        let seal = seal_segment(&path).unwrap();
        assert_eq!(seal.first_seq, 1);
        assert_eq!(seal.last_seq, 1);
        assert_eq!(seal.record_count, 1);
        assert_eq!(seal.representation, "canonical-jsonl-segment");
        assert!(seal.source_journal_ref.is_none());
        assert!(verify_segment_seal(&path).is_ok());
    }

    #[test]
    fn imports_sealed_jsonl_segment_to_nrj_record_stream() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl_path = dir.path().join("records.jsonl");
        let nrj_path = dir.path().join("records.nrj");
        let mut writer = JsonlSegmentWriter::create(&jsonl_path, 7).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();
        seal_segment(&jsonl_path).unwrap();

        let summary =
            import_jsonl_segment_to_nrj_records(&jsonl_path, &nrj_path, WriteOptions::default())
                .unwrap();

        assert_eq!(summary.imported_record_count, 1);
        assert_eq!(summary.input_first_seq, Some(7));
        assert_eq!(summary.input_last_seq, Some(7));
        assert_eq!(summary.output_first_seq, Some(1));
        assert_eq!(summary.output_last_seq, Some(1));
        assert_eq!(summary.input_seal.first_seq, 7);
        assert_eq!(
            verify_nrj_record_stream(&nrj_path).unwrap(),
            RecordStreamSummary {
                record_count: 1,
                first_seq: Some(1),
                last_seq: Some(1),
            }
        );
    }

    #[test]
    fn import_summary_reports_output_range_when_appending_to_existing_nrj() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl_path = dir.path().join("records.jsonl");
        let nrj_path = dir.path().join("records.nrj");

        let mut nrj_writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default()).unwrap();
        nrj_writer.append(event_record()).unwrap();
        nrj_writer.finish().unwrap();

        let mut jsonl_writer = JsonlSegmentWriter::create(&jsonl_path, 50).unwrap();
        jsonl_writer.append(event_record()).unwrap();
        jsonl_writer.flush().unwrap();
        seal_segment(&jsonl_path).unwrap();

        let summary =
            import_jsonl_segment_to_nrj_records(&jsonl_path, &nrj_path, WriteOptions::default())
                .unwrap();

        assert_eq!(summary.input_first_seq, Some(50));
        assert_eq!(summary.input_last_seq, Some(50));
        assert_eq!(summary.output_first_seq, Some(2));
        assert_eq!(summary.output_last_seq, Some(2));
        assert_eq!(
            verify_nrj_record_stream(&nrj_path).unwrap(),
            RecordStreamSummary {
                record_count: 2,
                first_seq: Some(1),
                last_seq: Some(2),
            }
        );
    }

    #[test]
    fn import_rejects_unsealed_jsonl_before_creating_nrj() {
        let dir = tempfile::tempdir().unwrap();
        let jsonl_path = dir.path().join("records.jsonl");
        let nrj_path = dir.path().join("records.nrj");
        let mut writer = JsonlSegmentWriter::create(&jsonl_path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        assert!(matches!(
            import_jsonl_segment_to_nrj_records(&jsonl_path, &nrj_path, WriteOptions::default()),
            Err(JournalError::Io(_))
        ));
        assert!(!nrj_path.exists());
    }

    #[test]
    fn verifies_jsonl_segment_with_source_binding_report() {
        let dir = tempfile::tempdir().unwrap();
        let nrj_path = dir.path().join("records.nrj");
        let jsonl_path = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default()).unwrap();
        writer.append(event_record()).unwrap();
        writer.finish().unwrap();
        export_nrj_records_to_jsonl_segment(&nrj_path, &jsonl_path).unwrap();

        let verification = verify_jsonl_segment(&jsonl_path, true).unwrap();

        assert!(verification.valid);
        assert_eq!(verification.seal.record_count, 1);
        assert_eq!(verification.source.valid, Some(true));
        assert_eq!(
            verification.source.source_journal_ref.as_deref(),
            Some(nrj_path.to_str().unwrap())
        );
    }

    #[test]
    fn jsonl_segment_source_binding_can_be_reported_without_being_required() {
        let dir = tempfile::tempdir().unwrap();
        let nrj_path = dir.path().join("records.nrj");
        let jsonl_path = dir.path().join("records.jsonl");

        let mut writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default()).unwrap();
        writer.append(event_record()).unwrap();
        writer.finish().unwrap();
        export_nrj_records_to_jsonl_segment(&nrj_path, &jsonl_path).unwrap();
        fs::remove_file(&nrj_path).unwrap();

        let detached = verify_jsonl_segment(&jsonl_path, false).unwrap();
        assert!(detached.valid);
        assert_eq!(detached.source.valid, Some(false));
        assert_eq!(
            detached.source.error.as_deref(),
            Some("source journal not found")
        );

        let required = verify_jsonl_segment(&jsonl_path, true).unwrap();
        assert!(!required.valid);
        assert_eq!(required.source.valid, Some(false));
    }

    #[test]
    fn segment_seal_verification_rejects_unknown_representation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        let mut seal = seal_segment(&path).unwrap();
        seal.representation = "plain-jsonl".to_string();
        let seal_path = seal_path_for(&path);
        let seal_file = File::create(seal_path).unwrap();
        serde_json::to_writer_pretty(seal_file, &seal).unwrap();

        assert!(matches!(
            verify_segment_seal(&path),
            Err(JournalError::InvalidSegmentSeal(_))
        ));
    }

    #[test]
    fn segment_seal_verification_rejects_mismatched_segment_ref() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let other_path = dir.path().join("other.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();
        fs::write(&other_path, b"\n").unwrap();

        let mut seal = seal_segment(&path).unwrap();
        seal.segment_ref = other_path.to_string_lossy().to_string();
        let seal_file = File::create(seal_path_for(&path)).unwrap();
        serde_json::to_writer_pretty(seal_file, &seal).unwrap();

        assert!(matches!(
            verify_segment_seal(&path),
            Err(JournalError::InvalidSegmentSeal(_))
        ));
    }

    #[test]
    fn segment_seal_verification_rejects_duplicate_seal_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        let seal = seal_segment(&path).unwrap();
        let duplicate_schema_seal = format!(
            concat!(
                "{{",
                "\"schema\":\"northroot.segment-seal.v0\",",
                "\"schema\":\"northroot.segment-seal.v0\",",
                "\"representation\":{},",
                "\"segment_ref\":{},",
                "\"first_seq\":{},",
                "\"last_seq\":{},",
                "\"record_count\":{},",
                "\"segment_digest\":{}",
                "}}"
            ),
            serde_json::to_string(&seal.representation).unwrap(),
            serde_json::to_string(&seal.segment_ref).unwrap(),
            seal.first_seq,
            seal.last_seq,
            seal.record_count,
            serde_json::to_string(&seal.segment_digest).unwrap(),
        );
        fs::write(seal_path_for(&path), duplicate_schema_seal).unwrap();

        assert!(matches!(
            verify_segment_seal(&path),
            Err(JournalError::InvalidSegmentSeal(_))
        ));
    }

    #[test]
    fn jsonl_segment_reader_rejects_unknown_entry_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut entry = json!({
            "seq": 1,
            "record": event_record(),
        });
        entry["unexpected"] = json!("ignored before strict serde");
        fs::write(
            &path,
            format!("{}\n", serde_json::to_string(&entry).unwrap()),
        )
        .unwrap();

        let mut reader = JsonlSegmentReader::open(&path).unwrap();
        assert!(matches!(reader.read_next(), Err(JournalError::Json(_))));
    }

    #[test]
    fn segment_seal_verification_rejects_unknown_seal_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        let seal = seal_segment(&path).unwrap();
        let mut seal_value = serde_json::to_value(&seal).unwrap();
        seal_value["unexpected"] = json!("ignored before strict serde");
        fs::write(
            seal_path_for(&path),
            serde_json::to_string_pretty(&seal_value).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            verify_segment_seal(&path),
            Err(JournalError::Json(_))
        ));
    }
}
