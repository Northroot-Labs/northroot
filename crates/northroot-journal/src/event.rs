use serde_json::Value;

/// Event JSON payload type.
///
/// This is a type alias for `serde_json::Value` representing a canonical
/// Northroot event JSON object. The journal stores these as-is; canonicalization
/// and verification happen via `northroot-canonical`.
pub type EventJson = Value;

/// Helper to validate that a JSON value is a valid event object.
///
/// This performs only kernel structural checks: the value must be an object and
/// `event_id` must be present as a digest-shaped value. Domain fields such as
/// `event_type`, policy meaning, workflow state, and authorization semantics are
/// intentionally not validated by the journal core.
pub fn is_valid_event_structure(value: &EventJson) -> bool {
    validate_event_object_structure(value).is_ok()
}

/// Validates the kernel event-object boundary and returns the claimed event ID.
///
/// This does not validate event type, authorization, workflow state, or any
/// domain payload schema.
pub fn validate_event_object_structure(
    value: &EventJson,
) -> Result<northroot_canonical::Digest, String> {
    let Some(obj) = value.as_object() else {
        return Err("event payload must be a JSON object".to_string());
    };

    let event_id = obj
        .get("event_id")
        .ok_or_else(|| "event_id is required".to_string())?
        .clone();
    let digest: northroot_canonical::Digest = serde_json::from_value(event_id)
        .map_err(|e| format!("event_id must be digest-shaped: {}", e))?;
    northroot_canonical::Digest::new(digest.alg, digest.b64)
        .map_err(|e| format!("event_id must be digest-shaped: {}", e))
}
