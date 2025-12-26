//! Output formatting utilities.

use serde_json::Value;

/// Formats an event as a simple table row.
pub fn format_table_row(event: &Value) -> String {
    let event_id = event
        .get("event_id")
        .and_then(|v| v.get("b64"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let event_type = event
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let occurred_at = event
        .get("occurred_at")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let principal_id = event
        .get("principal_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    format!(
        "{:<44} {:<15} {:<20} {}",
        truncate(event_id, 44),
        event_type,
        truncate(occurred_at, 20),
        principal_id
    )
}

/// Prints table header.
#[allow(clippy::print_literal)]
pub fn print_table_header() {
    println!(
        "{:<44} {:<15} {:<20} {}",
        "EVENT_ID", "TYPE", "OCCURRED_AT", "PRINCIPAL"
    );
    println!("{}", "-".repeat(100));
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
