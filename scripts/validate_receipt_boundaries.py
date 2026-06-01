#!/usr/bin/env python3
"""Validate receipt terminology stays outside the neutral core boundary."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PROOF_DOC = ROOT / "docs" / "reference" / "proof-envelope.md"
PLATFORM_DOC = ROOT / "docs" / "reference" / "platform.md"
RECEIPT_SCHEMA = ROOT / "schemas" / "platform" / "v1" / "receipt.schema.json"
REFS_SCHEMA = ROOT / "schemas" / "platform" / "v1" / "refs.schema.json"
GOVERNANCE = ROOT / "GOVERNANCE.md"
CONTRIBUTING = ROOT / "CONTRIBUTING.md"
AGENTS = ROOT / "AGENTS.md"

REQUIRED = {
    PROOF_DOC: [
        "Northroot core verifies generic proof envelopes",
        "Receipt is a platform, profile, or domain name for a proof envelope",
        "Receipt-shaped bundle fields such as `receipts` remain valid compatibility terminology",
        "it does not define payment, settlement, work acceptance, policy approval, backup success, or other domain semantics",
    ],
    PLATFORM_DOC: [
        "Proof envelopes / verifiable events",
        "receipt-profile `event_id`s",
        "Receipt is compatibility/profile terminology, not a core semantic",
    ],
    RECEIPT_SCHEMA: [
        "Platform Receipt Profile Envelope",
        "Platform receipt profile over the generic Northroot proof envelope",
        "domain layers supply receipt semantics via profiles or consuming protocols",
    ],
    REFS_SCHEMA: [
        "receipt-profile proof envelope",
    ],
    GOVERNANCE: [
        "Proof Envelopes Are the Primary Artifact",
        "Receipt is a platform, profile, or domain name for a proof envelope",
        "does not define",
    ],
    CONTRIBUTING: [
        "Proof envelopes / verifiable events are the primary artifact for audit",
    ],
    AGENTS: [
        "Proof envelopes / verifiable events are the primary artifact for audit",
    ],
}

FORBIDDEN_EXACT = [
    "Receipts are the primary artifact",
    "Northroot core defines receipt semantics",
]

DOMAIN_RECEIPT_PATTERN = re.compile(
    r"\breceipts?\b.{0,80}\b("
    r"payment|settlement|settled|work acceptance|work accepted|"
    r"policy approval|policy approved|backup success|backup succeeded"
    r")\b",
    flags=re.IGNORECASE | re.DOTALL,
)

DOMAIN_ALLOWED = {
    PROOF_DOC,
    ROOT / "schemas" / "extensions" / "work_ledger" / "v0" / "work_event.schema.json",
    ROOT / "docs" / "reference" / "work-ledger.md",
}


def source_paths() -> list[Path]:
    roots = [
        ROOT / "AGENTS.md",
        ROOT / "CONTRIBUTING.md",
        ROOT / "GOVERNANCE.md",
        ROOT / "crates",
        ROOT / "docs",
        ROOT / "schemas",
        ROOT / "apps" / "northroot" / "src",
    ]
    paths: list[Path] = []
    for root in roots:
        if root.is_file():
            paths.append(root)
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix in {".md", ".json", ".rs", ".toml"}:
                paths.append(path)
    return sorted(paths)


def normalize(text: str) -> str:
    return " ".join(text.split())


def main() -> int:
    errors: list[str] = []

    for path, phrases in REQUIRED.items():
        if not path.exists():
            errors.append(f"{path}: missing required receipt-boundary file")
            continue
        text = normalize(path.read_text(encoding="utf-8"))
        for phrase in phrases:
            if phrase not in text:
                errors.append(f"{path}: missing required phrase: {phrase}")

    for path in source_paths():
        text = path.read_text(encoding="utf-8")
        for phrase in FORBIDDEN_EXACT:
            if phrase in text:
                errors.append(f"{path}: forbidden unqualified receipt core phrase: {phrase}")

        if path not in DOMAIN_ALLOWED and "schemas/extensions" not in path.as_posix():
            if DOMAIN_RECEIPT_PATTERN.search(text):
                errors.append(f"{path}: domain receipt semantics must stay in proof-envelope docs or profile/consumer surfaces")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("validated receipt boundary terminology")
    return 0


if __name__ == "__main__":
    sys.exit(main())
