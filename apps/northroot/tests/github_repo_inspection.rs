use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn northroot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_northroot"))
}

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/github-repo-inspection")
}

fn run_json(args: &[&str]) -> Value {
    let output = northroot()
        .args(args)
        .output()
        .expect("failed to run northroot");
    assert!(
        output.status.success(),
        "northroot {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("northroot did not emit JSON")
}

fn run(args: &[&str]) {
    let output = northroot()
        .args(args)
        .output()
        .expect("failed to run northroot");
    assert!(
        output.status.success(),
        "northroot {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn github_repo_inspection_bundle_is_canonically_verifiable() {
    let example = example_dir();
    let manifest_path = example.join("manifest.json");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should be valid JSON");

    for file in [
        "actor.json",
        "obligation.json",
        "policy.json",
        "receipt.json",
    ] {
        let output = northroot()
            .arg("validate")
            .arg(example.join(file))
            .output()
            .expect("failed to run northroot validate");
        assert!(
            output.status.success(),
            "validate {file} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for file in manifest
        .get("files")
        .and_then(Value::as_array)
        .expect("manifest files should be an array")
    {
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .expect("manifest file path should be a string");
        let expected_hash = file
            .get("hash")
            .and_then(Value::as_str)
            .expect("manifest file hash should be a string");
        let hash = run_json(&[
            "hash",
            example
                .join(path)
                .to_str()
                .expect("example path should be UTF-8"),
        ]);
        assert_eq!(hash["hash"], expected_hash, "{path} hash drifted");
    }

    let verified_receipt = run_json(&[
        "verify",
        example
            .join("receipt.json")
            .to_str()
            .expect("receipt path should be UTF-8"),
        "--base-dir",
        example.to_str().expect("example path should be UTF-8"),
        "--json",
    ]);
    assert_eq!(verified_receipt["valid"], true);
    assert_eq!(
        verified_receipt["evidence"]
            .as_array()
            .expect("receipt verification should list checked evidence")
            .len(),
        4
    );
}

#[test]
fn github_repo_inspection_events_replay_into_verifiable_journal() {
    let example = example_dir();
    let events = std::fs::read_to_string(example.join("events.jsonl"))
        .expect("events.jsonl should be readable");
    let temp = TempDir::new().expect("temp dir should be created");
    let journal = temp.path().join("github-repo-inspection.nrj");
    let journal_arg = journal.to_str().expect("journal path should be UTF-8");

    let mut expected_count = 0;
    for (idx, line) in events
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let event: Value = serde_json::from_str(line).expect("event line should be valid JSON");
        assert!(
            event.get("event_id").is_none(),
            "source JSONL should stay event-id free before journal append"
        );

        let event_path = temp.path().join(format!("event-{idx}.json"));
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap())
            .expect("event temp file should be written");
        run(&[
            "append",
            journal_arg,
            event_path.to_str().expect("event path should be UTF-8"),
        ]);
        expected_count += 1;
    }

    let list_output = northroot()
        .args(["list", journal_arg, "--json"])
        .output()
        .expect("failed to run northroot list");
    assert!(
        list_output.status.success(),
        "list journal failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&list_output.stdout),
        String::from_utf8_lossy(&list_output.stderr)
    );
    let listed_events: Vec<Value> = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("list output line should be JSON"))
        .collect();
    assert_eq!(listed_events.len(), expected_count);
    assert!(listed_events
        .iter()
        .all(|event| event.get("event_id").is_some()));

    let verified_events = run_json(&["verify", journal_arg, "--json", "--strict"]);
    let verified_events = verified_events
        .as_array()
        .expect("journal verification should return an array");
    assert_eq!(verified_events.len(), expected_count);
    assert!(verified_events
        .iter()
        .all(|event| event["valid"].as_bool() == Some(true)));
}
