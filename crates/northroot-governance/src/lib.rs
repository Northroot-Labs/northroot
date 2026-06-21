//! Governance matching over Northroot records.
//!
//! Policies are records. This crate only matches policy record payload patterns
//! against command records. It does not decide whether a command should execute,
//! enforce a policy, or attach domain meaning to predicates.

#![deny(missing_docs)]

use northroot_record::{validate_record, MethodKind, Record, RecordRole};
use serde::{Deserialize, Serialize};

/// Payload shape used by the boring governance matcher.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyPayload {
    /// Pattern to match against command records.
    #[serde(rename = "match")]
    pub match_: PolicyPattern,
    /// Optional effect label interpreted by a governance engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
}

/// Record pattern for policy matching.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyPattern {
    /// Optional subject match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Optional predicate match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    /// Optional object match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Optional workspace match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Optional custody class match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody_class: Option<String>,
    /// Optional method kind match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_kind: Option<MethodKind>,
}

/// Matched policy metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyMatch {
    /// Matching policy record identifier.
    pub policy_id: String,
    /// Optional policy effect label.
    pub effect: Option<String>,
}

/// Governance matcher error.
#[derive(thiserror::Error, Debug)]
pub enum GovernanceError {
    /// Record failed core validation.
    #[error("record validation failed: {0}")]
    Record(#[from] northroot_record::ValidationError),
    /// A policy record had an unexpected role.
    #[error("expected policy record, found {0:?}")]
    ExpectedPolicy(RecordRole),
    /// A command record had an unexpected role.
    #[error("expected command record, found {0:?}")]
    ExpectedCommand(RecordRole),
    /// Policy payload did not match the matcher schema.
    #[error("invalid policy payload: {0}")]
    Payload(#[from] serde_json::Error),
}

/// Returns all policy records that match a command record.
///
/// # Errors
///
/// Returns [`GovernanceError`] for invalid records or policy payloads.
pub fn matching_policies<'a, I>(
    policies: I,
    command: &Record,
) -> Result<Vec<PolicyMatch>, GovernanceError>
where
    I: IntoIterator<Item = &'a Record>,
{
    validate_record(command)?;
    if command.role != RecordRole::Command {
        return Err(GovernanceError::ExpectedCommand(command.role.clone()));
    }

    let mut matches = Vec::new();
    for policy in policies {
        if policy_matches_command(policy, command)? {
            let payload: PolicyPayload = serde_json::from_value(policy.payload.clone())?;
            matches.push(PolicyMatch {
                policy_id: policy.id.clone(),
                effect: payload.effect,
            });
        }
    }
    Ok(matches)
}

/// Returns whether a policy record matches a command record.
///
/// # Errors
///
/// Returns [`GovernanceError`] for invalid records or policy payloads.
pub fn policy_matches_command(policy: &Record, command: &Record) -> Result<bool, GovernanceError> {
    validate_record(policy)?;
    validate_record(command)?;
    if policy.role != RecordRole::Policy {
        return Err(GovernanceError::ExpectedPolicy(policy.role.clone()));
    }
    if command.role != RecordRole::Command {
        return Err(GovernanceError::ExpectedCommand(command.role.clone()));
    }
    let payload: PolicyPayload = serde_json::from_value(policy.payload.clone())?;
    Ok(matches_pattern(&payload.match_, command))
}

fn matches_pattern(pattern: &PolicyPattern, command: &Record) -> bool {
    optional_eq(pattern.subject.as_deref(), &command.statement.subject)
        && optional_eq(pattern.predicate.as_deref(), &command.statement.predicate)
        && optional_eq(pattern.object.as_deref(), &command.statement.object)
        && optional_eq(
            pattern.workspace_id.as_deref(),
            command
                .context
                .scope
                .as_ref()
                .map(|scope| scope.workspace_id.as_str())
                .unwrap_or_default(),
        )
        && optional_eq(
            pattern.custody_class.as_deref(),
            command
                .context
                .scope
                .as_ref()
                .map(|scope| scope.custody_class.as_str())
                .unwrap_or_default(),
        )
        && pattern.method_kind.as_ref().is_none_or(|kind| {
            command
                .context
                .method
                .as_ref()
                .map(|method| &method.kind == kind)
                .unwrap_or(false)
        })
}

fn optional_eq(expected: Option<&str>, found: &str) -> bool {
    expected.is_none_or(|value| value == found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_record::{
        compute_record_id, Context, Method, Record, RecordRefs, Scope, Statement, RECORD_SCHEMA_V0,
    };
    use serde_json::json;

    fn with_id(mut record: Record) -> Record {
        record.id = compute_record_id(&record).unwrap();
        record
    }

    #[test]
    fn matches_policy_over_command_shape() {
        let command = with_id(Record {
            schema: RECORD_SCHEMA_V0.to_string(),
            id: String::new(),
            profiles: Vec::new(),
            role: RecordRole::Command,
            statement: Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: "resource.classified".to_string(),
                object: "resource:document:seed-report".to_string(),
            },
            context: Context {
                scope: Some(Scope {
                    workspace_id: "workspace:ag-demo".to_string(),
                    custody_class: "restricted".to_string(),
                }),
                method: Some(Method {
                    kind: MethodKind::Tool,
                    ref_: "tool:classify-document".to_string(),
                }),
                ..Context::default()
            },
            refs: RecordRefs::default(),
            payload: json!({}),
        });
        let policy = with_id(Record {
            schema: RECORD_SCHEMA_V0.to_string(),
            id: String::new(),
            profiles: Vec::new(),
            role: RecordRole::Policy,
            statement: Statement {
                subject: "entity:governance:local".to_string(),
                predicate: "policy.applies".to_string(),
                object: "resource:policy:review".to_string(),
            },
            context: Context::default(),
            refs: RecordRefs::default(),
            payload: json!({
                "match": {
                    "predicate": "resource.classified",
                    "custody_class": "restricted",
                    "method_kind": "tool"
                },
                "effect": "requires_review"
            }),
        });

        let matches = matching_policies([&policy], &command).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].effect.as_deref(), Some("requires_review"));
    }
}
