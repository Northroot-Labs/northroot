//! Product-agnostic state projection and policy evaluation primitives.
//!
//! This crate intentionally does not own product policy, queues, runtimes,
//! provider adapters, databases, or observability infrastructure. It provides
//! small deterministic types and functions for:
//!
//! - identifying an ordered event prefix;
//! - folding an ordered event prefix into projected state;
//! - carrying projected state identity;
//! - composing predicate results with three-valued logic;
//! - deriving an evaluation delta from an evaluation tree;
//! - representing gate outcomes without appending events itself.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// A byte-oriented frame from an ordered event stream.
///
/// Decoding `bytes` into a domain event is an adapter concern. The eval core
/// keeps stream identity and cursor metadata so derived results can be tied
/// back to their source prefix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventFrame {
    /// Stable stream or journal identifier.
    pub stream_id: String,
    /// Ordinal in the ordered stream prefix.
    pub event_ordinal: u64,
    /// Byte offset or equivalent storage cursor, when available.
    pub byte_offset: Option<u64>,
    /// Observed event bytes.
    pub bytes: Vec<u8>,
    /// Digest of `bytes`, when supplied by an adapter or proof substrate.
    pub content_digest: Option<String>,
    /// Optional proof or journal reference for the frame.
    pub proof_ref: Option<String>,
}

/// Cursor identifying the event prefix covered by a derived value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCursor {
    /// Stable stream or journal identifier.
    pub stream_id: String,
    /// Ordinal at the tip of the covered event prefix.
    pub event_ordinal: u64,
    /// Byte offset or equivalent storage cursor at the tip, when available.
    pub byte_offset: Option<u64>,
    /// Content identity for the event at the prefix tip, when available.
    pub event_id: Option<String>,
    /// Digest of the source segment, bundle, or stream material, when available.
    pub source_digest: Option<String>,
    /// Identifier for the ordering rule used by the projector.
    pub order_basis: String,
}

/// A derived state value with explicit projection identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedState<T> {
    /// Schema identifier for `value`.
    pub schema_id: String,
    /// Projection function identifier.
    pub projection_id: String,
    /// Projection implementation version.
    pub projection_version: String,
    /// Event prefix covered by this projected state.
    pub cursor: EventCursor,
    /// Derived state value.
    pub value: T,
    /// Digest or identity of the projected state value.
    pub state_digest: String,
}

/// Request metadata for folding an ordered event prefix into projected state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionRequest<'a, T> {
    /// Schema identifier for the projected state value.
    pub schema_id: &'a str,
    /// Projection function identifier.
    pub projection_id: &'a str,
    /// Projection implementation version.
    pub projection_version: &'a str,
    /// State value before applying the supplied prefix.
    pub initial_state: T,
    /// Event cursor before applying the supplied prefix.
    pub initial_cursor: EventCursor,
}

/// Error returned when folding an event prefix into projected state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionError<E> {
    /// A frame belongs to a different stream than the prefix cursor.
    StreamIdMismatch {
        /// Stream identifier expected from the prefix cursor.
        expected: String,
        /// Stream identifier found on the frame.
        found: String,
    },
    /// A frame ordinal did not continue the ordered event prefix.
    NonContiguousOrdinal {
        /// Ordinal expected after the previous cursor tip.
        expected: u64,
        /// Ordinal found on the frame.
        found: u64,
    },
    /// Caller-supplied projection fold failed.
    Fold(E),
}

impl<E: std::fmt::Display> std::fmt::Display for ProjectionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamIdMismatch { expected, found } => {
                write!(
                    f,
                    "frame stream mismatch: expected {expected}, found {found}"
                )
            }
            Self::NonContiguousOrdinal { expected, found } => {
                write!(
                    f,
                    "frame ordinal does not continue prefix: expected {expected}, found {found}"
                )
            }
            Self::Fold(err) => write!(f, "projection fold failed: {err}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ProjectionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fold(err) => Some(err),
            Self::StreamIdMismatch { .. } | Self::NonContiguousOrdinal { .. } => None,
        }
    }
}

