//! Portable evidence bundle verification.
//!
//! Bundle manifests keep the compatibility term `receipts`; this verifier checks
//! receipt-shaped evidence artifacts for path, hash, event ID, and journal
//! membership. It does not define domain receipt semantics.

use base64::Engine;
use northroot_canonical::{parse_json_strict, Canonicalizer, Digest, DigestAlg, ProfileId};
use northroot_journal::{verify_event_id, JournalReader, ReadMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const BUNDLE_MANIFEST: &str = "northroot-bundle.json";
const BUNDLE_FORMAT: &str = "northroot-evidence-bundle";
const BUNDLE_VERSION: u64 = 1;
const CANONICAL_PROFILE: &str = "northroot-canonical-v1";
const REPORT_FORMAT: &str = "northroot-bundle-audit-v1";

#[derive(Debug, Deserialize)]
struct BundleManifest {
    bundle_format: String,
    bundle_version: u64,
    canonical_profile_id: String,
    journal: ManifestJournal,
    #[serde(default)]
    receipts: Vec<ManifestReceipt>,
    #[serde(default)]
    artifacts: Vec<ManifestArtifact>,
}

#[derive(Debug, Deserialize)]
struct ManifestJournal {
    path: String,
    sha256: Digest,
}

#[derive(Debug, Deserialize)]
struct ManifestReceipt {
    path: String,
    event_id: Digest,
    sha256: Digest,
}

#[derive(Debug, Deserialize)]
struct ManifestArtifact {
    path: String,
    content_id: Digest,
}

#[derive(Debug, Serialize)]
pub struct AuditReport {
    report_format: String,
    valid: bool,
    bundle_path: String,
    checks: Vec<CheckReport>,
    counts: CountReport,
    journal: Option<JournalReport>,
    receipts: Vec<ReceiptReport>,
    artifacts: Vec<ArtifactReport>,
    errors: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct CountReport {
    journal_events: usize,
    receipts: usize,
    artifacts: usize,
}

#[derive(Debug, Serialize)]
struct CheckReport {
    name: String,
    valid: bool,
}

#[derive(Debug, Serialize)]
struct JournalReport {
    path: String,
    sha256_expected: Digest,
    sha256_actual: Option<Digest>,
    event_count: usize,
    first_event_id: Option<Digest>,
    last_event_id: Option<Digest>,
    valid: bool,
}

#[derive(Debug, Serialize)]
struct ReceiptReport {
    path: String,
    event_id_expected: Digest,
    event_id_actual: Option<Digest>,
    sha256_expected: Digest,
    sha256_actual: Option<Digest>,
    in_journal: bool,
    valid: bool,
}

#[derive(Debug, Serialize)]
struct ArtifactReport {
    path: String,
    content_id_expected: Digest,
    content_id_actual: Option<Digest>,
    valid: bool,
}

impl AuditReport {
    fn new(bundle_path: &Path) -> Self {
        Self {
            report_format: REPORT_FORMAT.to_string(),
            valid: true,
            bundle_path: bundle_path.display().to_string(),
            checks: Vec::new(),
            counts: CountReport::default(),
            journal: None,
            receipts: Vec::new(),
            artifacts: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn add_check(&mut self, name: impl Into<String>, valid: bool) {
        if !valid {
            self.valid = false;
        }
        self.checks.push(CheckReport {
            name: name.into(),
            valid,
        });
    }

    fn add_error(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }
}

pub fn run(dir: String, _json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let report = verify_bundle_dir(Path::new(&dir));
    println!("{}", serde_json::to_string_pretty(&report)?);

    if !report.valid {
        std::process::exit(1);
    }

    Ok(())
}

pub fn verify_bundle_dir(dir: &Path) -> AuditReport {
    let root = match dir.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            let mut report = AuditReport::new(dir);
            report.add_check("bundle_directory", false);
            report.add_error(format!("bundle directory cannot be resolved: {}", e));
            return report;
        }
    };

    let mut report = AuditReport::new(&root);
    if !root.is_dir() {
        report.add_check("bundle_directory", false);
        report.add_error("bundle path is not a directory");
        return report;
    }
    report.add_check("bundle_directory", true);

    let manifest_path = root.join(BUNDLE_MANIFEST);
    let manifest: BundleManifest = match read_json_file(&manifest_path) {
        Ok(manifest) => {
            report.add_check("manifest_parse", true);
            manifest
        }
        Err(e) => {
            report.add_check("manifest_parse", false);
            report.add_error(format!("manifest invalid: {}", e));
            return report;
        }
    };

    validate_manifest_header(&manifest, &mut report);
    validate_duplicate_paths(&manifest, &mut report);

    let profile = match ProfileId::parse(&manifest.canonical_profile_id) {
        Ok(profile) => profile,
        Err(e) => {
            report.add_check("canonical_profile", false);
            report.add_error(format!("canonical profile invalid: {}", e));
            return report;
        }
    };
    let canonicalizer = Canonicalizer::new(profile);

    let journal_event_ids = verify_journal(&root, &manifest, &canonicalizer, &mut report);
    verify_receipts(
        &root,
        &manifest,
        &canonicalizer,
        &journal_event_ids,
        &mut report,
    );
    verify_artifacts(&root, &manifest, &mut report);

    report.counts.receipts = report.receipts.len();
    report.counts.artifacts = report.artifacts.len();
    report
}

fn validate_manifest_header(manifest: &BundleManifest, report: &mut AuditReport) {
    let format_ok = manifest.bundle_format == BUNDLE_FORMAT;
    report.add_check("manifest_bundle_format", format_ok);
    if !format_ok {
        report.add_error(format!(
            "unsupported bundle_format: {}",
            manifest.bundle_format
        ));
    }

    let version_ok = manifest.bundle_version == BUNDLE_VERSION;
    report.add_check("manifest_bundle_version", version_ok);
    if !version_ok {
        report.add_error(format!(
            "unsupported bundle_version: {}",
            manifest.bundle_version
        ));
    }

    let profile_ok = manifest.canonical_profile_id == CANONICAL_PROFILE;
    report.add_check("manifest_canonical_profile", profile_ok);
    if !profile_ok {
        report.add_error(format!(
            "unsupported canonical_profile_id: {}",
            manifest.canonical_profile_id
        ));
    }
}

fn validate_duplicate_paths(manifest: &BundleManifest, report: &mut AuditReport) {
    let mut seen = BTreeSet::new();
    let mut ok = true;

    for path in std::iter::once(&manifest.journal.path)
        .chain(manifest.receipts.iter().map(|receipt| &receipt.path))
        .chain(manifest.artifacts.iter().map(|artifact| &artifact.path))
    {
        if !seen.insert(path.clone()) {
            report.add_error(format!("duplicate bundle path listed: {}", path));
            ok = false;
        }
    }

    report.add_check("manifest_unique_paths", ok);
}

fn verify_journal(
    root: &Path,
    manifest: &BundleManifest,
    canonicalizer: &Canonicalizer,
    report: &mut AuditReport,
) -> BTreeSet<String> {
    let mut journal_report = JournalReport {
        path: manifest.journal.path.clone(),
        sha256_expected: manifest.journal.sha256.clone(),
        sha256_actual: None,
        event_count: 0,
        first_event_id: None,
        last_event_id: None,
        valid: true,
    };

    let journal_path = match resolve_bundle_file(root, &manifest.journal.path) {
        Ok(path) => {
            report.add_check("journal_path", true);
            path
        }
        Err(e) => {
            report.add_check("journal_path", false);
            report.add_error(format!("journal path invalid: {}", e));
            journal_report.valid = false;
            report.journal = Some(journal_report);
            return BTreeSet::new();
        }
    };

    match sha256_file(&journal_path) {
        Ok(actual) => {
            let ok = digest_matches(&manifest.journal.sha256, &actual);
            journal_report.sha256_actual = Some(actual);
            journal_report.valid &= ok;
            report.add_check("journal_sha256", ok);
            if !ok {
                report.add_error("journal sha256 does not match manifest");
            }
        }
        Err(e) => {
            journal_report.valid = false;
            report.add_check("journal_sha256", false);
            report.add_error(format!("journal sha256 failed: {}", e));
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut reader = match JournalReader::open(&journal_path, ReadMode::Strict) {
        Ok(reader) => {
            report.add_check("journal_open", true);
            reader
        }
        Err(e) => {
            journal_report.valid = false;
            report.add_check("journal_open", false);
            report.add_error(format!("journal open failed: {}", e));
            report.journal = Some(journal_report);
            return event_ids;
        }
    };

    let mut previous_event_id: Option<Digest> = None;
    let mut journal_events_ok = true;
    let mut continuity_ok = true;

    loop {
        let event = match reader.read_event() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(e) => {
                journal_events_ok = false;
                report.add_error(format!("journal read failed: {}", e));
                break;
            }
        };

        journal_report.event_count += 1;

        match verify_event_id(&event, canonicalizer) {
            Ok(true) => {}
            Ok(false) => {
                journal_events_ok = false;
                report.add_error(format!(
                    "journal event {} has event_id mismatch",
                    journal_report.event_count
                ));
            }
            Err(e) => {
                journal_events_ok = false;
                report.add_error(format!(
                    "journal event {} failed event_id verification: {}",
                    journal_report.event_count, e
                ));
            }
        }

        let event_id = match extract_digest(&event, "event_id") {
            Ok(id) => id,
            Err(e) => {
                journal_events_ok = false;
                report.add_error(format!(
                    "journal event {} missing valid event_id: {}",
                    journal_report.event_count, e
                ));
                continue;
            }
        };

        if journal_report.first_event_id.is_none() {
            journal_report.first_event_id = Some(event_id.clone());
        }
        journal_report.last_event_id = Some(event_id.clone());
        event_ids.insert(event_id.b64.clone());

        let prev_event_id = event
            .get("prev_event_id")
            .map(|_| extract_digest(&event, "prev_event_id"));

        match (&previous_event_id, prev_event_id) {
            (None, Some(Ok(_))) => {
                continuity_ok = false;
                report.add_error("first journal event must not include prev_event_id");
            }
            (None, Some(Err(e))) => {
                continuity_ok = false;
                report.add_error(format!("first journal prev_event_id invalid: {}", e));
            }
            (None, None) => {}
            (Some(expected), Some(Ok(actual))) if actual == *expected => {}
            (Some(expected), Some(Ok(actual))) => {
                continuity_ok = false;
                report.add_error(format!(
                    "journal event {} prev_event_id {} does not match previous event_id {}",
                    journal_report.event_count, actual.b64, expected.b64
                ));
            }
            (Some(_), Some(Err(e))) => {
                continuity_ok = false;
                report.add_error(format!(
                    "journal event {} prev_event_id invalid: {}",
                    journal_report.event_count, e
                ));
            }
            (Some(_), None) => {
                continuity_ok = false;
                report.add_error(format!(
                    "journal event {} missing prev_event_id",
                    journal_report.event_count
                ));
            }
        }

        previous_event_id = Some(event_id);
    }

    journal_report.valid &= journal_events_ok && continuity_ok;
    report.counts.journal_events = journal_report.event_count;
    report.add_check("journal_event_ids", journal_events_ok);
    report.add_check("journal_continuity", continuity_ok);
    report.journal = Some(journal_report);
    event_ids
}

fn verify_receipts(
    root: &Path,
    manifest: &BundleManifest,
    canonicalizer: &Canonicalizer,
    journal_event_ids: &BTreeSet<String>,
    report: &mut AuditReport,
) {
    let mut paths_ok = true;
    let mut hashes_ok = true;
    let mut event_ids_ok = true;
    let mut membership_ok = true;

    for receipt in &manifest.receipts {
        let mut receipt_report = ReceiptReport {
            path: receipt.path.clone(),
            event_id_expected: receipt.event_id.clone(),
            event_id_actual: None,
            sha256_expected: receipt.sha256.clone(),
            sha256_actual: None,
            in_journal: false,
            valid: true,
        };

        let receipt_path = match resolve_bundle_file(root, &receipt.path) {
            Ok(path) => path,
            Err(e) => {
                paths_ok = false;
                receipt_report.valid = false;
                report.add_error(format!("receipt path invalid: {}: {}", receipt.path, e));
                report.receipts.push(receipt_report);
                continue;
            }
        };

        match sha256_file(&receipt_path) {
            Ok(actual) => {
                let ok = digest_matches(&receipt.sha256, &actual);
                receipt_report.sha256_actual = Some(actual);
                receipt_report.valid &= ok;
                hashes_ok &= ok;
                if !ok {
                    report.add_error(format!(
                        "receipt sha256 does not match manifest: {}",
                        receipt.path
                    ));
                }
            }
            Err(e) => {
                hashes_ok = false;
                receipt_report.valid = false;
                report.add_error(format!("receipt sha256 failed: {}: {}", receipt.path, e));
            }
        }

        match read_json_file::<Value>(&receipt_path) {
            Ok(event) => {
                match verify_event_id(&event, canonicalizer) {
                    Ok(true) => {}
                    Ok(false) => {
                        event_ids_ok = false;
                        receipt_report.valid = false;
                        report.add_error(format!("receipt event_id mismatch: {}", receipt.path));
                    }
                    Err(e) => {
                        event_ids_ok = false;
                        receipt_report.valid = false;
                        report.add_error(format!(
                            "receipt event_id verification failed: {}: {}",
                            receipt.path, e
                        ));
                    }
                }

                match extract_digest(&event, "event_id") {
                    Ok(actual) => {
                        let ok = actual == receipt.event_id;
                        receipt_report.event_id_actual = Some(actual.clone());
                        receipt_report.valid &= ok;
                        event_ids_ok &= ok;
                        if !ok {
                            report.add_error(format!(
                                "receipt event_id does not match manifest: {}",
                                receipt.path
                            ));
                        }

                        let in_journal = journal_event_ids.contains(&actual.b64);
                        receipt_report.in_journal = in_journal;
                        receipt_report.valid &= in_journal;
                        membership_ok &= in_journal;
                        if !in_journal {
                            report.add_error(format!(
                                "receipt event_id not present in journal: {}",
                                receipt.path
                            ));
                        }
                    }
                    Err(e) => {
                        event_ids_ok = false;
                        receipt_report.valid = false;
                        report.add_error(format!(
                            "receipt missing valid event_id: {}: {}",
                            receipt.path, e
                        ));
                    }
                }
            }
            Err(e) => {
                event_ids_ok = false;
                receipt_report.valid = false;
                report.add_error(format!("receipt JSON invalid: {}: {}", receipt.path, e));
            }
        }

        report.receipts.push(receipt_report);
    }

    report.add_check("receipt_paths", paths_ok);
    report.add_check("receipt_sha256", hashes_ok);
    report.add_check("receipt_event_ids", event_ids_ok);
    report.add_check("receipt_journal_membership", membership_ok);
}

fn verify_artifacts(root: &Path, manifest: &BundleManifest, report: &mut AuditReport) {
    let mut paths_ok = true;
    let mut hashes_ok = true;

    for artifact in &manifest.artifacts {
        let mut artifact_report = ArtifactReport {
            path: artifact.path.clone(),
            content_id_expected: artifact.content_id.clone(),
            content_id_actual: None,
            valid: true,
        };

        let artifact_path = match resolve_bundle_file(root, &artifact.path) {
            Ok(path) => path,
            Err(e) => {
                paths_ok = false;
                artifact_report.valid = false;
                report.add_error(format!("artifact path invalid: {}: {}", artifact.path, e));
                report.artifacts.push(artifact_report);
                continue;
            }
        };

        match sha256_file(&artifact_path) {
            Ok(actual) => {
                let ok = digest_matches(&artifact.content_id, &actual);
                artifact_report.content_id_actual = Some(actual);
                artifact_report.valid &= ok;
                hashes_ok &= ok;
                if !ok {
                    report.add_error(format!(
                        "artifact content_id does not match manifest: {}",
                        artifact.path
                    ));
                }
            }
            Err(e) => {
                hashes_ok = false;
                artifact_report.valid = false;
                report.add_error(format!("artifact hash failed: {}: {}", artifact.path, e));
            }
        }

        report.artifacts.push(artifact_report);
    }

    report.add_check("artifact_paths", paths_ok);
    report.add_check("artifact_content_ids", hashes_ok);
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let input = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value = parse_json_strict(&input).map_err(|e| e.to_string())?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn resolve_bundle_file(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel.is_empty() {
        return Err("path is empty".to_string());
    }
    if rel_path.is_absolute() {
        return Err("absolute paths are not allowed".to_string());
    }

    for component in rel_path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err("parent directory components are not allowed".to_string())
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute path components are not allowed".to_string());
            }
        }
    }

    let candidate = root.join(rel_path);
    reject_symlink_components(root, rel_path)?;

    let resolved = candidate
        .canonicalize()
        .map_err(|e| format!("path cannot be resolved: {}", e))?;
    if !resolved.starts_with(root) {
        return Err("path escapes bundle root".to_string());
    }
    if !resolved.is_file() {
        return Err("path is not a file".to_string());
    }

    Ok(resolved)
}

fn reject_symlink_components(root: &Path, rel: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => current.push(part),
            Component::CurDir => continue,
            _ => continue,
        }

        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|e| format!("path cannot be inspected: {}", e))?;
        if metadata.file_type().is_symlink() {
            return Err("symlink paths are not allowed".to_string());
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<Digest, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
    Digest::new(DigestAlg::Sha256, b64).map_err(|e| e.to_string())
}

