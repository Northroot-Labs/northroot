use crate::id::sha256_hex;
use crate::{record_canonical_bytes, validate_record, Record, ValidationError};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// One canonical JSONL segment entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentEntry {
    /// Contiguous sequence number within the stream.
    pub seq: u64,
    /// Record at this sequence.
    pub record: Record,
}

/// Seal metadata for an immutable JSONL segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentSeal {
    /// Seal schema identifier.
    pub schema: String,
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
}

/// Append-only writer for canonical JSONL record segments.
pub struct JsonlSegmentWriter {
    path: PathBuf,
    seal_path: PathBuf,
    writer: BufWriter<File>,
    next_seq: u64,
}

impl JsonlSegmentWriter {
    /// Creates a new unsealed JSONL segment.
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

/// Reader for canonical JSONL record segments.
pub struct JsonlSegmentReader {
    reader: BufReader<File>,
    next_seq: Option<u64>,
}

impl JsonlSegmentReader {
    /// Opens a JSONL record segment.
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
        let entry: SegmentEntry = serde_json::from_str(line.trim_end())?;
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

/// Creates an adjacent seal file for a completed JSONL segment.
///
/// # Errors
///
/// Fails if the segment cannot be read, contains invalid entries, or the seal
/// cannot be written.
pub fn seal_segment(path: impl AsRef<Path>) -> Result<SegmentSeal, JournalError> {
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
    let seal = SegmentSeal {
        schema: "northroot.segment-seal.v0".to_string(),
        segment_ref: path.to_string_lossy().to_string(),
        first_seq: first_seq.unwrap_or(0),
        last_seq,
        record_count: count,
        segment_digest: sha256_hex(&bytes),
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
    let seal: SegmentSeal = serde_json::from_reader(File::open(seal_path)?)?;
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
}

fn canonical_entry_line(entry: &SegmentEntry) -> Result<Vec<u8>, JournalError> {
    let value = serde_json::to_value(entry)?;
    let profile = northroot_canonical::ProfileId::parse("northroot-canonical-v1")
        .map_err(|err| crate::RecordIdError::Serialization(err.to_string()))?;
    let canonicalizer = northroot_canonical::Canonicalizer::new(profile);
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
        let mut record = Record::new(
            RecordRole::Event,
            Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: "resource.classified".to_string(),
                object: "resource:document:seed-report".to_string(),
            },
            Context {
                node_id: Some("node:apd_croptrak_2026".to_string()),
                time: Some("2026-06-14T18:00:00Z".to_string()),
                ..Context::default()
            },
            RecordRefs {
                inputs: vec!["resource:document:seed-report".to_string()],
                outputs: vec!["resource:classification:xyz".to_string()],
                evidence: vec!["attestation:policy-decision-123".to_string()],
                causes: vec!["event:sha256:abc".to_string()],
            },
            json!({}),
        );
        record.id = compute_record_id(&record).unwrap();
        record
    }

    #[test]
    fn jsonl_segment_round_trips_and_seals() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut writer = JsonlSegmentWriter::create(&path, 1).unwrap();
        writer.append(event_record()).unwrap();
        writer.flush().unwrap();

        let seal = seal_segment(&path).unwrap();
        assert_eq!(seal.first_seq, 1);
        assert_eq!(seal.last_seq, 1);
        assert_eq!(seal.record_count, 1);
        assert!(verify_segment_seal(&path).is_ok());
    }
}
