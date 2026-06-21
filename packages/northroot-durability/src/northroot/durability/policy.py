"""Tiered durability and public-commit policy."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from enum import StrEnum
from pathlib import Path
from typing import Iterable


class BackupMode(StrEnum):
    CATALOG_ONLY = "catalog-only"
    ENCRYPTED_DR = "encrypted-dr"
    VERIFIED_OFFLOAD = "verified-offload"
    KNOWLEDGE_EXTRACT = "knowledge-extract"
    PRUNE_AFTER_RESTORE_PROOF = "prune-after-restore-proof"


@dataclass(frozen=True)
class ModePolicy:
    mode: BackupMode
    purpose: str
    encrypted_required: bool
    raw_payloads_allowed: bool
    public_commit_allowed: bool
    local_prune_allowed: bool
    required_evidence: tuple[str, ...]


MODE_POLICIES: dict[BackupMode, ModePolicy] = {
    BackupMode.CATALOG_ONLY: ModePolicy(
        BackupMode.CATALOG_ONLY,
        "read-only inventory, path classification, and planning",
        encrypted_required=False,
        raw_payloads_allowed=False,
        public_commit_allowed=True,
        local_prune_allowed=False,
        required_evidence=("redacted-example-or-generic-schema",),
    ),
    BackupMode.ENCRYPTED_DR: ModePolicy(
        BackupMode.ENCRYPTED_DR,
        "encrypted restic disaster recovery for raw machine and project data",
        encrypted_required=True,
        raw_payloads_allowed=True,
        public_commit_allowed=False,
        local_prune_allowed=False,
        required_evidence=("restic-snapshot-id", "restore-test-or-verify-result"),
    ),
    BackupMode.VERIFIED_OFFLOAD: ModePolicy(
        BackupMode.VERIFIED_OFFLOAD,
        "copy payloads to external media with manifest and restore script",
        encrypted_required=False,
        raw_payloads_allowed=True,
        public_commit_allowed=False,
        local_prune_allowed=False,
        required_evidence=("source-inventory", "copy-manifest", "restore-script", "copy-verification"),
    ),
    BackupMode.KNOWLEDGE_EXTRACT: ModePolicy(
        BackupMode.KNOWLEDGE_EXTRACT,
        "extract reviewed knowledge while keeping raw tool state private",
        encrypted_required=False,
        raw_payloads_allowed=False,
        public_commit_allowed=False,
        local_prune_allowed=False,
        required_evidence=("extraction-summary", "review-decision"),
    ),
    BackupMode.PRUNE_AFTER_RESTORE_PROOF: ModePolicy(
        BackupMode.PRUNE_AFTER_RESTORE_PROOF,
        "local removal only after backup, offload, restore proof, and review",
        encrypted_required=False,
        raw_payloads_allowed=False,
        public_commit_allowed=False,
        local_prune_allowed=True,
        required_evidence=(
            "encrypted-dr-snapshot-id",
            "verified-offload-manifest",
            "restore-script",
            "human-review-ack",
        ),
    ),
}

PUBLIC_BLOCKING_FLAGS = (
    "contains_real_paths",
    "contains_host_identity",
    "contains_customer_or_client_data",
    "contains_secret_material",
    "contains_backup_receipts",
    "contains_raw_tool_state",
    "contains_live_operational_state",
)

PRIVATE_PATH_HINTS = (
    ".northroot/state/",
    ".northroot/backup.local.toml",
    ".northroot/machine-node.json",
    ".northroot/project-nodes.json",
    ".northroot/durability-registry.json",
    ".cursor/",
    ".codex/",
    "backup-receipts/",
    "offload-result.json",
    "machine-inventory.json",
    "run-result.json",
)

PUBLIC_SAFE_SUFFIXES = (
    ".py",
    ".md",
    ".schema.json",
    ".redacted.json",
    ".example.json",
)


@dataclass(frozen=True)
class ArtifactClassification:
    path: str
    artifact_kind: str
    backup_mode: BackupMode
    public_safe_candidate: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def as_dict(self) -> dict[str, object]:
        payload = asdict(self)
        payload["backup_mode"] = self.backup_mode.value
        return payload


@dataclass(frozen=True)
class PublicCommitDecision:
    allowed: bool
    path: str
    visibility: str
    reasons: tuple[str, ...]

    def as_dict(self) -> dict[str, object]:
        return asdict(self)


def mode_policy(mode: str | BackupMode) -> ModePolicy:
    return MODE_POLICIES[BackupMode(mode)]


def all_mode_policies() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for policy in MODE_POLICIES.values():
        item = asdict(policy)
        item["mode"] = policy.mode.value
        rows.append(item)
    return rows


def classify_artifact(path: str | Path, artifact_kind: str = "unknown") -> ArtifactClassification:
    text = str(path)
    normalized = text.replace("\\", "/")
    lower_kind = artifact_kind.strip().lower()
    reasons: list[str] = []

    if any(hint in normalized for hint in PRIVATE_PATH_HINTS):
        reasons.append("path-matches-private-state-hint")
    if normalized.endswith(PUBLIC_SAFE_SUFFIXES) and ".northroot/state/" not in normalized:
        public_candidate = True
    else:
        public_candidate = False
        reasons.append("suffix-is-not-public-safe-by-default")

    if lower_kind in {"source", "schema", "docs", "redacted-example", "validator"}:
        mode = BackupMode.CATALOG_ONLY
    elif lower_kind in {"restic", "encrypted-backup", "disaster-recovery"}:
        mode = BackupMode.ENCRYPTED_DR
        public_candidate = False
    elif lower_kind in {"offload", "payload", "full-copy"}:
        mode = BackupMode.VERIFIED_OFFLOAD
        public_candidate = False
    elif lower_kind in {"knowledge", "cursor-extract", "codex-extract"}:
        mode = BackupMode.KNOWLEDGE_EXTRACT
        public_candidate = False
    elif lower_kind in {"prune", "cleanup"}:
        mode = BackupMode.PRUNE_AFTER_RESTORE_PROOF
        public_candidate = False
    else:
        mode = BackupMode.CATALOG_ONLY

    if reasons and lower_kind != "redacted-example":
        public_candidate = False

    return ArtifactClassification(
        path=text,
        artifact_kind=artifact_kind,
        backup_mode=mode,
        public_safe_candidate=public_candidate,
        reasons=tuple(reasons),
    )


def public_commit_decision(
    path: str | Path,
    *,
    artifact_kind: str = "unknown",
    is_redacted_example: bool = False,
    contains_real_paths: bool = False,
    contains_host_identity: bool = False,
    contains_customer_or_client_data: bool = False,
    contains_secret_material: bool = False,
    contains_backup_receipts: bool = False,
    contains_raw_tool_state: bool = False,
    contains_live_operational_state: bool = False,
) -> PublicCommitDecision:
    flags = {
        "contains_real_paths": contains_real_paths,
        "contains_host_identity": contains_host_identity,
        "contains_customer_or_client_data": contains_customer_or_client_data,
        "contains_secret_material": contains_secret_material,
        "contains_backup_receipts": contains_backup_receipts,
        "contains_raw_tool_state": contains_raw_tool_state,
        "contains_live_operational_state": contains_live_operational_state,
    }
    classification = classify_artifact(path, artifact_kind)
    reasons: list[str] = list(classification.reasons)
    for name in PUBLIC_BLOCKING_FLAGS:
        if flags[name]:
            reasons.append(name)

    if is_redacted_example:
        reasons = [reason for reason in reasons if reason not in {"contains_real_paths", "contains_host_identity"}]

    policy = mode_policy(classification.backup_mode)
    allowed = policy.public_commit_allowed and classification.public_safe_candidate and not reasons
    visibility = "public" if allowed else "private-or-internal"
    return PublicCommitDecision(allowed, str(path), visibility, tuple(reasons))


def public_commit_report(items: Iterable[dict[str, object]]) -> list[dict[str, object]]:
    decisions: list[dict[str, object]] = []
    for item in items:
        decision = public_commit_decision(
            str(item.get("path") or ""),
            artifact_kind=str(item.get("artifact_kind") or "unknown"),
            is_redacted_example=bool(item.get("is_redacted_example", False)),
            contains_real_paths=bool(item.get("contains_real_paths", False)),
            contains_host_identity=bool(item.get("contains_host_identity", False)),
            contains_customer_or_client_data=bool(item.get("contains_customer_or_client_data", False)),
            contains_secret_material=bool(item.get("contains_secret_material", False)),
            contains_backup_receipts=bool(item.get("contains_backup_receipts", False)),
            contains_raw_tool_state=bool(item.get("contains_raw_tool_state", False)),
            contains_live_operational_state=bool(item.get("contains_live_operational_state", False)),
        )
        decisions.append(decision.as_dict())
    return decisions