/// Fold an ordered event prefix into projected state.
///
/// Event decoding, domain semantics, and state digest computation are supplied
/// by the caller. This function only enforces single-stream contiguous prefix
/// order and preserves projection identity on the returned [`ProjectedState`].
pub fn project_event_prefix<'a, T, E, F, D, I>(
    request: ProjectionRequest<'_, T>,
    frames: I,
    mut fold: F,
    state_digest: D,
) -> Result<ProjectedState<T>, ProjectionError<E>>
where
    F: FnMut(T, &'a EventFrame) -> Result<T, E>,
    D: FnOnce(&T, &EventCursor) -> String,
    I: IntoIterator<Item = &'a EventFrame>,
{
    let ProjectionRequest {
        schema_id,
        projection_id,
        projection_version,
        initial_state,
        initial_cursor,
    } = request;

    let mut state = initial_state;
    let mut cursor = initial_cursor;

    for frame in frames {
        if frame.stream_id != cursor.stream_id {
            return Err(ProjectionError::StreamIdMismatch {
                expected: cursor.stream_id,
                found: frame.stream_id.clone(),
            });
        }

        let expected_ordinal =
            cursor
                .event_ordinal
                .checked_add(1)
                .ok_or(ProjectionError::NonContiguousOrdinal {
                    expected: u64::MAX,
                    found: frame.event_ordinal,
                })?;
        if frame.event_ordinal != expected_ordinal {
            return Err(ProjectionError::NonContiguousOrdinal {
                expected: expected_ordinal,
                found: frame.event_ordinal,
            });
        }

        state = fold(state, frame).map_err(ProjectionError::Fold)?;
        cursor.event_ordinal = frame.event_ordinal;
        cursor.byte_offset = frame.byte_offset;
        cursor.event_id = None;
        cursor.source_digest = frame.content_digest.clone();
    }

    let state_digest = state_digest(&state, &cursor);

    Ok(ProjectedState {
        schema_id: schema_id.to_string(),
        projection_id: projection_id.to_string(),
        projection_version: projection_version.to_string(),
        cursor,
        value: state,
        state_digest,
    })
}

/// Result of a predicate evaluated over projected state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruthValue {
    /// Predicate is satisfied by the projected state.
    True,
    /// Predicate is contradicted by the projected state.
    False,
    /// Predicate cannot be determined from the projected state and evidence.
    Unknown,
}

impl std::ops::Not for TruthValue {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

/// Compact satisfaction result for a policy expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Satisfaction {
    /// The policy expression is satisfied by projected state.
    Satisfied,
    /// The policy expression is not satisfied by projected state.
    Unsatisfied,
    /// The policy expression cannot be determined from projected state.
    Indeterminate,
}

impl From<TruthValue> for Satisfaction {
    fn from(value: TruthValue) -> Self {
        match value {
            TruthValue::True => Self::Satisfied,
            TruthValue::False => Self::Unsatisfied,
            TruthValue::Unknown => Self::Indeterminate,
        }
    }
}

/// Product-agnostic policy expression over predicate references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyExpr {
    /// Reference to an externally supplied predicate implementation/result.
    PredicateRef(String),
    /// Conjunction over child expressions.
    And(Vec<PolicyExpr>),
    /// Disjunction over child expressions.
    Or(Vec<PolicyExpr>),
    /// Negation of a child expression.
    Not(Box<PolicyExpr>),
}

/// Detailed result for one predicate reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateResult {
    /// Predicate identifier evaluated.
    pub predicate_id: String,
    /// Three-valued predicate result.
    pub truth_value: TruthValue,
    /// Human-readable or machine-stable reason strings.
    pub reasons: Vec<String>,
    /// Evidence references considered by this predicate.
    pub evidence_refs: Vec<String>,
    /// Evidence references or condition identifiers that are missing.
    pub missing_evidence: Vec<String>,
    /// Evidence references or condition identifiers with conflicts.
    pub conflicting_evidence: Vec<String>,
    /// Digest of the projected state evaluated.
    pub state_digest: String,
    /// Event prefix covered by the projected state.
    pub cursor: EventCursor,
}

