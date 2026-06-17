use crate::id::{is_content_id, verify_record_id};
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
    /// Content identifier is malformed.
    #[error("malformed content id at {field}: {value}")]
    MalformedContentId {
        /// Field being validated.
        field: &'static str,
        /// Supplied value.
        value: String,
    },
    /// Profile identifier is malformed.
    #[error("malformed profile id at {field}: {value}")]
    MalformedProfileId {
        /// Field being validated.
        field: &'static str,
        /// Supplied value.
        value: String,
    },
    /// Typed identifier is malformed.
    #[error("malformed typed id at {field}: {value}")]
    MalformedTypedId {
        /// Field being validated.
        field: &'static str,
        /// Supplied value.
        value: String,
    },
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
    validate_content_id("id", &record.id)?;
    for profile in &record.profiles {
        validate_profile_id("profiles[]", profile)?;
    }
    validate_non_empty("statement.subject", &record.statement.subject)?;
    validate_non_empty("statement.predicate", &record.statement.predicate)?;
    validate_non_empty("statement.object", &record.statement.object)?;
    validate_subject_or_object("statement.subject", &record.statement.subject)?;
    validate_predicate("statement.predicate", &record.statement.predicate)?;
    validate_subject_or_object("statement.object", &record.statement.object)?;

    if matches!(record.role, RecordRole::Event | RecordRole::Attestation) {
        let node_id = record
            .context
            .node_id
            .as_deref()
            .ok_or(ValidationError::MissingField("context.node_id"))?;
        validate_non_empty("context.node_id", node_id)?;
        validate_typed_id("context.node_id", "node", node_id)?;
        let time = record
            .context
            .time
            .as_deref()
            .ok_or(ValidationError::MissingField("context.time"))?;
        validate_timestamp(time)?;
    }
    if let Some(node_id) = record.context.node_id.as_deref() {
        validate_typed_id("context.node_id", "node", node_id)?;
    }
    if let Some(time) = record.context.time.as_deref() {
        validate_timestamp(time)?;
    }
    if let Some(intent) = record.context.intent.as_deref() {
        validate_token("context.intent", intent)?;
    }
    if let Some(scope) = &record.context.scope {
        validate_typed_id(
            "context.scope.workspace_id",
            "workspace",
            &scope.workspace_id,
        )?;
        validate_token("context.scope.custody_class", &scope.custody_class)?;
    }
    if let Some(method) = &record.context.method {
        validate_method_ref("context.method.ref", &method.kind, &method.ref_)?;
    }
    if let Some(authority) = &record.context.authority {
        validate_resource_ref("context.authority.grant_ref", &authority.grant_ref)?;
    }
    for key in record.context.extra.keys() {
        validate_extension_key("context.extra key", key)?;
    }

    for input in &record.refs.inputs {
        validate_resource_ref("refs.inputs", input)?;
    }
    for output in &record.refs.outputs {
        validate_resource_ref("refs.outputs", output)?;
    }
    for evidence in &record.refs.evidence {
        validate_typed_id("refs.evidence", "attestation", evidence)?;
    }
    for cause in &record.refs.causes {
        validate_event_ref("refs.causes", cause)?;
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

fn validate_content_id(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if is_content_id(value) {
        Ok(())
    } else {
        Err(ValidationError::MalformedContentId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_profile_id(field: &'static str, value: &str) -> Result<(), ValidationError> {
    let mut parts = value.split('.').peekable();
    let mut count = 0;
    while let Some(part) = parts.next() {
        count += 1;
        if parts.peek().is_none() {
            let version = part.strip_prefix('v').unwrap_or_default();
            if !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit()) {
                break;
            }
            return Err(ValidationError::MalformedProfileId {
                field,
                value: value.to_string(),
            });
        }
        if !is_name_segment(part) {
            return Err(ValidationError::MalformedProfileId {
                field,
                value: value.to_string(),
            });
        }
    }
    if count >= 2 {
        Ok(())
    } else {
        Err(ValidationError::MalformedProfileId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_subject_or_object(field: &'static str, value: &str) -> Result<(), ValidationError> {
    let Some((kind, _)) = value.split_once(':') else {
        return Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        });
    };
    match kind {
        "entity" => validate_typed_id(field, "entity", value),
        "resource" => validate_typed_id(field, "resource", value),
        "attestation" => validate_typed_id(field, "attestation", value),
        "node" => validate_typed_id(field, "node", value),
        "workspace" => validate_typed_id(field, "workspace", value),
        "policy" => validate_typed_id(field, "policy", value),
        "projection" => validate_typed_id(field, "projection", value),
        "tool" => validate_typed_id(field, "tool", value),
        "process" => validate_typed_id(field, "process", value),
        "connector" => validate_typed_id(field, "connector", value),
        "grant" => validate_typed_id(field, "grant", value),
        "value" => validate_typed_id(field, "value", value),
        "event" => validate_event_ref(field, value),
        _ => Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        }),
    }
}

fn validate_predicate(field: &'static str, value: &str) -> Result<(), ValidationError> {
    let parts = value.split('.');
    let mut count = 0;
    for part in parts {
        count += 1;
        if !is_name_segment(part) {
            return Err(ValidationError::MalformedTypedId {
                field,
                value: value.to_string(),
            });
        }
    }
    if count >= 2 {
        Ok(())
    } else {
        Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_resource_ref(field: &'static str, value: &str) -> Result<(), ValidationError> {
    validate_typed_id(field, "resource", value)
}

fn validate_event_ref(field: &'static str, value: &str) -> Result<(), ValidationError> {
    let Some(id) = value.strip_prefix("event:") else {
        return Err(ValidationError::TypedRefMismatch {
            field,
            expected: "event:sha256:<64 lowercase hex>",
            found: value.to_string(),
        });
    };
    if is_content_id(id) {
        Ok(())
    } else {
        Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_method_ref(
    field: &'static str,
    kind: &crate::MethodKind,
    value: &str,
) -> Result<(), ValidationError> {
    let expected = match kind {
        crate::MethodKind::PureFunction => "pure_function",
        crate::MethodKind::Tool => "tool",
        crate::MethodKind::Process => "process",
        crate::MethodKind::Connector => "connector",
        crate::MethodKind::HumanAction => "human_action",
    };
    validate_typed_id(field, expected, value)
}

fn validate_typed_id(
    field: &'static str,
    expected_kind: &'static str,
    value: &str,
) -> Result<(), ValidationError> {
    let Some(rest) = value.strip_prefix(expected_kind) else {
        return Err(ValidationError::TypedRefMismatch {
            field,
            expected: expected_kind,
            found: value.to_string(),
        });
    };
    let Some(rest) = rest.strip_prefix(':') else {
        return Err(ValidationError::TypedRefMismatch {
            field,
            expected: expected_kind,
            found: value.to_string(),
        });
    };
    if !rest.is_empty() && rest.split(':').all(is_name_segment) {
        Ok(())
    } else {
        Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_token(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if is_name_segment(value) {
        Ok(())
    } else {
        Err(ValidationError::MalformedTypedId {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_extension_key(field: &'static str, value: &str) -> Result<(), ValidationError> {
    validate_predicate(field, value)
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
    if looks_rfc3339_utc && calendar_parts_are_valid(bytes) {
        Ok(())
    } else {
        Err(ValidationError::InvalidTimestamp(value.to_string()))
    }
}

fn calendar_parts_are_valid(bytes: &[u8]) -> bool {
    let year = parse_digits(&bytes[0..4]);
    let month = parse_digits(&bytes[5..7]);
    let day = parse_digits(&bytes[8..10]);
    let hour = parse_digits(&bytes[11..13]);
    let minute = parse_digits(&bytes[14..16]);
    let second = parse_digits(&bytes[17..19]);

    (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(year, month)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn parse_digits(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0, |value, byte| (value * 10) + u32::from(byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn is_name_segment(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
