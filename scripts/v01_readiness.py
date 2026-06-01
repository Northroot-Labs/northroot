#!/usr/bin/env python3
"""Emit the Northroot v0.1 readiness report."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
TARGET_VERSION = "0.1.0"
REQUIRED_DOCS = [
    "LICENSE",
    "RELEASE_GUIDE.md",
    "docs/developer/api-contract.md",
    "docs/developer/script-inventory.md",
    "docs/reference/spec.md",
    "docs/reference/v0.1-stability.md",
    "docs/reference/segmented-journals.md",
    "docs/reference/profiles.md",
    "docs/reference/work-ledger.md",
    "docs/reference/proof-envelope.md",
    "docs/security/README.md",
]
REQUIRED_CLI_COMMANDS = [
    "canonicalize",
    "event-id",
    "append",
    "list",
    "verify",
    "verify-bundle",
    "journal",
    "work",
]
PACKAGE_FILES = [
    ("crates/northroot-canonical/Cargo.toml", "northroot-canonical"),
    ("crates/northroot-journal/Cargo.toml", "northroot-journal"),
    ("apps/northroot/Cargo.toml", "northroot"),
]
LOCK_FILES = [
    "Cargo.lock",
    "apps/northroot/Cargo.lock",
]
STALE_RELEASE_PATTERNS = [
    r"\bv1\.0\.0\b",
    r"\b1\.2\.0\b",
    r"ready for production use",
    r"Future Commands \(Not Yet Implemented\)",
    r"append command is planned but not yet implemented",
    r"LICENSE-APACHE",
    r"LICENSE-MIT",
    r"Apache-2\.0 OR MIT",
    r"MIT OR Apache-2\.0",
    r"docs/operator",
    r"docs/proposals",
    r"reference/extensions\.md",
    r"developer/extending\.md",
]
REMOVED_RELEASE_PATHS = [
    ".devcontainer",
    "Dockerfile",
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "docs/operator",
    "docs/proposals",
    "docs/security/threat-model.md",
    "docs/security/threat-model.json",
    "site",
    "wip",
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit JSON only")
    parser.add_argument(
        "--adoption-report",
        action="append",
        default=[],
        help="extra downstream adoption report path",
    )
    args = parser.parse_args()

    checks: list[dict[str, Any]] = []
    checks.append(check_versions())
    checks.append(check_license())
    checks.append(check_required_docs())
    checks.append(check_release_language())
    checks.append(check_removed_release_surfaces())
    checks.append(check_cli_commands())
    checks.extend(run_validation_scripts())
    checks.append(check_adoption_reports(args.adoption_report))

    report = {
        "schema": "northroot.v0_1.readiness_report.v0",
        "target_version": TARGET_VERSION,
        "valid": all(check["status"] == "pass" for check in checks),
        "checks": checks,
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"Northroot v0.1 readiness: {'pass' if report['valid'] else 'fail'}")
        for check in checks:
            print(f"- {check['name']}: {check['status']} - {check['detail']}")

    return 0 if report["valid"] else 1


def check_versions() -> dict[str, Any]:
    details: list[str] = []
    ok = True

    for relpath, package_name in PACKAGE_FILES:
        version = cargo_toml_version(ROOT / relpath)
        if version == TARGET_VERSION:
            details.append(f"{package_name}={version}")
        else:
            ok = False
            details.append(f"{package_name}={version or 'missing'} expected {TARGET_VERSION}")

    for relpath in LOCK_FILES:
        lock_versions = cargo_lock_versions(ROOT / relpath)
        for package_name in ["northroot-canonical", "northroot-journal", "northroot"]:
            if package_name not in lock_versions:
                if package_name == "northroot" and relpath == "Cargo.lock":
                    continue
                ok = False
                details.append(f"{relpath}:{package_name}=missing")
                continue
            version = lock_versions[package_name]
            if version != TARGET_VERSION:
                ok = False
                details.append(f"{relpath}:{package_name}={version} expected {TARGET_VERSION}")

    return check("version_consistency", ok, "; ".join(details))


def check_license() -> dict[str, Any]:
    findings: list[str] = []
    license_path = ROOT / "LICENSE"
    if not license_path.is_file():
        findings.append("LICENSE missing")
    elif "MIT License" not in license_path.read_text():
        findings.append("LICENSE must contain MIT License text")

    for relpath, package_name in PACKAGE_FILES:
        text = (ROOT / relpath).read_text()
        if 'license = "MIT"' not in text:
            findings.append(f"{package_name} must declare license = \"MIT\"")

    readme = (ROOT / "README.md").read_text()
    if "MIT License ([LICENSE](LICENSE))" not in readme:
        findings.append("README must point to root MIT LICENSE")

    return check(
        "license",
        not findings,
        "project license is MIT" if not findings else "; ".join(findings),
    )


def cargo_toml_version(path: Path) -> str | None:
    text = path.read_text()
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    return match.group(1) if match else None


def cargo_lock_versions(path: Path) -> dict[str, str]:
    versions: dict[str, str] = {}
    current_name: str | None = None
    for line in path.read_text().splitlines():
        name_match = re.match(r'name = "([^"]+)"', line)
        if name_match:
            current_name = name_match.group(1)
            continue
        version_match = re.match(r'version = "([^"]+)"', line)
        if version_match and current_name:
            versions[current_name] = version_match.group(1)
            current_name = None
    return versions


def check_required_docs() -> dict[str, Any]:
    missing = [relpath for relpath in REQUIRED_DOCS if not (ROOT / relpath).is_file()]
    return check(
        "required_docs",
        not missing,
        "all required docs present" if not missing else f"missing: {', '.join(missing)}",
    )


def check_release_language() -> dict[str, Any]:
    scanned = [
        ROOT / "README.md",
        ROOT / "AGENTS.md",
        ROOT / "GOVERNANCE.md",
        ROOT / "RELEASE_GUIDE.md",
        ROOT / "docs/developer/api-contract.md",
        ROOT / "docs/developer/architecture.md",
        ROOT / "docs/developer/layering.md",
        ROOT / "docs/reference/events.md",
        ROOT / "docs/reference/profiles.md",
        ROOT / "docs/reference/spec.md",
        ROOT / "schemas/README.md",
    ]
    findings: list[str] = []
    for path in scanned:
        text = path.read_text()
        for pattern in STALE_RELEASE_PATTERNS:
            if re.search(pattern, text, re.IGNORECASE):
                findings.append(f"{path.relative_to(ROOT)} matches {pattern}")

    stability = " ".join((ROOT / "docs/reference/v0.1-stability.md").read_text().split())
    boundary_rule = (
        "The kernel may preserve and verify profile-bearing events. "
        "The kernel must not decide profile meaning."
    )
    if boundary_rule not in stability:
        findings.append("v0.1 stability contract missing kernel/profile boundary rule")

    return check(
        "release_language",
        not findings,
        "v0.1 release language is consistent" if not findings else "; ".join(findings),
    )


def check_removed_release_surfaces() -> dict[str, Any]:
    present = [relpath for relpath in REMOVED_RELEASE_PATHS if (ROOT / relpath).exists()]
    return check(
        "removed_release_surfaces",
        not present,
        "non-core release surfaces absent" if not present else f"still present: {', '.join(present)}",
    )


def check_cli_commands() -> dict[str, Any]:
    main_rs = (ROOT / "apps/northroot/src/main.rs").read_text()
    missing = []
    for command in REQUIRED_CLI_COMMANDS:
        variant = "".join(part.capitalize() for part in command.split("-"))
        if variant not in main_rs:
            missing.append(command)
    return check(
        "cli_command_surface",
        not missing,
        "required CLI commands present" if not missing else f"missing: {', '.join(missing)}",
    )


def run_validation_scripts() -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []
    for name, command in [
        ("schema_validation", ["python3", "scripts/validate_schemas.py"]),
        ("nrj_fixture_validation", ["python3", "scripts/validate_nrj_fixtures.py"]),
        ("receipt_boundary_validation", ["python3", "scripts/validate_receipt_boundaries.py"]),
    ]:
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
        output = (result.stdout + result.stderr).strip()
        checks.append(
            check(
                name,
                result.returncode == 0,
                "ok" if result.returncode == 0 else truncate(output),
            )
        )
    return checks


def check_adoption_reports(extra_reports: list[str]) -> dict[str, Any]:
    report_paths = sorted((ROOT / "docs/adoption").glob("*.json"))
    report_paths.extend(Path(path) for path in extra_reports)
    passing_reports = []
    failures = []

    for path in report_paths:
        try:
            data = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as exc:
            failures.append(f"{path}: unreadable adoption report: {exc}")
            continue

        report_errors = validate_adoption_report(data)
        if report_errors:
            failures.append(f"{path}: {', '.join(report_errors)}")
        elif data.get("passed") is True:
            passing_reports.append(data.get("repo"))
        else:
            failures.append(f"{path}: passed must be true")

    if len(set(passing_reports)) < 2:
        failures.append(
            f"expected at least 2 passing downstream repos, found {len(set(passing_reports))}"
        )

    return check(
        "downstream_adoption",
        not failures,
        f"passing repos: {', '.join(sorted(set(passing_reports)))}"
        if not failures
        else "; ".join(failures),
    )


def validate_adoption_report(data: dict[str, Any]) -> list[str]:
    errors = []
    for field in ["schema", "repo", "check_name", "journal_path"]:
        if not isinstance(data.get(field), str) or not data[field]:
            errors.append(f"{field} must be a non-empty string")
    if data.get("schema") != "northroot.v0_1.adoption_report.v0":
        errors.append("schema must be northroot.v0_1.adoption_report.v0")

    for field in [
        "event_count",
        "kernel_valid_event_count",
        "profile_valid_event_count",
        "invalid_event_count",
    ]:
        if not isinstance(data.get(field), int) or data[field] < 0:
            errors.append(f"{field} must be a non-negative integer")

    if data.get("kernel_valid_event_count", 0) < data.get("profile_valid_event_count", 0):
        errors.append("profile_valid_event_count cannot exceed kernel_valid_event_count")
    if data.get("event_count", 0) < data.get("kernel_valid_event_count", 0):
        errors.append("kernel_valid_event_count cannot exceed event_count")
    if data.get("invalid_event_count", 0) != 0:
        errors.append("invalid_event_count must be 0")
    if data.get("projection_rebuilt") is not True:
        errors.append("projection_rebuilt must be true")
    return errors


def check(name: str, ok: bool, detail: str) -> dict[str, Any]:
    return {
        "name": name,
        "status": "pass" if ok else "fail",
        "detail": detail,
    }


def truncate(value: str, limit: int = 800) -> str:
    if len(value) <= limit:
        return value
    return value[: limit - 3] + "..."


if __name__ == "__main__":
    sys.exit(main())