impl PredicateResult {
    /// Build a predicate result with empty reason/evidence lists.
    #[must_use]
    pub fn new(
        predicate_id: impl Into<String>,
        truth_value: TruthValue,
        state_digest: impl Into<String>,
        cursor: EventCursor,
    ) -> Self {
        Self {
            predicate_id: predicate_id.into(),
            truth_value,
            reasons: Vec::new(),
            evidence_refs: Vec::new(),
            missing_evidence: Vec::new(),
            conflicting_evidence: Vec::new(),
            state_digest: state_digest.into(),
            cursor,
        }
    }
}

/// Evaluated policy tree node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationNode {
    /// Evaluated predicate leaf.
    Predicate(Box<PredicateResult>),
    /// Evaluated conjunction.
    And {
        /// Combined truth value.
        truth_value: TruthValue,
        /// Evaluated child nodes.
        children: Vec<EvaluationNode>,
    },
    /// Evaluated disjunction.
    Or {
        /// Combined truth value.
        truth_value: TruthValue,
        /// Evaluated child nodes.
        children: Vec<EvaluationNode>,
    },
    /// Evaluated negation.
    Not {
        /// Combined truth value.
        truth_value: TruthValue,
        /// Evaluated child node.
        child: Box<EvaluationNode>,
    },
}

impl EvaluationNode {
    /// Truth value at this node.
    #[must_use]
    pub fn truth_value(&self) -> TruthValue {
        match self {
            Self::Predicate(result) => result.truth_value,
            Self::And { truth_value, .. }
            | Self::Or { truth_value, .. }
            | Self::Not { truth_value, .. } => *truth_value,
        }
    }

    /// Collect predicate leaves from this node into `out`.
    pub fn collect_predicates<'a>(&'a self, out: &mut Vec<&'a PredicateResult>) {
        match self {
            Self::Predicate(result) => out.push(result.as_ref()),
            Self::And { children, .. } | Self::Or { children, .. } => {
                for child in children {
                    child.collect_predicates(out);
                }
            }
            Self::Not { child, .. } => child.collect_predicates(out),
        }
    }
}

/// Error returned when evaluating a policy expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// A predicate referenced by the policy was not provided by the caller.
    UnknownPredicate(String),
    /// An `And` expression had no children.
    EmptyAnd,
    /// An `Or` expression had no children.
    EmptyOr,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPredicate(id) => write!(f, "unknown predicate: {id}"),
            Self::EmptyAnd => f.write_str("and expression must contain at least one child"),
            Self::EmptyOr => f.write_str("or expression must contain at least one child"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Evaluate a policy expression using caller-supplied predicate results.
///
/// Predicate implementations and policy authority remain outside this crate.
pub fn evaluate_node<F>(expr: &PolicyExpr, predicate: &mut F) -> Result<EvaluationNode, EvalError>
where
    F: FnMut(&str) -> Option<PredicateResult>,
{
    match expr {
        PolicyExpr::PredicateRef(id) => predicate(id)
            .map(Box::new)
            .map(EvaluationNode::Predicate)
            .ok_or_else(|| EvalError::UnknownPredicate(id.clone())),
        PolicyExpr::And(children) => {
            if children.is_empty() {
                return Err(EvalError::EmptyAnd);
            }
            let nodes = children
                .iter()
                .map(|child| evaluate_node(child, predicate))
                .collect::<Result<Vec<_>, _>>()?;
            let truth_value = and_truth(nodes.iter().map(EvaluationNode::truth_value));
            Ok(EvaluationNode::And {
                truth_value,
                children: nodes,
            })
        }
        PolicyExpr::Or(children) => {
            if children.is_empty() {
                return Err(EvalError::EmptyOr);
            }
            let nodes = children
                .iter()
                .map(|child| evaluate_node(child, predicate))
                .collect::<Result<Vec<_>, _>>()?;
            let truth_value = or_truth(nodes.iter().map(EvaluationNode::truth_value));
            Ok(EvaluationNode::Or {
                truth_value,
                children: nodes,
            })
        }
        PolicyExpr::Not(child) => {
            let node = evaluate_node(child, predicate)?;
            let truth_value = !node.truth_value();
            Ok(EvaluationNode::Not {
                truth_value,
                child: Box::new(node),
            })
        }
    }
}

fn and_truth(values: impl IntoIterator<Item = TruthValue>) -> TruthValue {
    let mut saw_unknown = false;
    for value in values {
        match value {
            TruthValue::False => return TruthValue::False,
            TruthValue::Unknown => saw_unknown = true,
            TruthValue::True => {}
        }
    }
    if saw_unknown {
        TruthValue::Unknown
    } else {
        TruthValue::True
    }
}

fn or_truth(values: impl IntoIterator<Item = TruthValue>) -> TruthValue {
    let mut saw_unknown = false;
    for value in values {
        match value {
            TruthValue::True => return TruthValue::True,
            TruthValue::Unknown => saw_unknown = true,
            TruthValue::False => {}
        }
    }
    if saw_unknown {
        TruthValue::Unknown
    } else {
        TruthValue::False
    }
}

/// Complete policy evaluation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Evaluation identifier supplied by the caller or outer SDK.
    pub evaluation_id: String,
    /// Policy identifier evaluated.
    pub policy_id: String,
    /// Policy version evaluated.
    pub policy_version: String,
    /// Projection identifier evaluated.
    pub projection_id: String,
    /// Digest of the projected state evaluated.
    pub state_digest: String,
    /// Event prefix covered by the projected state.
    pub cursor: EventCursor,
    /// Compact satisfaction result.
    pub satisfaction: Satisfaction,
    /// Evaluated policy tree.
    pub root: EvaluationNode,
    /// Predicate results collected from the evaluated tree.
    pub predicate_results: Vec<PredicateResult>,
    /// Aggregated reason strings.
    pub reasons: Vec<String>,
    /// Predicate IDs known to be unsatisfied.
    pub missing_conditions: Vec<String>,
    /// Predicate IDs whose result is unknown.
    pub unknown_conditions: Vec<String>,
}