fn digest_matches(expected: &Digest, actual: &Digest) -> bool {
    expected.alg == DigestAlg::Sha256 && expected == actual
}

fn extract_digest(event: &Value, field: &str) -> Result<Digest, String> {
    let value = event
        .get(field)
        .ok_or_else(|| format!("missing {}", field))?
        .clone();
    let digest: Digest = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Digest::new(digest.alg, digest.b64).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
    use northroot_journal::{JournalWriter, WriteOptions};
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn verifies_valid_agent_turn_bundle() {
        let temp = build_agent_turn_bundle(BundleVariant::Valid);
        let report = verify_bundle_dir(temp.path());

        assert!(report.valid, "{:#?}", report.errors);
        assert_eq!(report.counts.journal_events, 3);
        assert_eq!(report.counts.receipts, 3);
        assert_eq!(report.counts.artifacts, 3);
    }

    #[test]
    fn rejects_modified_artifact_hash() {
        let temp = build_agent_turn_bundle(BundleVariant::Valid);
        fs::write(temp.path().join("artifacts/input.txt"), "changed").unwrap();

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("artifact content_id")));
    }

    #[test]
    fn rejects_receipt_event_id_mismatch() {
        let temp = build_agent_turn_bundle(BundleVariant::Valid);
        let receipt_path = temp.path().join("receipts/002-output.json");
        let mut receipt: Value = read_json_file(&receipt_path).unwrap();
        receipt["note"] = json!("tampered");
        fs::write(receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("receipt event_id mismatch")));
    }

    #[test]
    fn rejects_receipt_json_with_duplicate_keys() {
        let temp = build_agent_turn_bundle(BundleVariant::Valid);
        let receipt_path = temp.path().join("receipts/002-output.json");
        let original = fs::read_to_string(&receipt_path).unwrap();
        let duplicate = format!(
            "{{\"duplicate_probe\":1,\"duplicate_probe\":2,{}",
            original.trim_start().trim_start_matches('{')
        );
        fs::write(&receipt_path, duplicate).unwrap();

        let manifest_path = temp.path().join(BUNDLE_MANIFEST);
        let mut manifest: Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["receipts"][1]["sha256"] =
            serde_json::to_value(sha256_file(&receipt_path).unwrap()).unwrap();
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("receipt JSON invalid")
                && error.contains("duplicate key 'duplicate_probe'")));
    }

    #[test]
    fn rejects_missing_receipt_file() {
        let temp = build_agent_turn_bundle(BundleVariant::Valid);
        fs::remove_file(temp.path().join("receipts/003-summary.json")).unwrap();

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("receipt path invalid")));
    }

    #[test]
    fn rejects_broken_prev_event_id() {
        let temp = build_agent_turn_bundle(BundleVariant::BrokenPrevEventId);

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("prev_event_id")));
    }

    #[test]
    fn rejects_journal_event_mutation() {
        let temp = build_agent_turn_bundle(BundleVariant::JournalEventMutation);

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("journal event")));
    }

    #[test]
    fn rejects_unsupported_manifest_version() {
        let temp = build_agent_turn_bundle(BundleVariant::UnsupportedManifestVersion);

        let report = verify_bundle_dir(temp.path());

        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("unsupported bundle_version")));
    }

    enum BundleVariant {
        Valid,
        BrokenPrevEventId,
        JournalEventMutation,
        UnsupportedManifestVersion,
    }

    fn build_agent_turn_bundle(variant: BundleVariant) -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("events")).unwrap();
        fs::create_dir_all(temp.path().join("receipts")).unwrap();
        fs::create_dir_all(temp.path().join("artifacts")).unwrap();

        fs::write(
            temp.path().join("artifacts/input.txt"),
            "review local input\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("artifacts/command-output.txt"),
            "check completed\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("artifacts/summary.md"),
            "portable evidence\n",
        )
        .unwrap();

        let profile = ProfileId::parse(CANONICAL_PROFILE).unwrap();
        let canonicalizer = Canonicalizer::new(profile);
        let mut events = vec![
            json!({
                "event_type": "evidence.recorded",
                "event_version": "1",
                "occurred_at": "2026-05-27T10:00:00Z",
                "principal_id": "agent:local",
                "canonical_profile_id": CANONICAL_PROFILE,
                "artifact_path": "artifacts/input.txt"
            }),
            json!({
                "event_type": "evidence.recorded",
                "event_version": "1",
                "occurred_at": "2026-05-27T10:01:00Z",
                "principal_id": "agent:local",
                "canonical_profile_id": CANONICAL_PROFILE,
                "artifact_path": "artifacts/command-output.txt"
            }),
            json!({
                "event_type": "evidence.recorded",
                "event_version": "1",
                "occurred_at": "2026-05-27T10:02:00Z",
                "principal_id": "agent:local",
                "canonical_profile_id": CANONICAL_PROFILE,
                "artifact_path": "artifacts/summary.md"
            }),
        ];

        let mut previous: Option<Digest> = None;
        for (index, event) in events.iter_mut().enumerate() {
            if let Some(prev) = previous.clone() {
                event["prev_event_id"] = serde_json::to_value(prev).unwrap();
            } else if matches!(variant, BundleVariant::BrokenPrevEventId) {
                event["prev_event_id"] = json!({
                    "alg": "sha-256",
                    "b64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
                });
            }

            if matches!(variant, BundleVariant::BrokenPrevEventId) && index == 1 {
                event["prev_event_id"] = json!({
                    "alg": "sha-256",
                    "b64": "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
                });
            }

            let event_id = compute_event_id(event, &canonicalizer).unwrap();
            event["event_id"] = serde_json::to_value(&event_id).unwrap();
            previous = Some(event_id);
        }

        if matches!(variant, BundleVariant::JournalEventMutation) {
            events[1]["note"] = json!("journal-only mutation");
        }

        let journal_path = temp.path().join("events/events.nrj");
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        for event in &events {
            writer.append_event(event).unwrap();
        }
        writer.finish().unwrap();

        if matches!(variant, BundleVariant::JournalEventMutation) {
            events[1].as_object_mut().unwrap().remove("note");
        }

        let receipt_names = [
            "receipts/001-input.json",
            "receipts/002-output.json",
            "receipts/003-summary.json",
        ];
        for (event, receipt_name) in events.iter().zip(receipt_names) {
            fs::write(
                temp.path().join(receipt_name),
                serde_json::to_vec_pretty(event).unwrap(),
            )
            .unwrap();
        }

        let receipts: Vec<Value> = receipt_names
            .iter()
            .zip(&events)
            .map(|(path, event)| {
                json!({
                    "path": path,
                    "event_id": event["event_id"],
                    "sha256": sha256_file(&temp.path().join(path)).unwrap()
                })
            })
            .collect();

        let artifacts = [
            "artifacts/input.txt",
            "artifacts/command-output.txt",
            "artifacts/summary.md",
        ]
        .iter()
        .map(|path| {
            json!({
                "path": path,
                "content_id": sha256_file(&temp.path().join(path)).unwrap()
            })
        })
        .collect::<Vec<_>>();

        let bundle_version = if matches!(variant, BundleVariant::UnsupportedManifestVersion) {
            2
        } else {
            BUNDLE_VERSION
        };
        let manifest = json!({
            "bundle_format": BUNDLE_FORMAT,
            "bundle_version": bundle_version,
            "canonical_profile_id": CANONICAL_PROFILE,
            "journal": {
                "path": "events/events.nrj",
                "sha256": sha256_file(&journal_path).unwrap()
            },
            "receipts": receipts,
            "artifacts": artifacts
        });

        fs::write(
            temp.path().join(BUNDLE_MANIFEST),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        temp
    }
}
