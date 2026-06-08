use northroot_canonical::Digest;
use serde_json::Value;

/// Event JSON payload type.
///
/// This is a type alias for `serde_json::Value` representing a canonical
/// Northroot event JSON object. The journal stores these as-is; canonicalization
/// and verification happen via `northroot-canonical`.
pub type EventJson = Value;

/// Structurally validated kernel event object.
///
/// This wrapper keeps the event payload untyped at the domain layer while
/// proving the kernel boundary checks that every journal verifier needs: the
/// payload is a JSON object and `event_id` is present as a valid digest shape.
/// It does not validate event type, authorization, workflow state, policy
/// meaning, or any domain schema.
#[derive(Debug, Clone, PartialEq)]
pub struct EventObject {
    value: EventJson,
    claimed_event_id: Digest,
}

impl EventObject {
    /// Validates an untyped event JSON value and returns a structural wrapper.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload is not a JSON object or `event_id` is
    /// missing or not digest-shaped.
    pub fn validate(value: EventJson) -> Result<Self, String> {
        let claimed_event_id = validate_event_object_structure(&value)?;
        Ok(Self {
            value,
            claimed_event_id,
        })
    }

    /// Returns the untyped JSON event value.
    pub fn as_json(&self) -> &EventJson {
        &self.value
    }

    /// Consumes the wrapper and returns the untyped JSON event value.
    pub fn into_json(self) -> EventJson {
        self.value
    }

    /// Returns the digest claimed by the event's `event_id` field.
    pub fn claimed_event_id(&self) -> &Digest {
        &self.claimed_event_id
    }
}

impl TryFrom<EventJson> for EventObject {
    type Error = String;

    fn try_from(value: EventJson) -> Result<Self, Self::Error> {
        Self::validate(value)
    }
}

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
pub fn validate_event_object_structure(value: &EventJson) -> Result<Digest, String> {
    let Some(obj) = value.as_object() else {
        return Err("event payload must be a JSON object".to_string());
    };

    let event_id = obj
        .get("event_id")
        .ok_or_else(|| "event_id is required".to_string())?
        .clone();
    let digest: Digest = serde_json::from_value(event_id)
        .map_err(|e| format!("event_id must be digest-shaped: {}", e))?;
    Digest::new(digest.alg, digest.b64)
        .map_err(|e| format!("event_id must be digest-shaped: {}", e))
}
