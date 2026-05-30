#!/usr/bin/env python3
"""Validate state-recovery invariant docs stay narrow and enforceable."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs" / "reference" / "state-recovery-invariants-v0.md"
WORK_LEDGER = ROOT / "docs" / "reference" / "work-ledger.md"
CORE_INVARIANTS = ROOT / "CORE_INVARIANTS.md"

REQUIRED_IDS = [f"SR-{index}" for index in range(1, 9)]
REQUIRED_PHRASES = [
    ".nrj",
    "canonical append-only event log",
    "Projections are derived read models",
    "Snapshots are content-addressed recovery artifacts",
    "Backups are disaster-recovery byte stores",
    "must not schedule, dispatch, lease, approve, execute, or decide",
    "Orchestration belongs outside `northroot`",
]
FORBIDDEN_AUTHORITY = [
    r"\bnorthroot owns control-plane orchestration\b",
    r"\bnorthroot owns queueing\b",
    r"\bnorthroot owns dispatch\b",
    r"\bnorthroot owns leases\b",
    r"\bnorthroot owns workers\b",
    r"\bnorthroot owns product workflows\b",
]


def main() -> int:
    errors: list[str] = []
    text = DOC.read_text(encoding="utf-8")

    for invariant_id in REQUIRED_IDS:
        if f"{invariant_id}:" not in text:
            errors.append(f"{DOC}: missing invariant {invariant_id}")

    for phrase in REQUIRED_PHRASES:
        if phrase not in text:
            errors.append(f"{DOC}: missing required phrase: {phrase}")

    for pattern in FORBIDDEN_AUTHORITY:
        if re.search(pattern, text, flags=re.IGNORECASE):
            errors.append(f"{DOC}: forbidden authority phrase matched: {pattern}")

    if "state-recovery-invariants-v0.md" not in WORK_LEDGER.read_text(encoding="utf-8"):
        errors.append(f"{WORK_LEDGER}: must link state-recovery invariants")

    core_text = CORE_INVARIANTS.read_text(encoding="utf-8")
    if "orchestration" not in core_text or "execution engines" not in core_text:
        errors.append(f"{CORE_INVARIANTS}: must preserve non-goal anchors")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("validated state recovery invariants")
    return 0


if __name__ == "__main__":
    sys.exit(main())
