//! Event filtering API for selective event iteration.

use crate::traits::StoreReader;
use crate::EventJson;
use northroot_canonical::{Digest, Timestamp};
use serde_json::Value;

/// Trait for filtering events during iteration.
pub trait EventFilter {
    /// Returns true if the event matches the filter criteria.
    fn matches(&self, event: &EventJson) -> bool;
}

/// Filter by event type.
#[derive(Debug, Clone)]
pub struct EventTypeFilter {
    /// Event type to match (e.g., "authorization", "execution").
    pub event_type: String,
}

impl EventFilter for EventTypeFilter {
    fn matches(&self, event: &EventJson) -> bool {
        event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(|s| s == self.event_type)
            .unwrap_or(false)
    }
}

/// Filter by principal ID.
#[derive(Debug, Clone)]
pub struct PrincipalFilter {
    /// Principal ID to match.
    pub principal_id: String,
}

impl EventFilter for PrincipalFilter {
    fn matches(&self, event: &EventJson) -> bool {
        event
            .get("principal_id")
            .and_then(|v| v.as_str())
            .map(|s| s == self.principal_id)
            .unwrap_or(false)
    }
}

/// Filter by time range.
#[derive(Debug, Clone)]
pub struct TimeRangeFilter {
    /// Include events after this timestamp (inclusive).
    pub after: Option<Timestamp>,
    /// Include events before this timestamp (inclusive).
    pub before: Option<Timestamp>,
}

impl EventFilter for TimeRangeFilter {
    fn matches(&self, event: &EventJson) -> bool {
        let occurred_at = event
            .get("occurred_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let occurred_at = match occurred_at {
            Some(t) => t,
            None => return false,
        };

        // Check after bound
        if let Some(ref after) = self.after {
            if occurred_at.as_str() < after.as_ref() {
                return false;
            }
        }

        // Check before bound
        if let Some(ref before) = self.before {
            if occurred_at.as_str() > before.as_ref() {
                return false;
            }
        }

        true
    }
}

/// Filter by event ID (exact match).
#[derive(Debug, Clone)]
pub struct EventIdFilter {
    /// Event ID digest to match.
    pub event_id: Digest,
}

impl EventFilter for EventIdFilter {
    fn matches(&self, event: &EventJson) -> bool {
        let event_id = event.get("event_id");
        match event_id {
            Some(Value::Object(obj)) => {
                let alg = obj
                    .get("alg")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "sha-256" => Some(northroot_canonical::DigestAlg::Sha256),
                        _ => None,
                    });
                let b64 = obj.get("b64").and_then(|v| v.as_str());

                match (alg, b64) {
                    (Some(a), Some(b)) => a == self.event_id.alg && b == self.event_id.b64,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// Composite filter: all filters must match (AND).
pub struct AndFilter {
    /// Filters to combine with AND logic.
    pub filters: Vec<Box<dyn EventFilter>>,
}

impl EventFilter for AndFilter {
    fn matches(&self, event: &EventJson) -> bool {
        self.filters.iter().all(|f| f.matches(event))
    }
}

/// Composite filter: any filter must match (OR).
pub struct OrFilter {
    /// Filters to combine with OR logic.
    pub filters: Vec<Box<dyn EventFilter>>,
}

impl EventFilter for OrFilter {
    fn matches(&self, event: &EventJson) -> bool {
        self.filters.iter().any(|f| f.matches(event))
    }
}

/// Reader that filters events from an underlying reader.
#[derive(Debug)]
pub struct FilteredReader<R: StoreReader, F: EventFilter> {
    /// Underlying reader.
    reader: R,
    /// Filter to apply.
    filter: F,
}

impl<R: StoreReader, F: EventFilter> FilteredReader<R, F> {
    /// Creates a new filtered reader.
    pub fn new(reader: R, filter: F) -> Self {
        Self { reader, filter }
    }
}

impl<R: StoreReader, F: EventFilter> StoreReader for FilteredReader<R, F> {
    fn read_next(&mut self) -> Result<Option<EventJson>, crate::error::StoreError> {
        loop {
            match self.reader.read_next()? {
                None => return Ok(None),
                Some(event) if self.filter.matches(&event) => return Ok(Some(event)),
                Some(_) => continue, // skip non-matching
            }
        }
    }
}
