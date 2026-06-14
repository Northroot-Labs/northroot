//! Exchange record profile.
//!
//! Exchange is a constrained record protocol over the Core V0 record shape. It
//! validates predicates, roles, refs, and context required for handoff and
//! result flows, without adding application semantics.

#![deny(missing_docs)]

use northroot_record::{validate_record, Record, RecordRole};

/// Handoff submitted predicate.
pub const HANDOFF_SUBMITTED: &str = "handoff.submitted";
/// Handoff accepted predicate.
pub const HANDOFF_ACCEPTED: &str = "handoff.accepted";
/// Handoff rejected predicate.
pub const HANDOFF_REJECTED: &str = "handoff.rejected";
/// Attestation issued predicate.
pub const ATTESTATION_ISSUED: &str = "attestation.issued";
/// Result retrieved predicate.
pub const RESULT_RETRIEVED: &str = "result.retrieved";

/// Validates an Exchange profile record.
///
/// # Errors
///
/// Returns [`ExchangeError`] when core validation or profile constraints fail.
pub fn validate_exchange_record(record: &Record) -> Result<(), ExchangeError> {
    validate_record(record)?;
    require_context(record)?;
    match record.statement.predicate.as_str() {
        HANDOFF_SUBMITTED => {
            require_role(record, RecordRole::Command)?;
            require_non_empty_refs("refs.inputs", &record.refs.inputs)?;
        }
        HANDOFF_ACCEPTED | HANDOFF_REJECTED => {
            require_role(record, RecordRole::Event)?;
            require_non_empty_refs("refs.causes", &record.refs.causes)?;
        }
        ATTESTATION_ISSUED => {
            require_role(record, RecordRole::Attestation)?;
            require_non_empty_refs("refs.evidence", &record.refs.evidence)?;
        }
        RESULT_RETRIEVED => {
            require_role(record, RecordRole::Event)?;
            require_non_empty_refs("refs.outputs", &record.refs.outputs)?;
            require_non_empty_refs("refs.causes", &record.refs.causes)?;
        }
        other => return Err(ExchangeError::UnknownPredicate(other.to_string())),
    }
    Ok(())
}

/// Exchange profile validation error.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ExchangeError {
    /// Core record validation failed.
    #[error("record validation failed: {0}")]
    Record(#[from] northroot_record::ValidationError),
    /// Predicate is not in the Exchange profile.
    #[error("unknown exchange predicate: {0}")]
    UnknownPredicate(String),
    /// Record role is wrong for its exchange predicate.
    #[error("role mismatch: expected {expected:?}, found {found:?}")]
    RoleMismatch {
        /// Expected role.
        expected: RecordRole,
        /// Found role.
        found: RecordRole,
    },
    /// Required profile field is absent.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
}

fn require_role(record: &Record, expected: RecordRole) -> Result<(), ExchangeError> {
    if record.role == expected {
        Ok(())
    } else {
        Err(ExchangeError::RoleMismatch {
            expected,
            found: record.role.clone(),
        })
    }
}

fn require_context(record: &Record) -> Result<(), ExchangeError> {
    if record
        .context
        .node_id
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return Err(ExchangeError::MissingField("context.node_id"));
    }
    if record
        .context
        .time
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return Err(ExchangeError::MissingField("context.time"));
    }
    if record.context.scope.is_none() {
        return Err(ExchangeError::MissingField("context.scope"));
    }
    Ok(())
}

fn require_non_empty_refs(field: &'static str, values: &[String]) -> Result<(), ExchangeError> {
    if values.is_empty() {
        Err(ExchangeError::MissingField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_record::{
        compute_record_id, Context, Record, RecordRefs, RecordRole, Scope, Statement,
    };
    use serde_json::json;

    #[test]
    fn validates_handoff_submitted_profile() {
        let mut record = Record::new(
            RecordRole::Command,
            Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: HANDOFF_SUBMITTED.to_string(),
                object: "resource:handoff:seed-report".to_string(),
            },
            Context {
                node_id: Some("node:apd_croptrak_2026".to_string()),
                time: Some("2026-06-14T18:00:00Z".to_string()),
                scope: Some(Scope {
                    workspace_id: "workspace:clientops-local".to_string(),
                    custody_class: "client_sensitive".to_string(),
                }),
                ..Context::default()
            },
            RecordRefs {
                inputs: vec!["resource:document:seed-report".to_string()],
                ..RecordRefs::default()
            },
            json!({}),
        );
        record.id = compute_record_id(&record).unwrap();
        validate_exchange_record(&record).unwrap();
    }
}