/// Evaluate a policy expression into an [`EvaluationResult`].
pub fn evaluate_policy<F>(
    request: EvaluationRequest<'_>,
    mut predicate: F,
) -> Result<EvaluationResult, EvalError>
where
    F: FnMut(&str) -> Option<PredicateResult>,
{
    let root = evaluate_node(request.expr, &mut predicate)?;
    let mut predicate_refs = Vec::new();
    root.collect_predicates(&mut predicate_refs);

    let predicate_results = predicate_refs
        .iter()
        .map(|result| (*result).clone())
        .collect::<Vec<_>>();
    let reasons = predicate_refs
        .iter()
        .flat_map(|result| result.reasons.iter().cloned())
        .collect::<Vec<_>>();
    let missing_conditions = predicate_refs
        .iter()
        .filter(|result| result.truth_value == TruthValue::False)
        .map(|result| result.predicate_id.clone())
        .collect::<Vec<_>>();
    let unknown_conditions = predicate_refs
        .iter()
        .filter(|result| result.truth_value == TruthValue::Unknown)
        .map(|result| result.predicate_id.clone())
        .collect::<Vec<_>>();

    Ok(EvaluationResult {
        evaluation_id: request.evaluation_id.to_string(),
        policy_id: request.policy_id.to_string(),
        policy_version: request.policy_version.to_string(),
        projection_id: request.projection_id.to_string(),
        state_digest: request.state_digest.to_string(),
        cursor: request.cursor.clone(),
        satisfaction: Satisfaction::from(root.truth_value()),
        root,
        predicate_results,
        reasons,
        missing_conditions,
        unknown_conditions,
    })
}

/// Request metadata for evaluating a policy expression.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationRequest<'a> {
    /// Evaluation identifier supplied by the caller or outer SDK.
    pub evaluation_id: &'a str,
    /// Policy identifier evaluated.
    pub policy_id: &'a str,
    /// Policy version evaluated.
    pub policy_version: &'a str,
    /// Projection identifier evaluated.
    pub projection_id: &'a str,
    /// Digest of the projected state evaluated.
    pub state_digest: &'a str,
    /// Event prefix covered by the projected state.
    pub cursor: &'a EventCursor,
    /// Policy expression to evaluate.
    pub expr: &'a PolicyExpr,
}

