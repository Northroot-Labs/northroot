"""Sanitize legacy machine-durability state into steward import bundles."""

from __future__ import annotations

import hashlib
import json
import plistlib
import re
from pathlib import Path
from typing import Any

from . import model


_SLUG_PATTERN = re.compile(r"[^A-Za-z0-9_.:-]+")


def _load_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return payload


def _load_plist(path: Path) -> dict[str, Any]:
    payload = plistlib.loads(path.read_bytes())
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a plist dictionary")
    return payload


def _slug(value: object, *, fallback: str) -> str:
    slug = _SLUG_PATTERN.sub("-", str(value or "").strip()).strip("-._:")
    return slug or fallback


def _fingerprint(*parts: object) -> str:
    digest = hashlib.sha256()
    for part in parts:
        digest.update(str(part or "").encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()[:12]


def _default_import_id(machine_node: dict[str, Any], project_nodes: dict[str, Any]) -> str:
    return "legacy/machine-durability-" + _fingerprint(
        machine_node.get("machine_node_id"),
        project_nodes.get("registry_id"),
        project_nodes.get("machine_node_id"),
    )


def _run_interval_seconds(launch_agent: dict[str, Any], machine_node: dict[str, Any]) -> int | None:
    start_interval = launch_agent.get("StartInterval")
    if isinstance(start_interval, int) and start_interval > 0:
        return start_interval
    backup_policy = machine_node.get("backup_policy")
    if isinstance(backup_policy, dict):
        interval = backup_policy.get("run_interval_seconds")
        if isinstance(interval, int) and interval > 0:
            return interval
    return None


def _project_node_objects(project_nodes: dict[str, Any]) -> list[dict[str, Any]]:
    nodes = project_nodes.get("nodes")
    if not isinstance(nodes, list) or not nodes:
        return [
            {
                "object_id": "repo/legacy-project-1",
                "object_type": "repo",
                "visibility": "private",
                "storage_binding": "workspace://legacy-project-1",
                "custody_policy": {
                    "backup": "metadata-only",
                    "retention": "git-remote-plus-steward-summary",
                },
                "redaction_policy": {"public_summary": "object-id-and-status"},
                "restore_class": "metadata-only",
                "legacy_source_ref": "project://legacy-project-nodes/node-1",
            }
        ]

    objects: list[dict[str, Any]] = []
    for index, node in enumerate(nodes, start=1):
        object_id = f"repo/legacy-project-{index}"
        objects.append(
            {
                "object_id": object_id,
                "object_type": "repo",
                "visibility": "private",
                "storage_binding": f"workspace://legacy-project-{index}",
                "custody_policy": {
                    "backup": "metadata-only",
                    "retention": "git-remote-plus-steward-summary",
                },
                "redaction_policy": {"public_summary": "object-id-and-status"},
                "restore_class": "metadata-only",
                "legacy_source_ref": f"project://legacy-project-nodes/node-{index}",
                "legacy_kind": str(node.get("kind") or "unknown") if isinstance(node, dict) else "unknown",
            }
        )
    return objects


def draft_legacy_profile_import(
    *,
    launch_agent_path: Path,
    machine_node_path: Path,
    project_nodes_path: Path,
    runner_state_path: Path,
    run_state_dir: Path,
    import_id: str | None = None,
    public_safe: bool = True,
) -> dict[str, Any]:
    """Build a public-safe legacy profile import bundle from raw local inputs."""

    launch_agent = _load_plist(launch_agent_path)
    machine_node = _load_json(machine_node_path)
    project_nodes = _load_json(project_nodes_path)
    runner_state = _load_json(runner_state_path)

    resolved_import_id = import_id or _default_import_id(machine_node, project_nodes)
    import_slug = _slug(resolved_import_id.rsplit("/", 1)[-1], fallback="machine-durability")
    project_id = f"project/{import_slug}"
    permission_id = f"perm/{import_slug}"
    state_object_id = f"state/{import_slug}-runner"
    secret_object_id = f"secrets/{import_slug}-runtime-env"
    source_id = f"source/{import_slug}-primary"
    primary_destination_id = f"dest/{import_slug}-primary"
    replica_destination_id = f"dest/{import_slug}-replica"
    receipts_destination_id = f"dest/{import_slug}-receipts"
    runner_status = str(runner_state.get("status") or "unknown")
    run_result_count = len(list(run_state_dir.glob("*/run-result.json"))) if run_state_dir.exists() else 0
    project_objects = _project_node_objects(project_nodes)
    object_ids = [str(item["object_id"]) for item in project_objects]
    object_ids.extend([state_object_id, secret_object_id])

    payload: dict[str, Any] = {
        "schema_version": model.LEGACY_PROFILE_IMPORT_SCHEMA,
        "import_id": resolved_import_id,
        "source": "legacy-machine-durability",
        "import_mode": "sanitized-run-summaries",
        "objects": [
            *project_objects,
            {
                "object_id": state_object_id,
                "object_type": "generated-state",
                "visibility": "private",
                "storage_binding": f"artifact://{import_slug}-machine-durability-state",
                "custody_policy": {
                    "backup": "sanitized-run-summaries",
                    "verification": "binding-present",
                },
                "redaction_policy": {"public_summary": "object-id-and-status"},
                "restore_class": "metadata-only",
            },
            {
                "object_id": secret_object_id,
                "object_type": "env-file",
                "visibility": "secret",
                "storage_binding": f"env://{import_slug}-runtime",
                "custody_policy": {
                    "backup": "provider-rehydrated",
                    "verification": "binding-present",
                },
                "redaction_policy": {"public_summary": "presence-only"},
                "restore_class": "rehydrate-from-provider",
            },
        ],
        "permissions": [
            {
                "permission_set_id": permission_id,
                "scope": "project",
                "project_id": project_id,
                "allowed_operations": [
                    "status",
                    "preflight",
                    "verify-state",
                    "report",
                    "run",
                    "verify",
                    "restore-drill",
                    "retention.evaluate",
                    "evidence.report",
                    "offsite.report",
                    "legacy.import",
                    "source.bind",
                    "replica.sync",
                ],
                "requires_human_clearance": [
                    "restore",
                    "schedule.install",
                    "schedule.uninstall",
                    "schedule.delete",
                ],
            },
            {
                "permission_set_id": f"perm/{import_slug}-secrets",
                "scope": "object",
                "object_id": secret_object_id,
                "allowed_operations": ["preflight", "verify-state"],
                "blocked_operations": ["run", "restore", "restore-drill", "evidence.record"],
            },
        ],
        "projects": [
            {
                "project_id": project_id,
                "workspace_id": f"{import_slug}-workspace",
                "node_ref": f"node://{import_slug}-machine",
                "permission_set_ref": permission_id,
                "object_ids": object_ids,
                "source_destination_ids": [source_id],
                "schedule_ref": f"scheduler://{import_slug}-machine-durability",
            }
        ],
        "destinations": [
            {
                "destination_id": primary_destination_id,
                "role": "primary",
                "adapter": "resticprofile",
                "storage_binding": f"repository://{import_slug}-primary",
                "visibility": "private",
            },
            {
                "destination_id": replica_destination_id,
                "role": "replica",
                "adapter": "external-delegated",
                "storage_binding": f"repository://{import_slug}-replica",
                "visibility": "private",
            },
            {
                "destination_id": receipts_destination_id,
                "role": "receipt-log",
                "adapter": "filesystem",
                "storage_binding": f"receipt://{import_slug}-backup-receipts",
                "visibility": "private",
            },
        ],
        "source_destinations": [
            {
                "source_destination_id": source_id,
                "project_id": project_id,
                "destination_id": primary_destination_id,
                "permission_set_ref": permission_id,
                "object_ids": [item for item in object_ids if item != secret_object_id],
                "consistency_boundary_ids": ["journal-seal"],
            }
        ],
        "replicas": [
            {
                "replica_id": f"replica/{import_slug}-offsite",
                "source_destination_id": source_id,
                "destination_id": replica_destination_id,
                "execution_model": "external-delegated",
                "required_evidence": ["verified_offsite_copy"],
                "resume_policy_ref": "resume/fail-closed-v0",
            }
        ],
        "legacy_imports": [
            {
                "import_id": resolved_import_id,
                "source": "legacy-machine-durability",
                "scheduler_ref": f"scheduler://{import_slug}-machine-durability",
                "machine_node_ref": f"node://{import_slug}-machine",
                "project_nodes_ref": f"project://{import_slug}-project-nodes",
                "runner_state_ref": f"run-state://{import_slug}-machine-durability",
                "per_run_state_ref": f"state://{import_slug}-machine-durability/runs",
                "import_mode": "sanitized-run-summaries",
                "status": "pending",
                "observed": {
                    "launch_agent_keys": sorted(str(key) for key in launch_agent.keys()),
                    "project_node_count": len(project_objects),
                    "run_interval_seconds": _run_interval_seconds(launch_agent, machine_node),
                    "run_result_count": run_result_count,
                    "runner_status": runner_status,
                },
            }
        ],
    }
    _raise_if_invalid(payload, public_safe=public_safe)
    return payload


def draft_legacy_run_import(
    *,
    run_state_dir: Path,
    import_id: str,
    legacy_import_ref: str | None = None,
    public_safe: bool = True,
) -> dict[str, Any]:
    """Build sanitized run summaries from legacy per-run result directories."""

    import_slug = _slug(import_id.rsplit("/", 1)[-1], fallback="machine-durability")
    run_summaries: list[dict[str, Any]] = []
    for index, result_path in enumerate(sorted(run_state_dir.glob("*/run-result.json")), start=1):
        result = _load_json(result_path)
        legacy_run_id = _slug(result.get("run_id") or result_path.parent.name, fallback=f"run-{index}")
        snapshot_id = _slug(result.get("backup_snapshot_id") or f"legacy-snapshot-{index}", fallback=f"legacy-snapshot-{index}")
        legacy_status = _slug(result.get("status") or "unknown", fallback="unknown")
        run_summaries.append(
            {
                "schema_version": model.RUN_SUMMARY_SCHEMA,
                "run_id": f"legacy-{legacy_run_id}",
                "workspace_id": f"{import_slug}-workspace",
                "status": "legacy-run-imported",
                "snapshot_result": {
                    "import_id": import_id,
                    "operation": "legacy-run-import",
                    "snapshot_id": snapshot_id,
                    "legacy_status": legacy_status,
                    "run_result_ref": f"state://{import_slug}-machine-durability/runs/run-{index}",
                    "imported_result_fields": sorted(str(key) for key in result.keys()),
                },
                "verification_result": {
                    "schema_version": model.VERIFICATION_RESULT_SCHEMA,
                    "repository_check": "not-run",
                    "restore_verified": False,
                    "restore_observation": None,
                    "external_evidence": [],
                },
                "tool_invocations": [
                    {
                        "tool": "legacy-machine-durability",
                        "operation": "legacy-run-import",
                        "executed": False,
                        "run_result_ref": f"state://{import_slug}-machine-durability/runs/run-{index}",
                    }
                ],
            }
        )

    payload: dict[str, Any] = {
        "schema_version": model.LEGACY_RUN_IMPORT_SCHEMA,
        "import_id": f"{import_id}-runs",
        "source": "legacy-machine-durability",
        "import_mode": "sanitized-run-summaries",
        "legacy_import_ref": legacy_import_ref or f"state://legacy-import/{import_slug}",
        "runner_state_ref": f"run-state://{import_slug}-machine-durability",
        "per_run_state_ref": f"state://{import_slug}-machine-durability/runs",
        "run_summaries": run_summaries,
    }
    _raise_if_invalid(payload, public_safe=public_safe)
    return payload


def _raise_if_invalid(payload: dict[str, Any], *, public_safe: bool) -> None:
    findings = model.validate_document(payload, public_safe=public_safe)
    errors = [finding for finding in findings if finding.severity == "error"]
    if errors:
        detail = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"invalid sanitized legacy import draft: {detail}")
