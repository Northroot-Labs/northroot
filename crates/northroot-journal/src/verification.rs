//! Verification helpers for journal events.

use crate::errors::JournalError;
use crate::event::{EventJson, EventObject};
use northroot_canonical::{compute_event_id, Canonicalizer};

/// Verifies an event JSON against its claimed event_id.
///
/// This parses the event, canonicalizes it, and checks that the computed
/// event_id matches the `event_id` field in the JSON.
pub fn verify_event_id(
    event: &EventJson,
    canonicalizer: &Canonicalizer,
) -> Result<bool, JournalError> {
    let event_object = EventObject::validate(event.clone()).map_err(JournalError::InvalidJson)?;

    // Compute actual event_id
    let computed_id = compute_event_id(event_object.as_json(), canonicalizer)
        .map_err(|e| JournalError::InvalidJson(format!("event ID computation failed: {}", e)))?;

    Ok(event_object.claimed_event_id() == &computed_id)
}