/// Difference between policy requirements and evaluated state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationDelta {
    /// Delta identifier supplied by the caller or outer SDK.
    pub delta_id: String,
    /// Evaluation that produced this delta.
    pub evaluation_id: String,
    /// Digest of the projected state evaluated.
    pub state_digest: String,
    /// Event prefix covered by the projected state.
    pub cursor: EventCursor,
    /// Predicate IDs known to be unsatisfied.
    pub unsatisfied_predicates: Vec<String>,
    /// Predicate IDs whose result is unknown.
    pub unknown_predicates: Vec<String>,
    /// Missing evidence references or condition identifiers.
    pub missing_evidence: Vec<String>,
    /// Evidence references or condition identifiers with conflicts.
    pub conflicting_evidence: Vec<String>,
}

impl EvaluationDelta {
    /// Derive an evaluation delta from an evaluation result.
    #[must_use]
    pub fn from_evaluation(delta_id: impl Into<String>, evaluation: &EvaluationResult) -> Self {
        let missing_evidence = evaluation
            .predicate_results
            .iter()
            .flat_map(|result| result.missing_evidence.iter().cloned())
            .collect::<Vec<_>>();
        let conflicting_evidence = evaluation
            .predicate_results
            .iter()
            .flat_map(|result| result.conflicting_evidence.iter().cloned())
            .collect::<Vec<_>>();

        Self {
            delta_id: delta_id.into(),
            evaluation_id: evaluation.evaluation_id.clone(),
            state_digest: evaluation.state_digest.clone(),
            cursor: evaluation.cursor.clone(),
            unsatisfied_predicates: evaluation.missing_conditions.clone(),
            unknown_predicates: evaluation.unknown_conditions.clone(),
            missing_evidence,
            conflicting_evidence,
        }
    }
}

/// Gate decision for a candidate action, event, or transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Candidate may proceed.
    Accept,
    /// Candidate must not proceed.
    Reject,
    /// Candidate requires review, more evidence, or another authority check.
    Hold,
}

/// Gate result for a candidate action, event, or transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    /// Gate decision.
    pub decision: GateDecision,
    /// Reason strings explaining the decision.
    pub reasons: Vec<String>,
    /// Optional evaluation result used to reach the decision.
    pub evaluation: Option<EvaluationResult>,
}

