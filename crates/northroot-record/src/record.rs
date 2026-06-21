use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Schema identifier for the Core V0 record contract.
pub const RECORD_SCHEMA_V0: &str = "northroot.record.v0";

/// Core record role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordRole {
    /// A requested action. Policy and execution layers may interpret it.
    Command,
    /// A fact about what happened.
    Event,
    /// Evidence or assertion attached to another record/resource.
    Attestation,
    /// A policy artifact represented as data, interpreted above core.
    Policy,
    /// A derived state or view over a record prefix.
    Projection,
}

/// Northroot Core V0 record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// Schema identifier. Must be [`RECORD_SCHEMA_V0`].
    pub schema: String,
    /// Content identifier in `sha256:<hex>` form.
    pub id: String,
    /// Declared profile identifiers layered over the core record.
    ///
    /// Core validates profile identifier grammar only. Profile resolution and
    /// interpretation belong to profile/governance/application layers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
    /// Record role.
    pub role: RecordRole,
    /// Subject-predicate-object statement.
    pub statement: Statement,
    /// Context for the statement.
    pub context: Context,
    /// Typed references to related records/resources.
    #[serde(default)]
    pub refs: RecordRefs,
    /// Opaque canonical JSON payload owned by profiles/applications.
    #[serde(default)]
    pub payload: Value,
}

impl Record {
    /// Creates a record with an empty `id` field ready for content hashing.
    pub fn new(
        role: RecordRole,
        statement: Statement,
        context: Context,
        refs: RecordRefs,
        payload: Value,
    ) -> Self {
        Self {
            schema: RECORD_SCHEMA_V0.to_string(),
            id: String::new(),
            profiles: Vec::new(),
            role,
            statement,
            context,
            refs,
            payload,
        }
    }
}

/// Subject-predicate-object statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    /// Entity or resource being described.
    pub subject: String,
    /// Predicate string. Core validates presence, not meaning.
    pub predicate: String,
    /// Entity, resource, or value being linked.
    pub object: String,
}

/// Context carried by a record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    /// Node identifier. Required for event and attestation records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// RFC3339 UTC timestamp. Required for event and attestation records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Human or system intent label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Workspace and custody scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    /// Method description. Execution is outside core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<Method>,
    /// Authority reference. Interpretation is outside core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<Authority>,
    /// Extra context fields reserved for profiles.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

/// Workspace and custody scope convention.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Workspace identifier.
    pub workspace_id: String,
    /// Custody class label. Core validates presence only.
    pub custody_class: String,
}

/// Execution method descriptor carried as record context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    /// Method kind.
    pub kind: MethodKind,
    /// Method reference.
    #[serde(rename = "ref")]
    pub ref_: String,
}

/// Known execution method kinds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodKind {
    /// Deterministic pure function.
    PureFunction,
    /// Tool invocation.
    Tool,
    /// Local or remote process.
    Process,
    /// External connector.
    Connector,
    /// Human action.
    HumanAction,
}

/// Authority reference carried as context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    /// Grant reference.
    pub grant_ref: String,
}

/// Typed references associated with a record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordRefs {
    /// Input resources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Output resources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    /// Evidence attestations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    /// Causal event references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<String>,
}
