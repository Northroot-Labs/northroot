#!/usr/bin/env python3
"""Validate the Northroot kernel boundary is explicit and enforceable."""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python <3.11 fallback is not expected in CI.
    tomllib = None


ROOT = Path(__file__).resolve().parents[1]
BOUNDARY_FILE = ROOT / "kernel-boundary.json"
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"
BOUNDARY_DOC = ROOT / "docs" / "developer" / "kernel-boundary.md"
CODEOWNERS = ROOT / ".github" / "CODEOWNERS"
SIGNATURE_POLICY = ROOT / ".github" / "signature-policy.yml"

REQUIRED_DOC_PHRASES = [
    "Only `northroot-canonical` and `northroot-journal` are stable kernel crates.",
    "`northroot-record`, `northroot-node`, and `northroot-state-eval` are substrate layers above the kernel.",
    "`northroot-governance`, `northroot-execution`, `northroot-exchange`, and `northroot-ag` are capability or profile layers, not kernel crates.",
    "`apps/northroot` is a CLI application, not a kernel crate.",
    "Changing the kernel boundary requires updating `kernel-boundary.json` and passing `scripts/validate_kernel_boundary.py`.",
]

FORBIDDEN_KERNEL_SOURCE_TOKENS = [
    "northroot_record",
    "northroot_node",
    "northroot_state_eval",
    "northroot_governance",
    "northroot_execution",
    "northroot_exchange",
    "northroot_ag",
]


class BoundaryError(Exception):
    pass


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise BoundaryError(f"{path}: cannot read valid JSON: {exc}") from exc


def load_toml(path: Path) -> dict:
    if tomllib is None:
        raise BoundaryError("Python 3.11+ with tomllib is required")
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise BoundaryError(f"{path}: cannot read valid TOML: {exc}") from exc


def crate_entries(boundary: dict) -> list[dict]:
    entries = list(boundary["stable_kernel"]["crates"])
    layers = boundary.get("non_kernel_layers", {})
    for group in ("record_substrate", "capability_profiles"):
        entries.extend(layers.get(group, []))
    return entries


def northroot_dependency_names(manifest: dict) -> set[str]:
    names: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep_name, dep_spec in manifest.get(section, {}).items():
            package_name = dep_name
            if isinstance(dep_spec, dict) and isinstance(dep_spec.get("package"), str):
                package_name = dep_spec["package"]
            if package_name.startswith("northroot-"):
                names.add(package_name)
    return names


def check_workspace(boundary: dict, errors: list[str]) -> None:
    workspace = load_toml(WORKSPACE_MANIFEST)
    members = set(workspace.get("workspace", {}).get("members", []))
    classified = {entry["path"] for entry in crate_entries(boundary)}

    for kernel in boundary["stable_kernel"]["crates"]:
        if kernel["path"] not in members:
            errors.append(f"{WORKSPACE_MANIFEST}: stable kernel crate missing from workspace: {kernel['path']}")

    for path in sorted(members):
        if path.startswith("crates/") and path not in classified:
            errors.append(f"{WORKSPACE_MANIFEST}: workspace crate is not classified in kernel-boundary.json: {path}")

    if "apps/northroot" in members:
        errors.append(f"{WORKSPACE_MANIFEST}: apps/northroot must remain outside the Cargo workspace")


def check_kernel_dependencies(boundary: dict, errors: list[str]) -> None:
    kernel_names = {entry["name"] for entry in boundary["stable_kernel"]["crates"]}
    non_kernel_names = {entry["name"] for entry in crate_entries(boundary) if entry["name"] not in kernel_names}

    for kernel in boundary["stable_kernel"]["crates"]:
        manifest_path = ROOT / kernel["path"] / "Cargo.toml"
        manifest = load_toml(manifest_path)
        package_name = manifest.get("package", {}).get("name")
        if package_name != kernel["name"]:
            errors.append(f"{manifest_path}: package name {package_name!r} does not match boundary entry {kernel['name']!r}")

        allowed = set(kernel.get("allowed_northroot_dependencies", []))
        actual = northroot_dependency_names(manifest)
        unexpected = sorted(actual - allowed)
        if unexpected:
            errors.append(f"{manifest_path}: kernel crate depends on forbidden Northroot crate(s): {', '.join(unexpected)}")

        missing_allowed = sorted(dep for dep in allowed if dep not in actual)
        if missing_allowed:
            errors.append(f"{manifest_path}: declared allowed kernel dependency is not present: {', '.join(missing_allowed)}")

        source_tokens = list(FORBIDDEN_KERNEL_SOURCE_TOKENS)
        if kernel["name"] == "northroot-canonical":
            source_tokens.append("northroot_journal")
        for path in (ROOT / kernel["path"]).rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            for token in source_tokens:
                if token in text:
                    errors.append(f"{path}: kernel source references non-kernel or upward crate token: {token}")

    for name in sorted(non_kernel_names):
        if name in kernel_names:
            errors.append(f"kernel-boundary.json: {name} is both kernel and non-kernel")


def check_boundary_docs(boundary: dict, errors: list[str]) -> None:
    text = BOUNDARY_DOC.read_text(encoding="utf-8") if BOUNDARY_DOC.exists() else ""
    if not text:
        errors.append(f"{BOUNDARY_DOC}: missing boundary document")
        return
    for phrase in REQUIRED_DOC_PHRASES:
        if phrase not in text:
            errors.append(f"{BOUNDARY_DOC}: missing required phrase: {phrase}")

    for protected_path in boundary.get("protected_boundary_paths", []):
        if protected_path not in text:
            errors.append(f"{BOUNDARY_DOC}: missing protected path reference: {protected_path}")


def check_policy_surfaces(boundary: dict, errors: list[str]) -> None:
    codeowners = CODEOWNERS.read_text(encoding="utf-8")
    signature_policy = SIGNATURE_POLICY.read_text(encoding="utf-8")
    for path in (CODEOWNERS, SIGNATURE_POLICY):
        text = path.read_text(encoding="utf-8")
        if "northroot-core" in text:
            errors.append(f"{path}: must not reference nonexistent northroot-core")

    for protected_path in boundary.get("protected_boundary_paths", []):
        if protected_path not in codeowners:
            errors.append(f"{CODEOWNERS}: missing protected kernel boundary path: {protected_path}")
        if f'"{protected_path}"' not in signature_policy:
            errors.append(f"{SIGNATURE_POLICY}: missing Tier B kernel boundary path: {protected_path}")


def main() -> int:
    errors: list[str] = []
    try:
        boundary = load_json(BOUNDARY_FILE)
        if boundary.get("schema") != "northroot.kernel_boundary.v0":
            errors.append(f"{BOUNDARY_FILE}: schema must be northroot.kernel_boundary.v0")
        check_workspace(boundary, errors)
        check_kernel_dependencies(boundary, errors)
        check_boundary_docs(boundary, errors)
        check_policy_surfaces(boundary, errors)
    except BoundaryError as exc:
        errors.append(str(exc))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("validated kernel boundary")
    return 0


if __name__ == "__main__":
    sys.exit(main())
