"""Naming rules for Northroot durability roots."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RootName:
    name: str
    purpose: str
    visibility: str
    contains_payloads: bool


ROOTS: tuple[RootName, ...] = (
    RootName(
        "northroot-durability",
        "public-safe reusable policy, validators, redacted examples, and dry-run tooling",
        "public-safe",
        False,
    ),
    RootName(
        "northroot-dr-restic",
        "encrypted disaster-recovery repository or restic-specific configuration root",
        "private-or-encrypted",
        True,
    ),
    RootName(
        "northroot-machine-custody",
        "private machine identity, local grants, backup inputs, and inventories",
        "private",
        False,
    ),
    RootName(
        "northroot-offload-vault",
        "external-drive payload bundles moved off the machine after verification",
        "private-or-encrypted",
        True,
    ),
    RootName(
        "northroot-knowledge-archive",
        "reviewed knowledge extracted from old tool state; raw source remains private",
        "private-by-default",
        True,
    ),
    RootName(
        "northroot-receipts",
        "copy, backup, hash, restore, and review proofs",
        "private-unless-redacted",
        False,
    ),
)

LEGACY_NAME_MAP: dict[str, str] = {
    "northroot-restic": "northroot-dr-restic",
    "restic": "northroot-dr-restic",
    "northroot-machine-node": "northroot-machine-custody",
    "machine-node": "northroot-machine-custody",
    "offloads": "northroot-offload-vault",
    "offload": "northroot-offload-vault",
    "backup-receipts": "northroot-receipts",
    "receipts": "northroot-receipts",
}


def canonical_roots() -> list[dict[str, object]]:
    return [root.__dict__.copy() for root in ROOTS]


def normalize_root_name(name: str) -> str:
    """Return the canonical durability root name for a legacy or fuzzy label."""

    key = name.strip().lower().replace("_", "-")
    return LEGACY_NAME_MAP.get(key, key)


def root_purpose(name: str) -> RootName | None:
    canonical = normalize_root_name(name)
    for root in ROOTS:
        if root.name == canonical:
            return root
    return None
