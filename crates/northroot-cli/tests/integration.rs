//! Integration tests for CLI commands.

use northroot_store::{
    JournalBackendReader, JournalBackendWriter, ReadMode, StoreReader, StoreWriter, WriteOptions,
};
use serde_json::json;
use std::process::Command;
use tempfile::TempDir;

fn make_test_event(id: &str) -> serde_json::Value {
    // Helper to create valid digest strings
    let mut b64 = id.to_string();
    while b64.len() < 43 {
        b64.push('A');
    }
    if b64.len() > 44 {
        b64.truncate(44);
    }

    json!({
        "event_id": { "alg": "sha-256", "b64": b64 },
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": b64 }
        },
        "policy_id": "test-policy",
        "policy_digest": { "alg": "sha-256", "b64": b64 },
        "decision": "allow",
        "decision_code": "ALLOW",
        "authorization": {
            "kind": "grant",
            "bounds": {
                "allowed_tools": ["test.tool"],
                "meter_caps": []
            }
        }
    })
}

fn make_execution_event(id: &str, auth_id: &str) -> serde_json::Value {
    let mut b64_id = id.to_string();
    while b64_id.len() < 43 {
        b64_id.push('A');
    }
    if b64_id.len() > 44 {
        b64_id.truncate(44);
    }

    let mut b64_auth = auth_id.to_string();
    while b64_auth.len() < 43 {
        b64_auth.push('A');
    }
    if b64_auth.len() > 44 {
        b64_auth.truncate(44);
    }

    json!({
        "event_id": { "alg": "sha-256", "b64": b64_id },
        "event_type": "execution",
        "event_version": "1",
        "occurred_at": "2024-01-01T01:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": b64_id }
        },
        "auth_event_id": { "alg": "sha-256", "b64": b64_auth },
        "tool_name": "test.tool",
        "meter_used": [],
        "outcome": "success"
    })
}

fn create_test_journal() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer
            .append(&make_execution_event("exec1", "auth1"))
            .unwrap();
        writer.append(&make_test_event("auth2")).unwrap();
        writer.finish().unwrap();
    }

    (temp_dir, journal_path.to_string_lossy().to_string())
}

fn run_cli(args: &[&str]) -> (bool, String, String) {
    run_cli_with_features(args, &[])
}

fn run_cli_with_features(args: &[&str], features: &[&str]) -> (bool, String, String) {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "northroot"]);
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    cmd.arg("--").args(args);
    let output = cmd.output().expect("Failed to execute CLI");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let success = output.status.success();

    (success, stdout, stderr)
}

#[test]
fn test_list_command() {
    let (_temp_dir, journal_path) = create_test_journal();

    let (success, stdout, _) = run_cli(&["list", &journal_path]);
    assert!(success);
    assert!(stdout.contains("EVENT_ID"));
    assert!(stdout.contains("authorization"));
}

#[test]
fn test_list_with_type_filter() {
    let (_temp_dir, journal_path) = create_test_journal();

    let (success, stdout, _) = run_cli(&["list", &journal_path, "--type", "execution"]);
    assert!(success);
    assert!(!stdout.contains("authorization"));
    assert!(stdout.contains("execution"));
}

#[test]
fn test_list_json_output() {
    let (_temp_dir, journal_path) = create_test_journal();

    let (success, stdout, _) = run_cli(&["list", &journal_path, "--json"]);
    assert!(success);
    // JSON output should be parseable
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty());
    for line in lines {
        serde_json::from_str::<serde_json::Value>(line).expect("Invalid JSON");
    }
}

#[test]
fn test_get_command() {
    let (_temp_dir, journal_path) = create_test_journal();

    // Get first auth event (need to extract the actual digest from the journal)
    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let first_event = reader.read_next().unwrap().unwrap();
    let event_id = first_event["event_id"]["b64"].as_str().unwrap();

    let (success, stdout, _) = run_cli(&["get", &journal_path, event_id]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["event_type"], "authorization");
}

