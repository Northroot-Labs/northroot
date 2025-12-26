//! Typed event parsing from JSON.

use crate::EventJson;
use northroot_core::{AttestationEvent, AuthorizationEvent, CheckpointEvent, ExecutionEvent};
use thiserror::Error;

/// Error that can occur when parsing an event.
#[derive(Error, Debug)]
pub enum ParseError {
    /// JSON deserialization error.
    #[error("deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),
}

/// Typed representation of an event.
#[derive(Debug, Clone)]
pub enum TypedEvent {
    /// Authorization event.
    Authorization(AuthorizationEvent),
    /// Execution event.
    Execution(ExecutionEvent),
    /// Checkpoint event.
    Checkpoint(CheckpointEvent),
    /// Attestation event.
    Attestation(AttestationEvent),
    /// Unknown event type or unparseable event.
    Unknown(EventJson),
}

/// Parses a JSON event into a typed event.
///
/// Inspects the `event_type` field to determine the event variant,
/// then deserializes to the appropriate typed struct. Falls back to
/// `TypedEvent::Unknown` if the event type is unrecognized or if
/// deserialization fails.
pub fn parse_event(json: &EventJson) -> Result<TypedEvent, ParseError> {
    // Extract event_type field
    let event_type = json
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Try to deserialize based on event_type
    match event_type {
        "authorization" => {
            let event: AuthorizationEvent = serde_json::from_value(json.clone())?;
            Ok(TypedEvent::Authorization(event))
        }
        "execution" => {
            let event: ExecutionEvent = serde_json::from_value(json.clone())?;
            Ok(TypedEvent::Execution(event))
        }
        "checkpoint" => {
            let event: CheckpointEvent = serde_json::from_value(json.clone())?;
            Ok(TypedEvent::Checkpoint(event))
        }
        "attestation" => {
            let event: AttestationEvent = serde_json::from_value(json.clone())?;
            Ok(TypedEvent::Attestation(event))
        }
        _ => {
            // Unknown event type or missing event_type field
            Ok(TypedEvent::Unknown(json.clone()))
        }
    }
}