impl GateResult {
    /// Build a gate result from a policy evaluation.
    ///
    /// `Satisfied` maps to `Accept`, `Unsatisfied` maps to `Reject`, and
    /// `Indeterminate` maps to `Hold`.
    #[must_use]
    pub fn from_evaluation(evaluation: EvaluationResult) -> Self {
        let decision = match evaluation.satisfaction {
            Satisfaction::Satisfied => GateDecision::Accept,
            Satisfaction::Unsatisfied => GateDecision::Reject,
            Satisfaction::Indeterminate => GateDecision::Hold,
        };
        Self {
            reasons: evaluation.reasons.clone(),
            decision,
            evaluation: Some(evaluation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor() -> EventCursor {
        EventCursor {
            stream_id: "stream:test".to_string(),
            event_ordinal: 7,
            byte_offset: Some(128),
            event_id: Some("evt_test".to_string()),
            source_digest: Some("sha256:test".to_string()),
            order_basis: "single-stream-ordinal".to_string(),
        }
    }

    fn predicate(id: &str, value: TruthValue) -> PredicateResult {
        let mut result = PredicateResult::new(id, value, "state:test", cursor());
        if value == TruthValue::Unknown {
            result.missing_evidence.push(format!("evidence:{id}"));
        }
        if value == TruthValue::False {
            result.reasons.push(format!("{id} unsatisfied"));
        }
        result
    }

    fn initial_cursor() -> EventCursor {
        EventCursor {
            stream_id: "stream:test".to_string(),
            event_ordinal: 0,
            byte_offset: None,
            event_id: None,
            source_digest: None,
            order_basis: "single-stream-ordinal".to_string(),
        }
    }

    fn frame(ordinal: u64, bytes: &[u8]) -> EventFrame {
        EventFrame {
            stream_id: "stream:test".to_string(),
            event_ordinal: ordinal,
            byte_offset: Some(ordinal * 10),
            bytes: bytes.to_vec(),
            content_digest: Some(format!("sha256:{ordinal}")),
            proof_ref: Some(format!("nrj:test:{ordinal}")),
        }
    }

    #[test]
    fn projection_fold_builds_state_from_ordered_prefix() {
        let frames = vec![frame(1, b"alpha"), frame(2, b"beta")];
        let request = ProjectionRequest {
            schema_id: "state.schema.v1",
            projection_id: "byte_count",
            projection_version: "1",
            initial_state: 0usize,
            initial_cursor: initial_cursor(),
        };

        let projected = project_event_prefix(
            request,
            &frames,
            |count, frame| Ok::<_, std::convert::Infallible>(count + frame.bytes.len()),
            |count, cursor| format!("count:{count}:ordinal:{}", cursor.event_ordinal),
        )
        .unwrap();

        assert_eq!(projected.value, 9);
        assert_eq!(projected.schema_id, "state.schema.v1");
        assert_eq!(projected.projection_id, "byte_count");
        assert_eq!(projected.projection_version, "1");
        assert_eq!(projected.cursor.event_ordinal, 2);
        assert_eq!(projected.cursor.byte_offset, Some(20));
        assert_eq!(projected.cursor.source_digest, Some("sha256:2".to_string()));
        assert_eq!(projected.state_digest, "count:9:ordinal:2");
    }

    #[test]
    fn projection_fold_rejects_stream_mismatch() {
        let mut frames = vec![frame(1, b"alpha")];
        frames[0].stream_id = "stream:other".to_string();
        let request = ProjectionRequest {
            schema_id: "state.schema.v1",
            projection_id: "byte_count",
            projection_version: "1",
            initial_state: 0usize,
            initial_cursor: initial_cursor(),
        };

        let err = project_event_prefix(
            request,
            &frames,
            |count, frame| Ok::<_, std::convert::Infallible>(count + frame.bytes.len()),
            |count, _cursor| count.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ProjectionError::StreamIdMismatch {
                expected: "stream:test".to_string(),
                found: "stream:other".to_string(),
            }
        );
    }

    #[test]
    fn projection_fold_rejects_non_contiguous_prefix() {
        let frames = vec![frame(1, b"alpha"), frame(3, b"beta")];
        let request = ProjectionRequest {
            schema_id: "state.schema.v1",
            projection_id: "byte_count",
            projection_version: "1",
            initial_state: 0usize,
            initial_cursor: initial_cursor(),
        };

        let err = project_event_prefix(
            request,
            &frames,
            |count, frame| Ok::<_, std::convert::Infallible>(count + frame.bytes.len()),
            |count, _cursor| count.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ProjectionError::NonContiguousOrdinal {
                expected: 2,
                found: 3,
            }
        );
    }

    #[test]
    fn and_uses_three_valued_logic() {
        let expr = PolicyExpr::And(vec![
            PolicyExpr::PredicateRef("present".to_string()),
            PolicyExpr::PredicateRef("missing".to_string()),
        ]);
        let node = evaluate_node(&expr, &mut |id| match id {
            "present" => Some(predicate(id, TruthValue::True)),
            "missing" => Some(predicate(id, TruthValue::Unknown)),
            _ => None,
        })
        .unwrap();

        assert_eq!(node.truth_value(), TruthValue::Unknown);
    }

    #[test]
    fn or_short_result_is_true_when_any_child_true() {
        let expr = PolicyExpr::Or(vec![
            PolicyExpr::PredicateRef("bad".to_string()),
            PolicyExpr::PredicateRef("good".to_string()),
            PolicyExpr::PredicateRef("unknown".to_string()),
        ]);
        let node = evaluate_node(&expr, &mut |id| match id {
            "bad" => Some(predicate(id, TruthValue::False)),
            "good" => Some(predicate(id, TruthValue::True)),
            "unknown" => Some(predicate(id, TruthValue::Unknown)),
            _ => None,
        })
        .unwrap();

        assert_eq!(node.truth_value(), TruthValue::True);
    }

    #[test]
    fn not_preserves_unknown() {
        let expr = PolicyExpr::Not(Box::new(PolicyExpr::PredicateRef("unknown".to_string())));
        let node = evaluate_node(&expr, &mut |id| {
            assert_eq!(id, "unknown");
            Some(predicate(id, TruthValue::Unknown))
        })
        .unwrap();

        assert_eq!(node.truth_value(), TruthValue::Unknown);
    }

    #[test]
    fn evaluation_result_collects_reasons_and_delta() {
        let expr = PolicyExpr::And(vec![
            PolicyExpr::PredicateRef("certificate_present".to_string()),
            PolicyExpr::PredicateRef("spreadsheet_present".to_string()),
            PolicyExpr::PredicateRef("buyer_template_present".to_string()),
        ]);
        let request = EvaluationRequest {
            evaluation_id: "eval:1",
            policy_id: "submission_ready",
            policy_version: "v1",
            projection_id: "submission_state",
            state_digest: "state:test",
            cursor: &cursor(),
            expr: &expr,
        };
        let evaluation = evaluate_policy(request, |id| match id {
            "certificate_present" => Some(predicate(id, TruthValue::True)),
            "spreadsheet_present" => Some(predicate(id, TruthValue::False)),
            "buyer_template_present" => Some(predicate(id, TruthValue::Unknown)),
            _ => None,
        })
        .unwrap();

        assert_eq!(evaluation.satisfaction, Satisfaction::Unsatisfied);
        assert_eq!(
            evaluation.missing_conditions,
            vec!["spreadsheet_present".to_string()]
        );
        assert_eq!(
            evaluation.unknown_conditions,
            vec!["buyer_template_present".to_string()]
        );

        let delta = EvaluationDelta::from_evaluation("delta:1", &evaluation);
        assert_eq!(
            delta.unsatisfied_predicates,
            vec!["spreadsheet_present".to_string()]
        );
        assert_eq!(
            delta.unknown_predicates,
            vec!["buyer_template_present".to_string()]
        );
        assert_eq!(
            delta.missing_evidence,
            vec!["evidence:buyer_template_present".to_string()]
        );
    }

    #[test]
    fn evaluation_delta_uses_structured_conflicting_evidence() {
        let expr = PolicyExpr::PredicateRef("evidence_consistent".to_string());
        let request = EvaluationRequest {
            evaluation_id: "eval:conflict",
            policy_id: "evidence_policy",
            policy_version: "v1",
            projection_id: "submission_state",
            state_digest: "state:test",
            cursor: &cursor(),
            expr: &expr,
        };
        let evaluation = evaluate_policy(request, |id| {
            let mut result = predicate(id, TruthValue::False);
            result
                .reasons
                .push("no conflict in free-form prose".to_string());
            result
                .conflicting_evidence
                .push("evidence:invoice-001".to_string());
            Some(result)
        })
        .unwrap();

        let delta = EvaluationDelta::from_evaluation("delta:conflict", &evaluation);
        assert_eq!(
            delta.conflicting_evidence,
            vec!["evidence:invoice-001".to_string()]
        );
    }

    #[test]
    fn gate_maps_satisfaction_to_decision() {
        let expr = PolicyExpr::PredicateRef("candidate_valid".to_string());
        let request = EvaluationRequest {
            evaluation_id: "eval:gate",
            policy_id: "candidate_policy",
            policy_version: "v1",
            projection_id: "authority_state",
            state_digest: "state:test",
            cursor: &cursor(),
            expr: &expr,
        };
        let evaluation =
            evaluate_policy(request, |id| Some(predicate(id, TruthValue::Unknown))).unwrap();

        let gate = GateResult::from_evaluation(evaluation);
        assert_eq!(gate.decision, GateDecision::Hold);
    }

    #[test]
    fn serde_round_trip_keeps_core_shapes_plain() {
        let frame = EventFrame {
            stream_id: "stream:test".to_string(),
            event_ordinal: 1,
            byte_offset: Some(0),
            bytes: b"{}".to_vec(),
            content_digest: Some("sha256:abc".to_string()),
            proof_ref: None,
        };

        let encoded = serde_json::to_string(&frame).unwrap();
        let decoded: EventFrame = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, frame);
    }
}
