use crate::id::verify_record_id;
use crate::record::{Record, RecordRole, RECORD_SCHEMA_V0};
use northroot_canonical::{Canonicalizer, ProfileId};

/// Record validation error.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// Schema is not known to this core.
    #[error("unknown schema: {0}")]
    UnknownSchema(String),
    /// Required field is missing or empty.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Record identifier is malformed.
    #[error("malformed record id: {0}")]
    MalformedId(String),
    /// Record identifier does not match canonical content.
    #[error("record id does not match content hash")]
    IdMismatch,
    /// Reference has the wrong type prefix.
    #[error("typed ref mismatch at {field}: expected {expected}, found {found}")]
    TypedRefMismatch {
        /// Field being validated.
        field: &'static str,
        /// Required prefix.
        expected: &'static str,
        /// Supplied value.
        found: String,
    },
    /// Timestamp is not accepted by the core validator.
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    /// Canonical payload validation failed.
    #[error("payload is not canonicalizable JSON: {0}")]
    PayloadNotCanonical(String),
    /// Identifier computation failed.
    #[error("record id computation failed: {0}")]
    IdComputation(String),
}

/// Validates the Core V0 record invariants.
///
/// This function checks only boring guarantees: known schema/role, required
/// statement fields, required event/attestation context, typed refs,
/// canonicalizable payload JSON, and content-derived record ID.
///
/// # Errors
///
/// Returns [`ValidationError`] when any core invariant is violated.
pub fn validate_record(record: &Record) -> Result<(), ValidationError> {
    if record.schema != RECORD_SCHEMA_V0 {
        return Err(ValidationError::UnknownSchema(record.schema.clone()));
    }
    validate_non_empty("id", &record.id)?;
    validate_id_shape(&record.id)?;
    validate_non_empty("statement.subject", &record.statement.subject)?;
    validate_non_empty("statement.predicate", &record.statement.predicate)?;
    validate_non_empty("statement.object", &record.statement.object)?;

    if matches!(record.role, RecordRole::Event | RecordRole::Attestation) {
        let node_id = record
            .context
            .node_id
            .as_deref()
            .ok_or(ValidationError::MissingField("context.node_id"))?;
        validate_non_empty("context.node_id", node_id)?;
        let time = record
            .context
            .time
            .as_deref()
            .ok_or(ValidationError::MissingField("context.time"))?;
        validate_timestamp(time)?;
    }

    for input in &record.refs.inputs {
        validate_typed_ref("refs.inputs", "resource:", input)?;
    }
    for output in &record.refs.outputs {
        validate_typed_ref("refs.outputs", "resource:", output)?;
    }
    for evidence in &record.refs.evidence {
        validate_typed_ref("refs.evidence", "attestation:", evidence)?;
    }
    for cause in &record.refs.causes {
        validate_typed_ref("refs.causes", "event:", cause)?;
    }

    let profile = ProfileId::parse("northroot-canonical-v1")
        .map_err(|err| ValidationError::PayloadNotCanonical(err.to_string()))?;
    Canonicalizer::new(profile)
        .canonicalize(&record.payload)
        .map_err(|err| ValidationError::PayloadNotCanonical(err.to_string()))?;

    let matches =
        verify_record_id(record).map_err(|err| ValidationError::IdComputation(err.to_string()))?;
    if !matches {
        return Err(ValidationError::IdMismatch);
    }

    Ok(())
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::MissingField(field))
    } else {
        Ok(())
    }
}

fn validate_id_shape(value: &str) -> Result<(), ValidationError> {
    let hex = value
        .strip_prefix("sha256:")
        .ok_or_else(|| ValidationError::MalformedId(value.to_string()))?;
    if hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ValidationError::MalformedId(value.to_string()))
    }
}

fn validate_typed_ref(
    field: &'static str,
    expected: &'static str,
    value: &str,
) -> Result<(), ValidationError> {
    if value.starts_with(expected) && value.len() > expected.len() {
        Ok(())
    } else {
        Err(ValidationError::TypedRefMismatch {
            field,
            expected,
            found: value.to_string(),
        })
    }
}

fn validate_timestamp(value: &str) -> Result<(), ValidationError> {
    let bytes = value.as_bytes();
    let looks_rfc3339_utc = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(idx, byte)| matches!(idx, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit());
    if looks_rfc3339_utc {
        Ok(())
    } else {
        Err(ValidationError::InvalidTimestamp(value.to_string()))
    }
}
