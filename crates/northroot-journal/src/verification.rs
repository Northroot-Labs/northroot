//! Verification helpers for journal events.

use crate::errors::JournalError;
use crate::event::{validate_event_object_structure, EventJson};
use northroot_canonical::{compute_event_id, Canonicalizer};

/// Verifies an event JSON against its claimed event_id.
///
/// This parses the event, canonicalizes it, and checks that the computed
/// event_id matches the `event_id` field in the JSON.
pub fn verify_event_id(
    event: &EventJson,
    canonicalizer: &Canonicalizer,
) -> Result<bool, JournalError> {
    let claimed_id = validate_event_object_structure(event).map_err(JournalError::InvalidJson)?;

    // Compute actual event_id
    let computed_id = compute_event_id(event, canonicalizer)
        .map_err(|e| JournalError::InvalidJson(format!("event ID computation failed: {}", e)))?;

    Ok(claimed_id == computed_id)
}
