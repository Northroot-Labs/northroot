//! Verification helpers for journal events.

use crate::errors::JournalError;
use crate::event::EventJson;
use northroot_canonical::Canonicalizer;
use northroot_core::{compute_event_id, VerificationVerdict, Verifier};

/// Verifies an event JSON against its claimed event_id.
///
/// This parses the event, canonicalizes it, and checks that the computed
/// event_id matches the `event_id` field in the JSON.
pub fn verify_event_id(
    event: &EventJson,
    canonicalizer: &Canonicalizer,
) -> Result<bool, JournalError> {
    // Extract event_id from JSON
    let claimed_id = event
        .get("event_id")
        .and_then(|v| serde_json::from_value::<northroot_canonical::Digest>(v.clone()).ok())
        .ok_or_else(|| JournalError::InvalidJson("missing or invalid event_id".to_string()))?;

    // Compute actual event_id
    let computed_id = compute_event_id(event, canonicalizer)
        .map_err(|e| JournalError::InvalidJson(format!("event ID computation failed: {}", e)))?;

    Ok(claimed_id == computed_id)
}

/// Verifies an event using the core verifier.
///
/// This is a convenience wrapper that parses the event JSON into a typed
/// event and runs full verification. Returns the verification verdict.
pub fn verify_event(
    event: &EventJson,
    verifier: &Verifier,
) -> Result<VerificationVerdict, JournalError> {
    // Try to determine event type and verify accordingly
    let event_type = event
        .get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JournalError::InvalidJson("missing event_type".to_string()))?;

    match event_type {
        "authorization" => {
            let auth_event: northroot_core::AuthorizationEvent =
                serde_json::from_value(event.clone()).map_err(JournalError::JsonParse)?;
            let (_, verdict) = verifier
                .verify_authorization(&auth_event)
                .map_err(JournalError::InvalidJson)?;
            Ok(verdict)
        }
        "checkpoint" => {
            let checkpoint_event: northroot_core::CheckpointEvent =
                serde_json::from_value(event.clone()).map_err(JournalError::JsonParse)?;
            let (_, verdict) = verifier
                .verify_checkpoint(&checkpoint_event)
                .map_err(JournalError::InvalidJson)?;
            Ok(verdict)
        }
        "attestation" => {
            let attestation_event: northroot_core::AttestationEvent =
                serde_json::from_value(event.clone()).map_err(JournalError::JsonParse)?;
            let (_, verdict) = verifier
                .verify_attestation(&attestation_event)
                .map_err(JournalError::InvalidJson)?;
            Ok(verdict)
        }
        "execution" => {
            // Execution events require the authorization event for verification
            Err(JournalError::InvalidJson(
                "execution events require authorization event for verification; use verify_execution directly".to_string(),
            ))
        }
        _ => Err(JournalError::InvalidJson(format!(
            "unknown event type: {}",
            event_type
        ))),
    }
}
