#!/usr/bin/env python3
"""Compare JSONL and NRJ for intended Northroot journal workloads.

This script is a reproducible proof harness, not a microbenchmark gate. It
generates equivalent isolated JSONL and NRJ streams, checks that both contain
the same workload classes, verifies the NRJ stream through the CLI, and reports
the policies each format forces consumers to own.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import struct
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any


DEFAULT_COUNTS = {
    "codex_session_meta": 1_000,
    "work_observed": 1_000,
    "tool_call": 1_500,
    "tool_output_small": 1_500,
    "tool_output_large": 400,
    "snapshot_generated_pointer": 100,
    "snapshot_payload_stress_not_recommended": 20,
}

NRJ_HEADER = b"NRJ1" + struct.pack("<HH", 1, 0) + (b"\0" * 8)
EVENT_JSON_KIND = 0x01


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate equivalent JSONL and NRJ workload streams and compare format complexity."
    )
    parser.add_argument(
        "--out",
        default="/tmp/northroot-format-complexity-bench",
        help="Output directory for generated streams and report.",
    )
    parser.add_argument(
        "--northroot-bin",
        default=None,
        help="Path to a built northroot CLI. Defaults to apps/northroot/target/debug/northroot.",
    )
    parser.add_argument(
        "--scale",
        type=float,
        default=1.0,
        help="Scale default workload counts. Use 0.1 for a quick smoke run.",
    )
    parser.add_argument(
        "--keep",
        action="store_true",
        help="Keep any existing output directory contents instead of replacing them.",
    )
    parser.add_argument(
        "--quiet",
        action="store_true",
        help="Write the JSON report but print only the report path.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def northroot_bin(root: Path, override: str | None) -> Path:
    path = Path(override) if override else root / "apps/northroot/target/debug/northroot"
    if path.exists():
        return path
    subprocess.run(
        ["cargo", "build", "--manifest-path", "apps/northroot/Cargo.toml"],
        cwd=root,
        check=True,
    )
    if not path.exists():
        raise FileNotFoundError(f"northroot binary not found: {path}")
    return path


def event_with_id(binary: Path, root: Path, event: dict[str, Any]) -> dict[str, Any]:
    unsigned = dict(event)
    unsigned.pop("event_id", None)
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(unsigned, handle, separators=(",", ":"), sort_keys=True)
        temp_name = handle.name
    try:
        result = subprocess.run(
            [str(binary), "event-id", temp_name],
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
    finally:
        os.unlink(temp_name)
    signed = dict(unsigned)
    signed["event_id"] = {"alg": "sha-256", "b64": result.stdout.strip()}
    return signed


def scaled_counts(scale: float) -> dict[str, int]:
    if scale <= 0:
        raise ValueError("--scale must be greater than zero")
    return {name: max(1, round(count * scale)) for name, count in DEFAULT_COUNTS.items()}


def workload_prototypes(binary: Path, root: Path) -> dict[str, dict[str, Any]]:
    base = {
        "canonical_profile_id": "northroot-canonical-v1",
        "event_version": "0",
        "occurred_at": "2026-05-30T20:00:00Z",
        "principal_id": "agent:codex",
        "schema": "northroot.work_ledger.v0",
        "work_id": "work:bench",
        "source_system": "complexity_bench",
        "source": {
            "kind": "synthetic_bench",
            "path": "/tmp/northroot-format-complexity-bench/source.jsonl",
            "line": 1,
        },
    }
    large_output = (
        "line 0000: tool output with paths /tmp/build and /Users/example "
        "plus redacted-looking token sk-REDACTED\n"
        * 48
    )
    snapshot_state = {
        "workspace_id": "bench",
        "schema": "northroot.workspace_snapshot.v0",
        "generated_at": "2026-05-30T20:00:00Z",
        "state": {
            "open_work": 17,
            "completed_work": 432,
            "evidence_count": 1284,
            "runs": [
                {
                    "run_id": f"run:{index}",
                    "status": "completed",
                    "evidence": [f"evidence:{evidence_index}" for evidence_index in range(8)],
                }
                for index in range(80)
            ],
        },
        "projection_sections": [
            {
                "name": "work_ledger_summary",
                "event_count": 6000,
                "tip_event_id": "event:tip",
            }
        ],
        "evidence_index_sections": [
            {
                "kind": "digest_index",
                "items": [
                    {
                        "digest": f"sha256:{index:064x}",
                        "path": f"artifacts/{index}.json",
                    }
                    for index in range(80)
                ],
            }
        ],
    }

    return {
        "codex_session_meta": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "run.observed",
                "run_id": "run:codex-session",
                "observation": {
                    "session_id": "session:abc",
                    "cwd": str(root),
                    "model": "gpt-5-codex",
                    "turn_id": "turn:1",
                },
            },
        ),
        "work_observed": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "work.observed",
                "observation": {
                    "title": "Implement state recovery slice",
                    "status": "open",
                    "labels": ["northroot", "snapshot", "journal"],
                },
            },
        ),
        "tool_call": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "artifact.observed",
                "run_id": "run:codex-session",
                "artifact": {
                    "kind": "tool_call",
                    "tool_name": "exec_command",
                    "arguments_digest": {"alg": "sha-256", "b64": "abc"},
                    "command": "cargo test --workspace",
                    "touched_paths": [
                        "apps/northroot/src/commands/work.rs",
                        "crates/northroot-journal/src/reader.rs",
                    ],
                },
            },
        ),
        "tool_output_small": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "artifact.observed",
                "run_id": "run:codex-session",
                "artifact": {
                    "kind": "tool_output",
                    "call_id": "call:1",
                    "exit_code": 0,
                    "output_digest": {"alg": "sha-256", "b64": "def"},
                    "output_len": 181,
                },
            },
        ),
        "tool_output_large": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "artifact.observed",
                "run_id": "run:codex-session",
                "artifact": {
                    "kind": "tool_output",
                    "call_id": "call:2",
                    "exit_code": 0,
                    "output_digest": {"alg": "sha-256", "b64": "ghi"},
                    "output_len": len(large_output),
                    "sample": large_output[:1600],
                },
            },
        ),
        "snapshot_generated_pointer": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "snapshot.generated",
                "snapshot": {
                    "manifest_digest": {"alg": "sha-256", "b64": "snapmanifest"},
                    "snapshot": f"snapshot:sha256:{'a' * 64}",
                    "covered_event_count": 6000,
                    "covered_tip_event_id": {"alg": "sha-256", "b64": "tip"},
                    "covered_journal_byte_offset": 1_234_567,
                },
            },
        ),
        "snapshot_payload_stress_not_recommended": event_with_id(
            binary,
            root,
            {
                **base,
                "event_type": "artifact.observed",
                "run_id": "run:codex-session",
                "artifact": {
                    "kind": "snapshot_payload_stress_not_recommended",
                    "payload": snapshot_state,
                },
            },
        ),
    }


def classify(event: dict[str, Any]) -> str:
    event_type = event.get("event_type")
    artifact = event.get("artifact") if isinstance(event.get("artifact"), dict) else {}
    if event_type == "run.observed":
        return "codex_session_meta"
    if event_type == "work.observed":
        return "work_observed"
    if event_type == "snapshot.generated":
        return "snapshot_generated_pointer"
    if artifact.get("kind") == "tool_call":
        return "tool_call"
    if artifact.get("kind") == "tool_output":
        return "tool_output_large" if artifact.get("output_len", 0) > 1000 else "tool_output_small"
    if artifact.get("kind") == "snapshot_payload_stress_not_recommended":
        return "snapshot_payload_stress_not_recommended"
    return "unknown"


def write_jsonl(path: Path, sequence: list[str], payloads: dict[str, bytes]) -> float:
    start = time.perf_counter()
    with path.open("wb") as handle:
        for name in sequence:
            handle.write(payloads[name] + b"\n")
    return time.perf_counter() - start


def write_nrj(path: Path, sequence: list[str], payloads: dict[str, bytes]) -> float:
    start = time.perf_counter()
    with path.open("wb") as handle:
        handle.write(NRJ_HEADER)
        for name in sequence:
            payload = payloads[name]
            handle.write(struct.pack("<B3sI", EVENT_JSON_KIND, b"\0\0\0", len(payload)))
            handle.write(payload)
    return time.perf_counter() - start


def read_jsonl(path: Path, expected_classes: dict[str, int]) -> tuple[float, int, dict[str, int]]:
    per_class = {name: 0 for name in expected_classes}
    start = time.perf_counter()
    total = 0
    with path.open("rb") as handle:
        for line in handle:
            event = json.loads(line)
            per_class[classify(event)] += 1
            total += 1
    return time.perf_counter() - start, total, per_class


def read_nrj(path: Path, expected_classes: dict[str, int]) -> tuple[float, int, dict[str, int]]:
    per_class = {name: 0 for name in expected_classes}
    start = time.perf_counter()
    total = 0
    with path.open("rb") as handle:
        header = handle.read(16)
        if header != NRJ_HEADER:
            raise ValueError("invalid NRJ header")
        while True:
            frame_header = handle.read(8)
            if not frame_header:
                break
            if len(frame_header) != 8:
                raise ValueError("partial NRJ frame header")
            kind, reserved, length = struct.unpack("<B3sI", frame_header)
            if kind != EVENT_JSON_KIND or reserved != b"\0\0\0":
                raise ValueError("unsupported NRJ frame")
            body = handle.read(length)
            if len(body) != length:
                raise ValueError("partial NRJ frame body")
            event = json.loads(body)
            per_class[classify(event)] += 1
            total += 1
    return time.perf_counter() - start, total, per_class


def trunc_jsonl(path: Path) -> dict[str, Any]:
    complete = 0
    offset = 0
    with path.open("rb") as handle:
        for line in handle:
            start = offset
            offset += len(line)
            if not line.endswith(b"\n"):
                return {
                    "complete_records": complete,
                    "failure": "partial_line_or_missing_newline",
                    "offset": start,
                }
            json.loads(line)
            complete += 1
    return {"complete_records": complete, "failure": None, "offset": offset}


def trunc_nrj(path: Path) -> dict[str, Any]:
    complete = 0
    offset = 16
    with path.open("rb") as handle:
        header = handle.read(16)
        if len(header) != 16:
            return {"complete_records": 0, "failure": "partial_header", "offset": 0}
        while True:
            frame_header = handle.read(8)
            if not frame_header:
                return {"complete_records": complete, "failure": None, "offset": offset}
            if len(frame_header) != 8:
                return {
                    "complete_records": complete,
                    "failure": "partial_frame_header",
                    "offset": offset,
                }
            _, _, length = struct.unpack("<B3sI", frame_header)
            offset += 8
            body = handle.read(length)
            if len(body) != length:
                return {
                    "complete_records": complete,
                    "failure": "partial_frame_body",
                    "offset": offset,
                    "expected_len": length,
                    "actual_len": len(body),
                }
            offset += length
            complete += 1


def format_policy_failures(
    out: Path,
    jsonl_path: Path,
    nrj_path: Path,
    counts: dict[str, int],
) -> dict[str, str]:
    failures: dict[str, str] = {}
    jsonl_comment = out / "mixed-workload-comment.jsonl"
    jsonl_comment.write_bytes(jsonl_path.read_bytes() + b"# operator note\n")
    nrj_unknown = out / "mixed-workload-unknown-frame.nrj"
    nrj_unknown.write_bytes(
        nrj_path.read_bytes() + struct.pack("<B3sI", 0x7F, b"\0\0\0", 7) + b"ignored"
    )
    checks = [
        ("jsonl_comment_line", read_jsonl, jsonl_comment),
        ("nrj_unknown_frame", read_nrj, nrj_unknown),
    ]
    for label, reader, path in checks:
        try:
            reader(path, counts)
            failures[label] = "accepted"
        except Exception as exc:  # noqa: BLE001 - report format policy behavior
            failures[label] = f"{type(exc).__name__}: {str(exc)[:120]}"
    return failures


def assert_report(report: dict[str, Any]) -> None:
    expected = report["workload_counts"]
    if report["jsonl"]["records_read"] != report["event_total"]:
        raise AssertionError("JSONL record count mismatch")
    if report["nrj"]["records_read"] != report["event_total"]:
        raise AssertionError("NRJ record count mismatch")
    if report["jsonl"]["per_class"] != expected:
        raise AssertionError("JSONL workload class mismatch")
    if report["nrj"]["per_class"] != expected:
        raise AssertionError("NRJ workload class mismatch")
    if report["nrj"]["cli_verify_first_event_exit"] != 0:
        raise AssertionError("northroot verify failed for generated NRJ stream")
    if report["jsonl"]["truncated"]["complete_records"] != report["event_total"] - 1:
        raise AssertionError("JSONL truncation scan did not preserve expected complete records")
    if report["nrj"]["truncated"]["complete_records"] != report["event_total"] - 1:
        raise AssertionError("NRJ truncation scan did not preserve expected complete records")


def main() -> int:
    args = parse_args()
    root = repo_root()
    binary = northroot_bin(root, args.northroot_bin)
    out = Path(args.out)
    if out.exists() and not args.keep:
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)

    counts = scaled_counts(args.scale)
    prototypes = workload_prototypes(binary, root)
    payloads = {
        name: json.dumps(prototypes[name], separators=(",", ":"), sort_keys=True).encode("utf-8")
        for name in counts
    }
    sequence: list[str] = []
    for name, count in counts.items():
        sequence.extend([name] * count)

    jsonl_path = out / "mixed-workload.jsonl"
    nrj_path = out / "mixed-workload.nrj"
    jsonl_write = write_jsonl(jsonl_path, sequence, payloads)
    nrj_write = write_nrj(nrj_path, sequence, payloads)
    jsonl_read, jsonl_total, jsonl_per_class = read_jsonl(jsonl_path, counts)
    nrj_read, nrj_total, nrj_per_class = read_nrj(nrj_path, counts)

    jsonl_truncated = out / "mixed-workload-truncated.jsonl"
    nrj_truncated = out / "mixed-workload-truncated.nrj"
    jsonl_truncated.write_bytes(jsonl_path.read_bytes()[:-113])
    nrj_truncated.write_bytes(nrj_path.read_bytes()[:-113])

    verify = subprocess.run(
        [str(binary), "verify", str(nrj_path), "--max-events", "1", "--json", "--strict"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    report = {
        "workdir": str(out),
        "event_total": len(sequence),
        "workload_counts": counts,
        "workload_payload_bytes": {name: len(payloads[name]) for name in counts},
        "jsonl": {
            "bytes": jsonl_path.stat().st_size,
            "write_seconds": round(jsonl_write, 6),
            "read_parse_classify_seconds": round(jsonl_read, 6),
            "records_read": jsonl_total,
            "per_class": jsonl_per_class,
            "truncated": trunc_jsonl(jsonl_truncated),
        },
        "nrj": {
            "bytes": nrj_path.stat().st_size,
            "write_seconds": round(nrj_write, 6),
            "read_parse_classify_seconds": round(nrj_read, 6),
            "records_read": nrj_total,
            "per_class": nrj_per_class,
            "truncated": trunc_nrj(nrj_truncated),
            "cli_verify_first_event_exit": verify.returncode,
            "cli_verify_first_event_stderr": verify.stderr.strip(),
        },
        "size_delta_bytes": nrj_path.stat().st_size - jsonl_path.stat().st_size,
        "size_delta_percent": round(
            ((nrj_path.stat().st_size / jsonl_path.stat().st_size) - 1) * 100,
            4,
        ),
        "format_policy_failures": format_policy_failures(out, jsonl_path, nrj_path, counts),
        "complexity_observations": {
            "jsonl_extra_policies_needed": [
                "canonical header/version sidecar or convention",
                "line discipline for partial final record",
                "comment/blank-line policy",
                "unknown record envelope policy",
                "separate schema/export contract to avoid treating ad hoc JSONL as canonical",
            ],
            "nrj_extra_policies_needed": [
                "adapter/export path for humans and external systems",
                "tooling for inspecting frames",
                "policy for unknown non-EventJson frames",
                "do not use journal frames as workspace bundle or backup format",
            ],
            "snapshot_policy": (
                "snapshot.generated pointer belongs in the event log; full workspace snapshot "
                "payload remains content-addressed JSON artifact outside the journal"
            ),
        },
    }
    assert_report(report)
    report_path = out / "format-complexity-report.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    if not args.quiet:
        print(json.dumps(report, indent=2, sort_keys=True))
        print()
    print(f"report: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
