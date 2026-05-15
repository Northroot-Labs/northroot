use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

    let output = northroot()
        .arg("verify")
        .arg(example.join("receipt.json"))
        .arg("--base-dir")
        .arg(&example)
        .output()
        .expect("failed to run northroot verify");
    assert!(
        output.status.success(),
        "verify receipt failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
