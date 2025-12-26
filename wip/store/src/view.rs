//! View API for typed event access and linkage navigation.

use crate::error::StoreError;
use crate::traits::StoreReader;
use crate::typed::parse_event;
use crate::typed::TypedEvent;
use northroot_canonical::Digest;
use northroot_core::{AuthorizationEvent, ExecutionEvent};

/// Resolves an authorization event by ID from a reader.
///
/// Performs a sequential scan through the reader until an authorization event
/// with the matching `event_id` is found. Returns `None` if not found.
///
/// This function requires a full scan (no indexing per contract Section 10).
pub fn resolve_auth<R: StoreReader>(
    reader: &mut R,
    auth_event_id: &Digest,
) -> Result<Option<AuthorizationEvent>, StoreError> {
    loop {
        match reader.read_next()? {
            None => return Ok(None),
            Some(event_json) => match parse_event(&event_json)? {
                TypedEvent::Authorization(auth) if auth.event_id == *auth_event_id => {
                    return Ok(Some(auth));
                }
                _ => continue,
            },
        }
    }
}

/// Collects all execution events linked to an authorization.
///
/// Performs a sequential scan through the reader and collects all execution
/// events whose `auth_event_id` matches the provided authorization event ID.
///
/// This function requires a full scan (no indexing per contract Section 10).
pub fn executions_for_auth<R: StoreReader>(
    reader: &mut R,
    auth_event_id: &Digest,
) -> Result<Vec<ExecutionEvent>, StoreError> {
    let mut executions = Vec::new();

    loop {
        match reader.read_next()? {
            None => break,
            Some(event_json) => match parse_event(&event_json)? {
                TypedEvent::Execution(exec) if exec.auth_event_id == *auth_event_id => {
                    executions.push(exec);
                }
                _ => continue,
            },
        }
    }

    Ok(executions)
}
