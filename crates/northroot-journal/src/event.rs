use serde_json::Value;

/// Event JSON payload type.
///
/// This is a type alias for `serde_json::Value` representing a canonical
/// Northroot event JSON object. The journal stores these as-is; canonicalization
/// and verification happen via `northroot-core`.
pub type EventJson = Value;

/// Helper to validate that a JSON value is a valid event object.
///
/// This performs basic structural checks (is an object, has required fields).
/// Full verification (event_id computation, canonicalization) should be done
/// via `northroot-core::Verifier`.
pub fn is_valid_event_structure(value: &EventJson) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };

    // Check for required top-level fields
    obj.contains_key("event_id")
        && obj.contains_key("event_type")
        && obj.contains_key("event_version")
        && obj.contains_key("occurred_at")
        && obj.contains_key("principal_id")
        && obj.contains_key("canonical_profile_id")
}
