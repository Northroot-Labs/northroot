#!/usr/bin/env python3
"""Validate a Northroot capability index without external dependencies."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = "northroot.capability-index.v0"
PRIVATE_MARKERS = (
    "/Users/",
    "/Volumes/",
    "op://",
    "sk-",
)


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        value = json.load(f)
    if not isinstance(value, dict):
        raise ValueError("capability index must be a JSON object")
    return value


def strings(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, dict):
        result: list[str] = []
        for child in value.values():
            result.extend(strings(child))
        return result
    if isinstance(value, list):
        result = []
        for child in value:
            result.extend(strings(child))
        return result
    return []


def require_string(capability: dict[str, Any], key: str, path: str, errors: list[str]) -> str | None:
    value = capability.get(key)
    if not isinstance(value, str) or not value:
        errors.append(f"{path}.{key}: must be a non-empty string")
        return None
    return value


def require_string_list(value: Any, path: str, errors: list[str], *, min_items: int = 0) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
        errors.append(f"{path}: must be a list of non-empty strings")
        return []
    if len(value) < min_items:
        errors.append(f"{path}: must contain at least {min_items} item(s)")
    return value


def validate_source_ref(capability: dict[str, Any], path: str, errors: list[str]) -> None:
    source_ref = capability.get("source_ref")
    if not isinstance(source_ref, dict):
        errors.append(f"{path}.source_ref: must be an object")
        return

    repo = source_ref.get("repo")
    commit = source_ref.get("commit")
    subdirectory = source_ref.get("subdirectory")
    if not isinstance(repo, str) or not repo:
        errors.append(f"{path}.source_ref.repo: must be a non-empty string")
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{7,40}", commit):
        errors.append(f"{path}.source_ref.commit: must be a 7-40 character lowercase hex git commit")
    if not isinstance(subdirectory, str) or not subdirectory:
        errors.append(f"{path}.source_ref.subdirectory: must be a non-empty string")
        return
    if subdirectory.startswith(("/", "..")) or "/../" in subdirectory:
        errors.append(f"{path}.source_ref.subdirectory: must be repo-relative")
        return

    package_dir = ROOT / subdirectory
    if not package_dir.is_dir():
        errors.append(f"{path}.source_ref.subdirectory: does not exist: {subdirectory}")
        return

    pyproject = package_dir / "pyproject.toml"
    if pyproject.is_file():
        project = tomllib.loads(pyproject.read_text(encoding="utf-8")).get("project", {})
        package_name = project.get("name")
        package_version = project.get("version")
        if package_name != capability.get("name"):
            errors.append(f"{path}.name: does not match {pyproject} project.name")
        if package_version != capability.get("current_version"):
            errors.append(f"{path}.current_version: does not match {pyproject} project.version")


def validate_install_methods(capability: dict[str, Any], path: str, errors: list[str]) -> None:
    methods = capability.get("install_methods")
    if not isinstance(methods, list) or not methods:
        errors.append(f"{path}.install_methods: must be a non-empty list")
        return

    source_ref = capability.get("source_ref") if isinstance(capability.get("source_ref"), dict) else {}
    commit = source_ref.get("commit")
    subdirectory = source_ref.get("subdirectory")
    kinds = set()
    for index, method in enumerate(methods):
        method_path = f"{path}.install_methods[{index}]"
        if not isinstance(method, dict):
            errors.append(f"{method_path}: must be an object")
            continue
        kind = method.get("kind")
        if not isinstance(kind, str):
            errors.append(f"{method_path}.kind: must be a string")
            continue
        kinds.add(kind)
        if kind == "pip-git-subdirectory":
            ref = method.get("ref")
            if not isinstance(ref, str) or "#subdirectory=" not in ref:
                errors.append(f"{method_path}.ref: must include #subdirectory=")
            else:
                if isinstance(commit, str) and f"@{commit}" not in ref:
                    errors.append(f"{method_path}.ref: must pin source_ref.commit")
                if isinstance(subdirectory, str) and f"#subdirectory={subdirectory}" not in ref:
                    errors.append(f"{method_path}.ref: must pin source_ref.subdirectory")
        elif kind == "local-editable":
            local_path = method.get("path")
            if not isinstance(local_path, str) or local_path.startswith(("/", "..")):
                errors.append(f"{method_path}.path: must be a repo-relative path")
        elif kind == "wheel":
            artifact = method.get("artifact")
            if not isinstance(artifact, str) or not artifact:
                errors.append(f"{method_path}.artifact: must be a non-empty string")
        else:
            errors.append(f"{method_path}.kind: unsupported install method {kind!r}")

    if "pip-git-subdirectory" not in kinds and "wheel" not in kinds:
        errors.append(f"{path}.install_methods: must include a non-local absorption method")


def validate_capability(capability: Any, path: str, errors: list[str]) -> str | None:
    if not isinstance(capability, dict):
        errors.append(f"{path}: must be an object")
        return None

    capability_id = require_string(capability, "capability_id", path, errors)
    if capability_id and not re.fullmatch(r"[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+", capability_id):
        errors.append(f"{path}.capability_id: must be a dotted capability id")

    require_string(capability, "name", path, errors)
    if capability.get("layer") not in {
        "stable-kernel",
        "record-substrate",
        "capability-crate",
        "promoted-package",
        "private-adapter",
    }:
        errors.append(f"{path}.layer: unsupported layer")
    if capability.get("status") not in {"incubating", "usable-dogfood", "promoted", "stable", "deprecated"}:
        errors.append(f"{path}.status: unsupported status")
    if capability.get("visibility") not in {"public-safe", "private", "secret"}:
        errors.append(f"{path}.visibility: unsupported visibility")
    version = require_string(capability, "current_version", path, errors)
    if version and not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        errors.append(f"{path}.current_version: must be semantic version core x.y.z")

    validate_source_ref(capability, path, errors)
    validate_install_methods(capability, path, errors)

    exports = capability.get("exports")
    if not isinstance(exports, dict):
        errors.append(f"{path}.exports: must be an object")
    else:
        require_string_list(exports.get("python_imports"), f"{path}.exports.python_imports", errors)
        require_string_list(exports.get("commands"), f"{path}.exports.commands", errors)

    require_string_list(capability.get("contracts"), f"{path}.contracts", errors, min_items=1)
    require_string_list(capability.get("use_cases"), f"{path}.use_cases", errors, min_items=1)

    verification = capability.get("verification")
    if not isinstance(verification, dict):
        errors.append(f"{path}.verification: must be an object")
    else:
        require_string_list(verification.get("commands"), f"{path}.verification.commands", errors, min_items=1)

    boundaries = capability.get("boundaries")
    if not isinstance(boundaries, dict):
        errors.append(f"{path}.boundaries: must be an object")
    else:
        require_string_list(boundaries.get("owns"), f"{path}.boundaries.owns", errors, min_items=1)
        require_string_list(boundaries.get("does_not_own"), f"{path}.boundaries.does_not_own", errors, min_items=1)

    return capability_id


def validate_index(path: Path) -> list[str]:
    errors: list[str] = []
    try:
        payload = load_json(path)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        return [f"{path}: {exc}"]

    if payload.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"{path}: schema_version must be {SCHEMA_VERSION}")

    capabilities = payload.get("capabilities")
    if not isinstance(capabilities, list) or not capabilities:
        errors.append(f"{path}: capabilities must be a non-empty list")
        return errors

    seen: dict[str, int] = {}
    for index, capability in enumerate(capabilities):
        capability_id = validate_capability(capability, f"$.capabilities[{index}]", errors)
        if capability_id:
            if capability_id in seen:
                errors.append(f"$.capabilities[{index}].capability_id: duplicate also used at index {seen[capability_id]}")
            seen[capability_id] = index

    if path.name.endswith(".public.json"):
        for text in strings(payload):
            for marker in PRIVATE_MARKERS:
                if marker in text:
                    errors.append(f"{path}: public index contains private marker {marker!r}")

    return errors


def main(argv: list[str]) -> int:
    paths = [Path(arg) for arg in argv[1:]] or [ROOT / "capabilities" / "index.public.json"]
    errors: list[str] = []
    for path in paths:
        errors.extend(validate_index(path))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"validated {len(paths)} capability index file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