#[test]
fn test_get_command_not_found() {
    let (_temp_dir, journal_path) = create_test_journal();

    let mut fake_id = "nonexistent".to_string();
    while fake_id.len() < 43 {
        fake_id.push('A');
    }
    if fake_id.len() > 44 {
        fake_id.truncate(44);
    }

    let (success, _, stderr) = run_cli(&["get", &journal_path, &fake_id]);
    assert!(!success);
    assert!(stderr.contains("not found") || stderr.contains("Error"));
}

#[test]
fn test_verify_command() {
    let (_temp_dir, journal_path) = create_test_journal();

    let (success, stdout, _) = run_cli(&["verify", &journal_path]);
    assert!(success);
    assert!(stdout.contains("VERDICT") || stdout.contains("verdict"));
}

#[test]
fn test_inspect_command() {
    let (_temp_dir, journal_path) = create_test_journal();

    // Get first auth event ID
    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let first_event = reader.read_next().unwrap().unwrap();
    let auth_id = first_event["event_id"]["b64"].as_str().unwrap();

    let (success, stdout, _) = run_cli(&["inspect", &journal_path, "--auth", auth_id]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["authorization"]["event_type"], "authorization");
    assert!(parsed["executions"].is_array());
}

#[test]
#[cfg(feature = "dev-tools")]
fn test_gen_command_creates_valid_journal() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("gen_test.nrj");
    let journal_str = journal_path.to_string_lossy().to_string();

    // Generate journal with gen command
    let (success, stdout, _) = run_cli_with_features(
        &[
            "gen",
            "--output",
            &journal_str,
            "--count-auth",
            "3",
            "--count-exec-ok",
            "3",
        ],
        &["dev-tools"],
    );
    assert!(success, "gen command should succeed");
    assert!(stdout.contains("Generated"));

    // Verify we can list the generated events
    let (success, stdout, _) = run_cli(&["list", &journal_str, "--json"]);
    assert!(success, "list should work on generated journal");

    // Count events in JSON output
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() >= 6,
        "Should have at least 6 events (3 auth + 3 exec)"
    );

    // Verify all lines are valid JSON
    for line in lines {
        let parsed: serde_json::Value = serde_json::from_str(line).expect("Invalid JSON");
        assert!(
            parsed["event_type"] == "authorization" || parsed["event_type"] == "execution",
            "Event should be authorization or execution"
        );
    }
}

#[test]
#[cfg(feature = "dev-tools")]
fn test_verify_with_bad_exec() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("gen_bad_exec.nrj");
    let journal_str = journal_path.to_string_lossy().to_string();

    // Generate journal with one bad execution (orphan reference)
    let (success, _, _) = run_cli_with_features(
        &[
            "gen",
            "--output",
            &journal_str,
            "--count-auth",
            "1",
            "--count-exec-ok",
            "0",
            "--count-exec-bad",
            "1",
            "--force",
        ],
        &["dev-tools"],
    );
    assert!(success, "gen should succeed");

    // Verify should fail with --strict when there's a bad exec
    let (success, _, stderr) = run_cli(&["verify", &journal_str, "--strict"]);
    assert!(!success, "verify --strict should fail with bad exec");
    assert!(
        stderr.contains("Error") || stderr.contains("denied") || stderr.contains("violation"),
        "Should report verification error"
    );
}

#[test]
#[cfg(feature = "dev-tools")]
fn test_verify_with_malformed() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("gen_malformed.nrj");
    let journal_str = journal_path.to_string_lossy().to_string();

    // Generate journal with malformed event
    let (success, _, _) = run_cli_with_features(
        &[
            "gen",
            "--output",
            &journal_str,
            "--count-auth",
            "1",
            "--count-exec-ok",
            "0",
            "--with-bad",
            "--force",
        ],
        &["dev-tools"],
    );
    assert!(success, "gen should succeed even with --with-bad");

    // Verify should surface error for malformed event
    let (_success, _, stderr) = run_cli(&["verify", &journal_str]);
    // Note: verify might still exit 0 without --strict, but should report the error
    // If --strict is used, it should definitely fail
    let (strict_success, _, _) = run_cli(&["verify", &journal_str, "--strict"]);
    assert!(
        !strict_success || stderr.contains("Error") || stderr.contains("invalid"),
        "Should report malformed event error"
    );
}
