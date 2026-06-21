//! Sanitized agricultural profile example over Northroot records.
//!
//! This crate is intentionally above core. It owns example agricultural
//! vocabulary such as custody classes, domain types, and profile predicates
//! without changing the Core V0 record shape.

#![deny(missing_docs)]

use northroot_record::{validate_record, Record};

/// Seed evaluation completed predicate.
pub const AG_SEED_EVAL_COMPLETED: &str = "ag.seed_eval.completed";
/// Field review package prepared predicate.
pub const AG_FIELD_REVIEW_PREPARED: &str = "ag.field_review.prepared";
/// Agricultural artifact classified predicate.
pub const AG_ARTIFACT_CLASSIFIED: &str = "ag.artifact.classified";

/// Supported agricultural profile custody classes.
pub const CUSTODY_CLASSES: &[&str] = &["public", "internal", "restricted", "regulated"];
/// Supported agricultural profile domain types.
pub const DOMAIN_TYPES: &[&str] = &["crop_plan", "seed_eval", "field_observation"];

/// Validates an agricultural profile record.
///
/// # Errors
///
/// Returns [`AgProfileError`] when core validation or profile constraints fail.
pub fn validate_ag_record(record: &Record) -> Result<(), AgProfileError> {
    validate_record(record)?;
    if !is_ag_predicate(&record.statement.predicate) {
        return Err(AgProfileError::UnknownPredicate(
            record.statement.predicate.clone(),
        ));
    }
    let custody_class = record
        .context
        .scope
        .as_ref()
        .map(|scope| scope.custody_class.as_str())
        .ok_or(AgProfileError::MissingField("context.scope.custody_class"))?;
    if !CUSTODY_CLASSES.contains(&custody_class) {
        return Err(AgProfileError::UnknownCustodyClass(
            custody_class.to_string(),
        ));
    }
    if let Some(domain_type) = record
        .payload
        .get("domain_type")
        .and_then(|value| value.as_str())
    {
        if !DOMAIN_TYPES.contains(&domain_type) {
            return Err(AgProfileError::UnknownDomainType(domain_type.to_string()));
        }
    }
    Ok(())
}

/// Agricultural profile validation error.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum AgProfileError {
    /// Core validation failed.
    #[error("record validation failed: {0}")]
    Record(#[from] northroot_record::ValidationError),
    /// Required field is absent.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Predicate is not owned by this profile.
    #[error("unknown ag predicate: {0}")]
    UnknownPredicate(String),
    /// Custody class is not supported by this profile.
    #[error("unknown custody class: {0}")]
    UnknownCustodyClass(String),
    /// Domain type is not supported by this profile.
    #[error("unknown domain type: {0}")]
    UnknownDomainType(String),
}

fn is_ag_predicate(predicate: &str) -> bool {
    matches!(
        predicate,
        AG_SEED_EVAL_COMPLETED | AG_FIELD_REVIEW_PREPARED | AG_ARTIFACT_CLASSIFIED
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_record::{
        compute_record_id, Context, Record, RecordRefs, RecordRole, Scope, Statement,
    };
    use serde_json::json;

    #[test]
    fn ag_vocabulary_stays_above_core() {
        let mut record = Record::new(
            RecordRole::Event,
            Statement {
                subject: "entity:principal:operator".to_string(),
                predicate: AG_SEED_EVAL_COMPLETED.to_string(),
                object: "resource:seed_eval:trial-2026".to_string(),
            },
            Context {
                node_id: Some("node:ag_demo_2026".to_string()),
                time: Some("2026-06-14T18:00:00Z".to_string()),
                scope: Some(Scope {
                    workspace_id: "workspace:ag-demo".to_string(),
                    custody_class: "restricted".to_string(),
                }),
                ..Context::default()
            },
            RecordRefs {
                inputs: vec!["resource:document:agronomy-report".to_string()],
                outputs: vec!["resource:seed_eval:trial-2026".to_string()],
                ..RecordRefs::default()
            },
            json!({"domain_type": "seed_eval"}),
        );
        record.id = compute_record_id(&record).unwrap();
        validate_ag_record(&record).unwrap();
    }
}
