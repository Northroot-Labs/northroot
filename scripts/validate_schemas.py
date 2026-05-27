#!/usr/bin/env python3
"""Validate Northroot JSON schema files with repo-local invariants."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urldefrag


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_ROOT = ROOT / "schemas"
REMOTE_PREFIX = "https://northroot.dev/schemas/"


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        value = json.load(f)
    if not isinstance(value, dict):
        raise ValueError("schema root must be a JSON object")
    return value


def walk_refs(value: Any) -> list[str]:
    refs: list[str] = []
    if isinstance(value, dict):
        ref = value.get("$ref")
        if isinstance(ref, str):
            refs.append(ref)
        for child in value.values():
            refs.extend(walk_refs(child))
    elif isinstance(value, list):
        for child in value:
            refs.extend(walk_refs(child))
    return refs


def resolve_pointer(document: Any, pointer: str) -> bool:
    if pointer in ("", "/"):
        return True
    if not pointer.startswith("/"):
        return False

    current = document
    for raw_part in pointer.lstrip("/").split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and part in current:
            current = current[part]
        elif isinstance(current, list) and part.isdigit() and int(part) < len(current):
            current = current[int(part)]
        else:
            return False
    return True


def ref_to_path(current_path: Path, ref: str) -> tuple[Path, str] | None:
    url, fragment = urldefrag(ref)
    if not url:
        return current_path, fragment
    if url.startswith(("http://", "https://")):
        if not url.startswith(REMOTE_PREFIX):
            return None
        relative = url.removeprefix(REMOTE_PREFIX)
        return (SCHEMA_ROOT / relative).resolve(), fragment
    return (current_path.parent / url).resolve(), fragment


def schema_paths() -> list[Path]:
    return sorted(SCHEMA_ROOT.rglob("*.schema.json"))


def validate_schema(path: Path, loaded: dict[Path, dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    data = loaded[path]

    if data.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        errors.append(f"{path}: $schema must be draft 2020-12")

    schema_id = data.get("$id")
    if not isinstance(schema_id, str) or not schema_id.startswith(REMOTE_PREFIX):
        errors.append(f"{path}: $id must be a northroot.dev schema URL")

    if "title" not in data:
        errors.append(f"{path}: missing title")

    for ref in walk_refs(data):
        resolved = ref_to_path(path, ref)
        if resolved is None:
            errors.append(f"{path}: unsupported external $ref {ref}")
            continue

        target_path, fragment = resolved
        if target_path not in loaded:
            errors.append(f"{path}: $ref target file not found: {ref}")
            continue

        if fragment and not resolve_pointer(loaded[target_path], fragment):
            errors.append(f"{path}: $ref target pointer not found: {ref}")

    return errors


def validate_platform_policy(path: Path, data: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if "/platform/" not in str(path):
        return errors

    properties = data.get("properties", {})
    if not isinstance(properties, dict):
        errors.append(f"{path}: platform schema properties must be an object")
        return errors

    if "event_id" in properties:
        ref_text = json.dumps(properties["event_id"], sort_keys=True)
        if "ids.schema.json#/$defs/EventID" not in ref_text:
            errors.append(f"{path}: event_id must reference platform EventID")

    return errors


def main() -> int:
    paths = schema_paths()
    if not paths:
        print("no schema files found", file=sys.stderr)
        return 1

    loaded: dict[Path, dict[str, Any]] = {}
    errors: list[str] = []

    for path in paths:
        try:
            loaded[path.resolve()] = load_json(path)
        except (json.JSONDecodeError, ValueError) as exc:
            errors.append(f"{path}: {exc}")

    id_values: dict[str, Path] = {}
    for path, data in loaded.items():
        schema_id = data.get("$id")
        if isinstance(schema_id, str):
            if schema_id in id_values:
                errors.append(f"{path}: duplicate $id also used by {id_values[schema_id]}")
            id_values[schema_id] = path

    for path in sorted(loaded):
        errors.extend(validate_schema(path, loaded))
        errors.extend(validate_platform_policy(path, loaded[path]))

    for path in sorted(loaded):
        text = path.read_text(encoding="utf-8")
        if re.search(r"northroot-(core|store|cli)", text):
            errors.append(f"{path}: references a known non-core crate/package name")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"validated {len(paths)} schema files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
