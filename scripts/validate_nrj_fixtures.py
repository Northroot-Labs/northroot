#!/usr/bin/env python3
"""Validate checked-in NRJ fixtures against the documented wire format."""

from __future__ import annotations

import json
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_DIR = ROOT / "fixtures" / "nrj"
EXPECTED_HEADER = bytes.fromhex("4e524a31010000000000000000000000")
EVENT_JSON_KIND = 0x01
FRAME_HEADER_SIZE = 8
MAX_PAYLOAD_SIZE = 16 * 1024 * 1024


def validate_fixture(path: Path) -> list[str]:
    errors: list[str] = []
    data = path.read_bytes()

    if len(data) < 16:
        return [f"{path}: file shorter than 16-byte NRJ header"]

    if data[:16] != EXPECTED_HEADER:
        errors.append(
            f"{path}: invalid header {data[:16].hex(' ')}, expected {EXPECTED_HEADER.hex(' ')}"
        )

    offset = 16
    event_frames = 0
    while offset < len(data):
        if offset + FRAME_HEADER_SIZE > len(data):
            errors.append(f"{path}: truncated frame header at offset {offset}")
            break

        frame = data[offset : offset + FRAME_HEADER_SIZE]
        kind = frame[0]
        reserved = frame[1:4]
        payload_len = struct.unpack("<I", frame[4:8])[0]
        offset += FRAME_HEADER_SIZE

        if reserved != b"\x00\x00\x00":
            errors.append(f"{path}: non-zero frame reserved bytes at offset {offset - 7}")
        if payload_len > MAX_PAYLOAD_SIZE:
            errors.append(f"{path}: payload too large: {payload_len}")
        if offset + payload_len > len(data):
            errors.append(f"{path}: truncated payload at offset {offset}")
            break

        payload = data[offset : offset + payload_len]
        offset += payload_len

        if kind == EVENT_JSON_KIND:
            event_frames += 1
            try:
                value: Any = json.loads(payload.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                errors.append(f"{path}: invalid EventJson payload: {exc}")
                continue
            if not isinstance(value, dict):
                errors.append(f"{path}: EventJson payload must be a JSON object")
            elif "event_id" not in value:
                errors.append(f"{path}: EventJson payload missing event_id")

    if event_frames == 0:
        errors.append(f"{path}: no EventJson frames found")

    return errors


def verify_with_cli(path: Path) -> list[str]:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(ROOT / "apps/northroot/Cargo.toml"),
            "--bin",
            "northroot",
            "--",
            "verify",
            str(path),
            "--json",
            "--strict",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        return [f"{path}: northroot verify failed: {result.stderr.strip()}"]
    return []


def main() -> int:
    fixtures = sorted(FIXTURE_DIR.glob("*.nrj"))
    if not fixtures:
        print(f"{FIXTURE_DIR}: no .nrj fixtures found", file=sys.stderr)
        return 1

    errors: list[str] = []
    for fixture in fixtures:
        errors.extend(validate_fixture(fixture))
        errors.extend(verify_with_cli(fixture))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"validated {len(fixtures)} NRJ fixture(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
