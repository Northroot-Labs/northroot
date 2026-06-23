"""Durable service-registry state helpers for the Northroot steward."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Callable

from . import model


REGISTRY_FILENAME = "service-registry.json"
LOCK_FILENAME = "service-registry.lock.json"
OPERATION_DIRNAME = "registry-operations"
OPERATION_INDEX_FILENAME = "index.json"
OPERATION_INDEX_SCHEMA = "northroot.steward.registry-operation-index.v0"


class RegistryLockedError(RuntimeError):
    """Raised when a registry mutation finds an unresolved operation lock."""

    def __init__(self, lock: dict[str, Any]) -> None:
        super().__init__("service registry has an unresolved operation lock")
        self.lock = lock


def _utc_stamp() -> str:
    return dt.datetime.now(tz=dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")


def _operation_id(operation: str) -> str:
    safe_operation = operation.replace(".", "-").replace("/", "-")
    return f"{_utc_stamp()}-{safe_operation}"


def registry_path(state_dir: Path) -> Path:
    return state_dir / REGISTRY_FILENAME


def lock_path(state_dir: Path) -> Path:
    return state_dir / LOCK_FILENAME


def operation_dir(state_dir: Path) -> Path:
    return state_dir / OPERATION_DIRNAME


def operation_index_path(state_dir: Path) -> Path:
    return operation_dir(state_dir) / OPERATION_INDEX_FILENAME


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _file_sha256(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _json_bytes(payload: dict[str, Any]) -> bytes:
    return (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _fsync_directory(path: Path) -> None:
    if not path.exists():
        return
    flags = getattr(os, "O_DIRECTORY", 0) | os.O_RDONLY
    try:
        fd = os.open(path, flags)
    except OSError:
        return
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def _atomic_write_json(path: Path, payload: dict[str, Any]) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = _json_bytes(payload)
    temp_path = path.with_name(f".{path.name}.{os.getpid()}.{_utc_stamp()}.tmp")
    with temp_path.open("wb") as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temp_path, path)
    _fsync_directory(path.parent)
    return _sha256_bytes(data)


def _read_json(path: Path) -> dict[str, Any]:
    return model.load_json(path)


def _validate_or_raise(registry: dict[str, Any], *, public_safe: bool) -> None:
    findings = model.validate_service_registry(registry, public_safe=public_safe)
    errors = [finding for finding in findings if finding.severity == "error"]
    if errors:
        detail = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"invalid service registry: {detail}")


def _load_lock(state_dir: Path) -> dict[str, Any] | None:
    path = lock_path(state_dir)
    if not path.is_file():
        return None
    return _read_json(path)


def _write_operation_summary(state_dir: Path, summary: dict[str, Any]) -> Path:
    out_dir = operation_dir(state_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"{summary['operation_id']}.json"
    digest = _atomic_write_json(path, summary)
    _index_operation_summary(state_dir, path, operation_id=str(summary["operation_id"]), sha256=digest)
    return path


def _operation_summary_paths(state_dir: Path) -> list[Path]:
    out_dir = operation_dir(state_dir)
    if not out_dir.is_dir():
        return []
    return sorted(path for path in out_dir.glob("*.json") if path.name != OPERATION_INDEX_FILENAME)


def _load_operation_index(state_dir: Path) -> dict[str, Any]:
    path = operation_index_path(state_dir)
    if not path.exists():
        return {
            "schema_version": OPERATION_INDEX_SCHEMA,
            "updated_at": None,
            "entries": [],
        }
    index = _read_json(path)
    if index.get("schema_version") != OPERATION_INDEX_SCHEMA:
        raise ValueError(f"registry operation index schema_version must be {OPERATION_INDEX_SCHEMA}")
    if not isinstance(index.get("entries"), list):
        raise ValueError("registry operation index entries must be a list")
    return index


def _write_operation_index(state_dir: Path, entries: list[dict[str, Any]]) -> None:
    payload = {
        "schema_version": OPERATION_INDEX_SCHEMA,
        "updated_at": _utc_stamp(),
        "entries": sorted(entries, key=lambda entry: str(entry.get("operation_id", ""))),
    }
    _atomic_write_json(operation_index_path(state_dir), payload)


def _index_operation_summary(state_dir: Path, summary_path: Path, *, operation_id: str, sha256: str | None = None) -> None:
    index = _load_operation_index(state_dir)
    entries = [
        entry
        for entry in index.get("entries", [])
        if isinstance(entry, dict)
        and entry.get("path") != summary_path.name
        and entry.get("operation_id") != operation_id
    ]
    entries.append(
        {
            "operation_id": operation_id,
            "path": summary_path.name,
            "sha256": sha256 or _file_sha256(summary_path),
            "indexed_at": _utc_stamp(),
        }
    )
    _write_operation_index(state_dir, entries)


def load_registry(state_dir: Path) -> dict[str, Any]:
    return _read_json(registry_path(state_dir))


def _check(name: str, ok: bool, detail: str, *, code: str | None = None) -> dict[str, Any]:
    return {
        "name": name,
        "status": "ok" if ok else "error",
        "code": None if ok else code,
        "detail": detail,
    }


def registry_operation_log_integrity(state_dir: Path) -> dict[str, Any]:
    summaries = _operation_summary_paths(state_dir)
    index_path = operation_index_path(state_dir)
    observations: list[dict[str, Any]] = []
    if not summaries and not index_path.exists():
        return {
            "schema_version": "northroot.steward.registry-operation-log-integrity.v0",
            "ok": True,
            "index_path": str(index_path),
            "observations": [],
        }
    try:
        index = _load_operation_index(state_dir)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.registry-operation-log-integrity.v0",
            "ok": False,
            "index_path": str(index_path),
            "observations": [
                {
                    "summary_path": str(index_path),
                    "status": "invalid-index",
                    "detail": str(err),
                }
            ],
        }

    entries_by_path: dict[str, dict[str, Any]] = {}
    duplicate_paths: set[str] = set()
    for entry in index.get("entries", []):
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            observations.append(
                {
                    "summary_path": str(index_path),
                    "status": "invalid-index-entry",
                    "detail": "registry operation index entries must include path strings",
                }
            )
            continue
        path_name = str(entry["path"])
        if path_name in entries_by_path:
            duplicate_paths.add(path_name)
        entries_by_path[path_name] = entry

    summary_names = {path.name for path in summaries}
    for path_name, entry in sorted(entries_by_path.items()):
        if path_name == OPERATION_INDEX_FILENAME:
            observations.append(
                {
                    "summary_path": str(index_path),
                    "status": "invalid-index-entry",
                    "detail": "registry operation index may not index itself",
                }
            )
            continue
        if path_name not in summary_names:
            observations.append(
                {
                    "summary_path": str(operation_dir(state_dir) / path_name),
                    "operation_id": entry.get("operation_id"),
                    "status": "missing-summary",
                    "detail": "indexed registry operation summary is missing",
                }
            )

    for summary_path in summaries:
        entry = entries_by_path.get(summary_path.name)
        if entry is None:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": None,
                    "status": "unindexed",
                    "detail": "registry operation summary is not present in the steward digest index",
                }
            )
            continue
        if summary_path.name in duplicate_paths:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": entry.get("operation_id"),
                    "status": "duplicate-index-entry",
                    "detail": "registry operation summary has duplicate digest index entries",
                }
            )
            continue
        expected_sha = entry.get("sha256")
        if not isinstance(expected_sha, str) or not expected_sha:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": entry.get("operation_id"),
                    "status": "invalid-index-entry",
                    "detail": "registry operation index entry is missing sha256",
                }
            )
            continue
        actual_sha = _file_sha256(summary_path)
        if actual_sha != expected_sha:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": entry.get("operation_id"),
                    "status": "digest-mismatch",
                    "detail": "registry operation summary digest does not match the steward digest index",
                    "expected_sha256": expected_sha,
                    "actual_sha256": actual_sha,
                }
            )
            continue
        try:
            summary = _read_json(summary_path)
        except (OSError, ValueError, json.JSONDecodeError) as err:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": entry.get("operation_id"),
                    "status": "invalid-json",
                    "detail": str(err),
                }
            )
            continue
        indexed_operation_id = entry.get("operation_id")
        summary_operation_id = summary.get("operation_id")
        if summary.get("schema_version") != "northroot.steward.registry-operation.v0":
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": summary_operation_id,
                    "status": "invalid-summary",
                    "detail": "unexpected registry operation summary schema",
                }
            )
        elif indexed_operation_id != summary_operation_id:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": summary_operation_id,
                    "status": "operation-id-mismatch",
                    "detail": "registry operation summary operation_id does not match the steward digest index",
                    "indexed_operation_id": indexed_operation_id,
                }
            )
        else:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "operation_id": summary_operation_id,
                    "status": "ok",
                    "sha256": actual_sha,
                }
            )

    ok = all(observation.get("status") == "ok" for observation in observations)
    return {
        "schema_version": "northroot.steward.registry-operation-log-integrity.v0",
        "ok": ok,
        "index_path": str(index_path),
        "observations": observations,
    }


def registry_integrity_report(state_dir: Path, *, public_safe: bool = False) -> dict[str, Any]:
    path = registry_path(state_dir)
    lock = _load_lock(state_dir)
    operation_log_integrity = registry_operation_log_integrity(state_dir)
    registry: dict[str, Any] | None = None
    registry_error: str | None = None
    findings: list[dict[str, str]] = []
    if path.is_file():
        try:
            registry = load_registry(state_dir)
            findings = [finding.as_dict() for finding in model.validate_service_registry(registry, public_safe=public_safe)]
        except Exception as exc:  # noqa: BLE001 - integrity reports should return structured failure context
            registry_error = str(exc)

    summaries: list[dict[str, Any]] = []
    invalid_summaries: list[dict[str, str]] = []
    for summary_path in _operation_summary_paths(state_dir):
        try:
            summary = _read_json(summary_path)
        except Exception as exc:  # noqa: BLE001 - continue reporting all readable and unreadable summaries
            invalid_summaries.append({"path": str(summary_path), "error": str(exc)})
            continue
        if summary.get("schema_version") != "northroot.steward.registry-operation.v0":
            invalid_summaries.append({"path": str(summary_path), "error": "unexpected registry operation schema"})
            continue
        summary = {**summary, "summary_path": str(summary_path)}
        summaries.append(summary)

    proving_statuses = {"completed", "recovered-after-interruption"}
    completed = [
        summary
        for summary in summaries
        if summary.get("status") in proving_statuses and isinstance(summary.get("registry_sha256"), str)
    ]
    latest = completed[-1] if completed else None
    current_sha256 = _file_sha256(path)
    expected_sha256 = str(latest["registry_sha256"]) if latest is not None else None
    error_count = len([finding for finding in findings if finding.get("severity") == "error"])
    registry_present = path.is_file()
    registry_readable = registry_present and registry_error is None and registry is not None
    registry_valid = registry_readable and error_count == 0
    operation_log_present = bool(summaries or invalid_summaries)
    operation_log_readable = not invalid_summaries and bool(operation_log_integrity["ok"])
    registry_matches_latest_operation = bool(expected_sha256 and current_sha256 == expected_sha256)
    protected_state_ok = bool(
        registry_present
        and registry_valid
        and operation_log_present
        and operation_log_readable
        and registry_matches_latest_operation
    )
    checks = [
        _check(
            "registry_present",
            registry_present,
            f"service registry exists at {path}" if registry_present else "service registry is missing",
            code="missing_service_registry",
        ),
        _check(
            "registry_readable",
            registry_readable,
            "service registry is readable" if registry_readable else f"service registry is unreadable: {registry_error}",
            code="unreadable_service_registry",
        ),
        _check(
            "registry_valid",
            registry_valid,
            "service registry validates"
            if registry_valid
            else f"service registry has {error_count} validation error(s)",
            code="invalid_service_registry",
        ),
        _check(
            "operation_log_present",
            operation_log_present,
            "registry operation log has at least one summary"
            if operation_log_present
            else "registry operation log is missing",
            code="missing_registry_operation_log",
        ),
        _check(
            "operation_log_readable",
            operation_log_readable,
            "registry operation summaries are readable and match the steward digest index"
            if operation_log_readable
            else "one or more registry operation summaries are unreadable, invalid, unindexed, or digest-mismatched",
            code="invalid_registry_operation_log",
        ),
        _check(
            "registry_matches_latest_operation",
            registry_matches_latest_operation,
            "registry digest matches latest completed operation summary"
            if registry_matches_latest_operation
            else "registry digest does not match latest completed operation summary",
            code="registry_digest_mismatch",
        ),
        _check(
            "registry_unlocked",
            lock is None,
            "no unresolved registry operation lock" if lock is None else "registry has an unresolved operation lock",
            code="registry_operation_lock_present",
        ),
    ]
    return {
        "schema_version": "northroot.steward.registry-integrity.v0",
        "configured": registry_present,
        "ready": protected_state_ok and lock is None,
        "protected_state_ok": protected_state_ok,
        "resume_required": lock is not None,
        "registry_path": str(path),
        "registry_sha256": current_sha256,
        "expected_registry_sha256": expected_sha256,
        "latest_operation_summary_path": latest.get("summary_path") if latest is not None else None,
        "operation_summary_count": len(summaries),
        "completed_operation_summary_count": len(completed),
        "failed_operation_summary_count": len([summary for summary in summaries if summary.get("status") == "failed"]),
        "invalid_operation_summaries": invalid_summaries,
        "operation_log_integrity": operation_log_integrity,
        "lock": lock,
        "finding_count": len(findings),
        "error_count": error_count,
        "findings": findings,
        "checks": checks,
    }


def registry_status(state_dir: Path, *, public_safe: bool = False) -> dict[str, Any]:
    path = registry_path(state_dir)
    lock = _load_lock(state_dir)
    if not path.is_file():
        return {
            "schema_version": "northroot.steward.registry-status.v0",
            "configured": False,
            "ready": False,
            "registry_path": str(path),
            "registry_sha256": None,
            "lock": lock,
            "resume_required": lock is not None,
            "findings": [
                {
                    "severity": "error",
                    "code": "missing_service_registry",
                    "path": str(path),
                    "detail": "service registry has not been initialized",
                }
            ],
        }
    integrity = registry_integrity_report(state_dir, public_safe=public_safe)
    try:
        registry = load_registry(state_dir)
    except Exception as exc:  # noqa: BLE001 - status must fail closed as data, not as an exception
        finding = {
            "severity": "error",
            "code": "unreadable_service_registry",
            "path": str(path),
            "detail": str(exc),
        }
        return {
            "schema_version": "northroot.steward.registry-status.v0",
            "configured": True,
            "ready": False,
            "registry_path": str(path),
            "registry_sha256": _file_sha256(path),
            "expected_registry_sha256": integrity["expected_registry_sha256"],
            "protected_state_ok": False,
            "integrity": integrity,
            "lock": lock,
            "resume_required": lock is not None,
            "finding_count": 1,
            "error_count": 1,
            "service_id": None,
            "node_id": None,
            "project_count": 0,
            "object_count": 0,
            "permission_count": 0,
            "destination_count": 0,
            "source_destination_count": 0,
            "replica_count": 0,
            "legacy_import_count": 0,
            "findings": [finding],
        }
    findings = model.validate_service_registry(registry, public_safe=public_safe)
    error_count = len([finding for finding in findings if finding.severity == "error"])
    return {
        "schema_version": "northroot.steward.registry-status.v0",
        "configured": True,
        "ready": error_count == 0 and integrity["ready"],
        "registry_path": str(path),
        "registry_sha256": _file_sha256(path),
        "expected_registry_sha256": integrity["expected_registry_sha256"],
        "protected_state_ok": integrity["protected_state_ok"],
        "integrity": integrity,
        "lock": lock,
        "resume_required": lock is not None,
        "finding_count": len(findings),
        "error_count": error_count,
        "service_id": registry.get("service_id"),
        "node_id": registry.get("node_id"),
        "project_count": len(registry.get("projects", [])) if isinstance(registry.get("projects"), list) else 0,
        "object_count": len(registry.get("objects", [])) if isinstance(registry.get("objects"), list) else 0,
        "permission_count": len(registry.get("permissions", [])) if isinstance(registry.get("permissions"), list) else 0,
        "destination_count": len(registry.get("destinations", [])) if isinstance(registry.get("destinations"), list) else 0,
        "source_destination_count": len(registry.get("source_destinations", []))
        if isinstance(registry.get("source_destinations"), list)
        else 0,
        "replica_count": len(registry.get("replicas", [])) if isinstance(registry.get("replicas"), list) else 0,
        "legacy_import_count": len(registry.get("legacy_imports", []))
        if isinstance(registry.get("legacy_imports"), list)
        else 0,
        "findings": [finding.as_dict() for finding in findings],
    }


def _items_by_id(items: Any, key: str) -> dict[str, dict[str, Any]]:
    if not isinstance(items, list):
        return {}
    return {str(item[key]): item for item in items if isinstance(item, dict) and isinstance(item.get(key), str)}


def registry_topology_report(
    state_dir: Path,
    *,
    project_id: str | None = None,
    public_safe: bool = False,
) -> dict[str, Any]:
    status = registry_status(state_dir, public_safe=public_safe)
    base = {
        "schema_version": "northroot.steward.registry-topology.v0",
        "registry_path": status["registry_path"],
        "registry_sha256": status["registry_sha256"],
        "registry_ready": bool(status["ready"]),
        "protected_state_ok": bool(status.get("protected_state_ok")),
        "resume_required": bool(status.get("resume_required")),
        "project_id": project_id,
    }
    if not status["ready"]:
        return {
            **base,
            "ready": False,
            "decision": "registry-not-ready",
            "reason": "registry integrity or validation must be repaired before topology can be trusted",
            "findings": status.get("findings", []),
            "projects": [],
            "resume_policy": None,
        }

    service_registry = load_registry(state_dir)
    projects = service_registry.get("projects", [])
    if not isinstance(projects, list):
        projects = []
    if project_id is not None:
        projects = [project for project in projects if isinstance(project, dict) and project.get("project_id") == project_id]
        if not projects:
            return {
                **base,
                "ready": False,
                "decision": "unknown-project",
                "reason": f"unknown project_id: {project_id}",
                "projects": [],
                "resume_policy": service_registry.get("resume_policy"),
            }

    objects_by_id = _items_by_id(service_registry.get("objects"), "object_id")
    destinations_by_id = _items_by_id(service_registry.get("destinations"), "destination_id")
    source_destinations_by_id = _items_by_id(service_registry.get("source_destinations"), "source_destination_id")
    replicas = [replica for replica in service_registry.get("replicas", []) if isinstance(replica, dict)]
    permissions_by_id = _items_by_id(service_registry.get("permissions"), "permission_set_id")
    resume_policy = service_registry.get("resume_policy") if isinstance(service_registry.get("resume_policy"), dict) else {}
    partial_run_handling = resume_policy.get("partial_run_handling")
    fail_closed_storage = resume_policy.get("on_disconnected_storage") == "fail-closed-record-summary"
    never_prune_without_decision = partial_run_handling == "never-prune-without-retention-decision"

    project_reports: list[dict[str, Any]] = []
    issue_count = 0
    for project in projects:
        if not isinstance(project, dict):
            continue
        project_id_value = str(project.get("project_id"))
        project_object_ids = {
            str(item)
            for item in project.get("object_ids", [])
            if isinstance(item, str)
        }
        project_source_ids = [str(item) for item in project.get("source_destination_ids", []) if isinstance(item, str)]
        source_reports: list[dict[str, Any]] = []
        for source_id in project_source_ids:
            source = source_destinations_by_id.get(source_id)
            destination = destinations_by_id.get(str(source.get("destination_id"))) if source else None
            object_ids = [str(item) for item in source.get("object_ids", [])] if source else []
            source_project_matches = bool(source and source.get("project_id") == project_id_value)
            source_objects_in_project = bool(source) and all(object_id in project_object_ids for object_id in object_ids)
            source_permission_matches = bool(source and source.get("permission_set_ref") == project.get("permission_set_ref"))
            source_destination_role_ok = bool(
                destination and destination.get("role") in {"primary", "source"}
            )
            source_replica_reports: list[dict[str, Any]] = []
            for replica in replicas:
                if replica.get("source_destination_id") != source_id:
                    continue
                replica_destination = destinations_by_id.get(str(replica.get("destination_id")))
                required_evidence = [
                    str(item) for item in replica.get("required_evidence", []) if isinstance(item, str)
                ]
                replica_ready = bool(
                    replica_destination
                    and replica_destination.get("role") == "replica"
                    and "verified_offsite_copy" in required_evidence
                    and fail_closed_storage
                    and never_prune_without_decision
                )
                if not replica_ready:
                    issue_count += 1
                source_replica_reports.append(
                    {
                        "replica_id": replica.get("replica_id"),
                        "destination_id": replica.get("destination_id"),
                        "destination": replica_destination,
                        "execution_model": replica.get("execution_model"),
                        "required_evidence": required_evidence,
                        "resume_policy_ref": replica.get("resume_policy_ref"),
                        "ready": replica_ready,
                        "readiness": {
                            "destination_registered": replica_destination is not None,
                            "destination_role": replica_destination.get("role") if replica_destination else None,
                            "destination_role_is_replica": bool(
                                replica_destination and replica_destination.get("role") == "replica"
                            ),
                            "required_evidence_declared": bool(required_evidence),
                            "verified_offsite_copy_required": "verified_offsite_copy" in required_evidence,
                            "storage_failure_policy": resume_policy.get("on_disconnected_storage"),
                            "partial_run_handling": partial_run_handling,
                        },
                    }
                )
            source_ready = bool(
                source
                and destination
                and object_ids
                and source_project_matches
                and source_objects_in_project
                and source_permission_matches
                and source_destination_role_ok
            )
            if not source_ready:
                issue_count += 1
            source_reports.append(
                {
                    "source_destination_id": source_id,
                    "project_id": source.get("project_id") if source else None,
                    "destination_id": source.get("destination_id") if source else None,
                    "destination": destination,
                    "permission_set_ref": source.get("permission_set_ref") if source else None,
                    "objects": [
                        {
                            "object_id": object_id,
                            "object_type": objects_by_id.get(object_id, {}).get("object_type"),
                            "visibility": objects_by_id.get(object_id, {}).get("visibility"),
                            "restore_class": objects_by_id.get(object_id, {}).get("restore_class"),
                        }
                        for object_id in object_ids
                    ],
                    "consistency_boundary_ids": source.get("consistency_boundary_ids", []) if source else [],
                    "replicas": source_replica_reports,
                    "ready": source_ready,
                    "readiness": {
                        "source_registered": source is not None,
                        "source_project_matches": source_project_matches,
                        "destination_registered": destination is not None,
                        "destination_role": destination.get("role") if destination else None,
                        "destination_role_is_source_compatible": source_destination_role_ok,
                        "permission_set_matches_project": source_permission_matches,
                        "object_count": len(object_ids),
                        "objects_in_project": source_objects_in_project,
                    },
                }
            )
        project_permission = permissions_by_id.get(str(project.get("permission_set_ref")))
        project_ready = bool(source_reports) and all(bool(source["ready"]) for source in source_reports)
        if not project_ready:
            issue_count += 1
        project_reports.append(
            {
                "project_id": project.get("project_id"),
                "workspace_id": project.get("workspace_id"),
                "node_ref": project.get("node_ref"),
                "permission_set_ref": project.get("permission_set_ref"),
                "permission_set": project_permission,
                "object_ids": project.get("object_ids", []),
                "schedule_ref": project.get("schedule_ref"),
                "source_destinations": source_reports,
                "ready": project_ready,
            }
        )

    topology_ready = bool(project_reports) and issue_count == 0 and fail_closed_storage and never_prune_without_decision
    return {
        **base,
        "ready": topology_ready,
        "decision": "ready" if topology_ready else "topology-incomplete",
        "reason": None
        if topology_ready
        else "project destination topology or resume policy is incomplete",
        "project_count": len(project_reports),
        "destination_count": status["destination_count"],
        "source_destination_count": status["source_destination_count"],
        "replica_count": status["replica_count"],
        "issue_count": issue_count,
        "resume_policy": resume_policy,
        "resume_policy_ready": {
            "on_disconnected_storage": resume_policy.get("on_disconnected_storage"),
            "partial_run_handling": partial_run_handling,
            "fail_closed_on_disconnected_storage": fail_closed_storage,
            "never_prune_without_retention_decision": never_prune_without_decision,
        },
        "projects": project_reports,
    }


def _project_by_id(registry: dict[str, Any], project_id: str) -> dict[str, Any] | None:
    for project in registry.get("projects", []):
        if isinstance(project, dict) and project.get("project_id") == project_id:
            return project
    return None


def _permission_by_id(registry: dict[str, Any], permission_set_id: str) -> dict[str, Any] | None:
    for permission in registry.get("permissions", []):
        if isinstance(permission, dict) and permission.get("permission_set_id") == permission_set_id:
            return permission
    return None


def _object_permissions(registry: dict[str, Any], object_id: str) -> list[dict[str, Any]]:
    return [
        permission
        for permission in registry.get("permissions", [])
        if isinstance(permission, dict) and permission.get("scope") == "object" and permission.get("object_id") == object_id
    ]


def _permission_values(permission: dict[str, Any], key: str) -> set[str]:
    values = permission.get(key, [])
    if not isinstance(values, list):
        return set()
    return {str(value) for value in values if isinstance(value, str)}


def authorize_operation(
    state_dir: Path,
    *,
    operation: str,
    project_id: str,
    object_id: str | None = None,
    public_safe: bool = False,
) -> dict[str, Any]:
    lock = _load_lock(state_dir)
    status = registry_status(state_dir, public_safe=public_safe)
    base = {
        "schema_version": "northroot.steward.authorization.v0",
        "operation": operation,
        "project_id": project_id,
        "object_id": object_id,
        "registry_path": status["registry_path"],
        "registry_sha256": status["registry_sha256"],
        "allowed": False,
        "decision": "blocked",
        "reason": None,
        "requires_human_clearance": False,
        "matched_permission_sets": [],
    }
    if operation not in model.SERVICE_PERMISSION_OPERATIONS:
        return {
            **base,
            "decision": "invalid-operation",
            "reason": f"operation is not in service permission vocabulary: {operation}",
        }
    if lock is not None:
        return {
            **base,
            "decision": "resume-required",
            "reason": "registry has an unresolved operation lock",
            "lock": lock,
        }
    if not status["ready"]:
        return {
            **base,
            "decision": "invalid-registry",
            "reason": "registry is not ready for authorization",
            "findings": status.get("findings", []),
        }

    service_registry = load_registry(state_dir)
    project = _project_by_id(service_registry, project_id)
    if project is None:
        return {**base, "decision": "unknown-project", "reason": f"unknown project_id: {project_id}"}
    if object_id is not None and object_id not in project.get("object_ids", []):
        return {
            **base,
            "decision": "unknown-project-object",
            "reason": f"object_id is not registered under project_id {project_id}: {object_id}",
        }

    permission_ref = project.get("permission_set_ref")
    project_permission = _permission_by_id(service_registry, str(permission_ref))
    if project_permission is None:
        return {
            **base,
            "decision": "missing-project-permission",
            "reason": f"project permission_set_ref is missing: {permission_ref}",
        }

    matched_permission_sets = [str(project_permission.get("permission_set_id"))]
    object_permissions = _object_permissions(service_registry, object_id) if object_id is not None else []
    matched_permission_sets.extend(str(permission["permission_set_id"]) for permission in object_permissions)

    permissions = [project_permission, *object_permissions]
    for permission in permissions:
        if operation in _permission_values(permission, "blocked_operations"):
            return {
                **base,
                "decision": "blocked",
                "reason": f"operation blocked by {permission.get('permission_set_id')}",
                "matched_permission_sets": matched_permission_sets,
            }
    for permission in permissions:
        if operation in _permission_values(permission, "requires_human_clearance"):
            return {
                **base,
                "decision": "human-clearance-required",
                "reason": f"operation requires human clearance by {permission.get('permission_set_id')}",
                "requires_human_clearance": True,
                "matched_permission_sets": matched_permission_sets,
            }

    project_allowed = operation in _permission_values(project_permission, "allowed_operations")
    if not project_allowed:
        return {
            **base,
            "decision": "not-allowed",
            "reason": f"operation is not allowed by project permission set {permission_ref}",
            "matched_permission_sets": matched_permission_sets,
        }

    if object_permissions:
        object_allowed = any(operation in _permission_values(permission, "allowed_operations") for permission in object_permissions)
        if not object_allowed:
            return {
                **base,
                "decision": "not-allowed",
                "reason": "operation is not allowed by object permission set",
                "matched_permission_sets": matched_permission_sets,
            }

    return {
        **base,
        "allowed": True,
        "decision": "allowed",
        "reason": "operation is allowed by registry permissions",
        "matched_permission_sets": matched_permission_sets,
    }


def initialize_registry(
    state_dir: Path,
    registry: dict[str, Any],
    *,
    public_safe: bool = False,
    overwrite: bool = False,
) -> dict[str, Any]:
    path = registry_path(state_dir)
    existing_lock = _load_lock(state_dir)
    if existing_lock is not None:
        raise RegistryLockedError(existing_lock)
    if path.exists() and not overwrite:
        raise FileExistsError(f"service registry already exists: {path}")
    _validate_or_raise(registry, public_safe=public_safe)
    state_dir.mkdir(parents=True, exist_ok=True)
    operation_id = _operation_id("registry.init")
    before_sha256 = _file_sha256(path)
    lock = {
        "schema_version": "northroot.steward.registry-operation-lock.v0",
        "operation_id": operation_id,
        "operation": "registry.init",
        "registry_path": str(path),
        "before_sha256": before_sha256,
        "started_at": _utc_stamp(),
        "pid": os.getpid(),
        "failure_policy": "fail-closed-record-summary",
        "resume_hint": "run registry recover before applying another initialization or mutation",
    }
    _atomic_write_json(lock_path(state_dir), lock)
    try:
        digest = _atomic_write_json(path, registry)
        summary = {
            "schema_version": "northroot.steward.registry-operation.v0",
            "operation_id": operation_id,
            "operation": "registry.init",
            "status": "completed",
            "registry_path": str(path),
            "before_sha256": before_sha256,
            "registry_sha256": digest,
            "public_safe": public_safe,
            "completed_at": _utc_stamp(),
        }
        summary_path = _write_operation_summary(state_dir, summary)
        lock_path(state_dir).unlink(missing_ok=True)
        _fsync_directory(state_dir)
        return {
            "initialized": True,
            "registry_path": str(path),
            "registry_sha256": digest,
            "operation_summary_path": str(summary_path),
        }
    except Exception as exc:
        summary = {
            "schema_version": "northroot.steward.registry-operation.v0",
            "operation_id": operation_id,
            "operation": "registry.init",
            "status": "failed",
            "registry_path": str(path),
            "before_sha256": before_sha256,
            "registry_sha256": _file_sha256(path),
            "public_safe": public_safe,
            "error": str(exc),
            "completed_at": _utc_stamp(),
        }
        _write_operation_summary(state_dir, summary)
        lock_path(state_dir).unlink(missing_ok=True)
        _fsync_directory(state_dir)
        raise


def mutate_registry(
    state_dir: Path,
    *,
    operation: str,
    mutator: Callable[[dict[str, Any]], None],
    public_safe: bool = False,
) -> dict[str, Any]:
    existing_lock = _load_lock(state_dir)
    if existing_lock is not None:
        raise RegistryLockedError(existing_lock)
    operation_id = _operation_id(operation)
    before_sha256 = _file_sha256(registry_path(state_dir))
    lock = {
        "schema_version": "northroot.steward.registry-operation-lock.v0",
        "operation_id": operation_id,
        "operation": operation,
        "started_at": _utc_stamp(),
        "pid": os.getpid(),
        "registry_path": str(registry_path(state_dir)),
        "before_sha256": before_sha256,
        "failure_policy": "fail-closed-record-summary",
        "resume_hint": "run registry recover before applying another mutation",
    }
    _atomic_write_json(lock_path(state_dir), lock)
    try:
        registry = load_registry(state_dir)
        mutator(registry)
        _validate_or_raise(registry, public_safe=public_safe)
        after_sha256 = _atomic_write_json(registry_path(state_dir), registry)
        summary = {
            "schema_version": "northroot.steward.registry-operation.v0",
            "operation_id": operation_id,
            "operation": operation,
            "status": "completed",
            "registry_path": str(registry_path(state_dir)),
            "before_sha256": before_sha256,
            "registry_sha256": after_sha256,
            "public_safe": public_safe,
            "completed_at": _utc_stamp(),
        }
        summary_path = _write_operation_summary(state_dir, summary)
        lock_path(state_dir).unlink(missing_ok=True)
        _fsync_directory(state_dir)
        return {
            "mutated": True,
            "operation": operation,
            "operation_id": operation_id,
            "registry_path": str(registry_path(state_dir)),
            "before_sha256": before_sha256,
            "registry_sha256": after_sha256,
            "operation_summary_path": str(summary_path),
        }
    except Exception as exc:
        summary = {
            "schema_version": "northroot.steward.registry-operation.v0",
            "operation_id": operation_id,
            "operation": operation,
            "status": "failed",
            "registry_path": str(registry_path(state_dir)),
            "before_sha256": before_sha256,
            "registry_sha256": _file_sha256(registry_path(state_dir)),
            "public_safe": public_safe,
            "error": str(exc),
            "completed_at": _utc_stamp(),
        }
        _write_operation_summary(state_dir, summary)
        lock_path(state_dir).unlink(missing_ok=True)
        _fsync_directory(state_dir)
        raise


def recover_registry(state_dir: Path, *, public_safe: bool = False) -> dict[str, Any]:
    lock = _load_lock(state_dir)
    if lock is None:
        return {
            "recovered": False,
            "resume_required": False,
            "detail": "no operation lock present",
        }
    registry_missing = not registry_path(state_dir).is_file()
    registry_error: str | None = None
    findings: list[model.Finding] = []
    if not registry_missing:
        try:
            registry_payload = load_registry(state_dir)
            findings = model.validate_service_registry(registry_payload, public_safe=public_safe)
        except Exception as exc:  # noqa: BLE001 - recovery must report unreadable registry state, not crash
            registry_error = str(exc)
    errors = [finding for finding in findings if finding.severity == "error"]
    operation_id = _operation_id("registry.recover")
    if registry_error is not None:
        status = "blocked-unreadable-registry"
    elif errors:
        status = "blocked-invalid-registry"
    else:
        status = "recovered-after-interruption"
    current_sha256 = _file_sha256(registry_path(state_dir))
    before_sha256 = lock.get("before_sha256")
    if registry_missing:
        resume_state = "registry-missing-after-lock"
    elif isinstance(before_sha256, str):
        resume_state = "registry-unchanged-after-lock" if current_sha256 == before_sha256 else "registry-changed-after-lock"
    else:
        resume_state = "registry-change-unknown"
    finding_payloads = [finding.as_dict() for finding in findings]
    if registry_missing:
        finding_payloads.append(
            {
                "severity": "warning",
                "code": "missing_service_registry_after_lock",
                "path": str(registry_path(state_dir)),
                "detail": "registry operation lock existed but service registry was never written",
            }
        )
    if registry_error is not None:
        finding_payloads.append(
            {
                "severity": "error",
                "code": "unreadable_service_registry",
                "path": str(registry_path(state_dir)),
                "detail": registry_error,
            }
        )
    summary = {
        "schema_version": "northroot.steward.registry-operation.v0",
        "operation_id": operation_id,
        "operation": "registry.recover",
        "status": status,
        "resume_state": resume_state,
        "interrupted_operation_lock": lock,
        "registry_path": str(registry_path(state_dir)),
        "interrupted_before_sha256": before_sha256 if isinstance(before_sha256, str) else None,
        "registry_sha256": current_sha256,
        "registry_changed_since_lock": resume_state == "registry-changed-after-lock",
        "public_safe": public_safe,
        "findings": finding_payloads,
        "completed_at": _utc_stamp(),
    }
    summary_path = _write_operation_summary(state_dir, summary)
    if registry_error is not None:
        return {
            "recovered": False,
            "resume_required": True,
            "operation_summary_path": str(summary_path),
            "resume_state": resume_state,
            "error_count": 1,
            "findings": finding_payloads,
        }
    if errors:
        return {
            "recovered": False,
            "resume_required": True,
            "operation_summary_path": str(summary_path),
            "resume_state": resume_state,
            "error_count": len(errors),
            "findings": finding_payloads,
        }
    lock_path(state_dir).unlink(missing_ok=True)
    _fsync_directory(state_dir)
    return {
        "recovered": True,
        "resume_required": False,
        "operation_summary_path": str(summary_path),
        "registry_sha256": summary["registry_sha256"],
        "resume_state": resume_state,
        "registry_changed_since_lock": summary["registry_changed_since_lock"],
    }


def _append_unique(items: list[dict[str, Any]], key: str, value: dict[str, Any]) -> None:
    identity = value.get(key)
    if any(item.get(key) == identity for item in items):
        raise ValueError(f"duplicate {key}: {identity}")
    items.append(value)


def _canonical_payload(value: dict[str, Any]) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def _append_unique_or_same(
    items: list[dict[str, Any]],
    key: str,
    value: dict[str, Any],
    *,
    counts: dict[str, int],
) -> None:
    identity = value.get(key)
    for item in items:
        if item.get(key) != identity:
            continue
        if _canonical_payload(item) == _canonical_payload(value):
            counts["skipped_existing"] += 1
            return
        raise ValueError(f"conflicting {key}: {identity}")
    items.append(value)
    counts["inserted"] += 1


def _set_unique(items: list[dict[str, Any]], key: str, value: dict[str, Any]) -> str:
    identity = value.get(key)
    for index, item in enumerate(items):
        if item.get(key) != identity:
            continue
        if _canonical_payload(item) == _canonical_payload(value):
            return "unchanged"
        items[index] = value
        return "replaced"
    items.append(value)
    return "inserted"


def _set_result(result: dict[str, Any], *, entity: str, identity: object, action: str, **extra: Any) -> dict[str, Any]:
    return {
        **result,
        "schema_version": "northroot.steward.registry-set-result.v0",
        "entity": entity,
        "identity": identity,
        "action": action,
        **extra,
    }


def add_object(state_dir: Path, custody_object: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("objects", []), "object_id", custody_object)

    return mutate_registry(state_dir, operation="registry.object.add", mutator=mutator, public_safe=public_safe)


def set_object(state_dir: Path, custody_object: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("objects", []), "object_id", custody_object)

    result = mutate_registry(state_dir, operation="registry.object.set", mutator=mutator, public_safe=public_safe)
    return _set_result(result, entity="object", identity=custody_object.get("object_id"), action=action)


def add_permission(state_dir: Path, permission: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("permissions", []), "permission_set_id", permission)

    return mutate_registry(state_dir, operation="registry.permission.add", mutator=mutator, public_safe=public_safe)


def set_permission(state_dir: Path, permission: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("permissions", []), "permission_set_id", permission)

    result = mutate_registry(
        state_dir,
        operation="registry.permission.set",
        mutator=mutator,
        public_safe=public_safe,
    )
    return _set_result(result, entity="permission", identity=permission.get("permission_set_id"), action=action)


def add_project(state_dir: Path, project: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("projects", []), "project_id", project)

    return mutate_registry(state_dir, operation="registry.project.add", mutator=mutator, public_safe=public_safe)


def set_project(state_dir: Path, project: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("projects", []), "project_id", project)

    result = mutate_registry(state_dir, operation="registry.project.set", mutator=mutator, public_safe=public_safe)
    return _set_result(result, entity="project", identity=project.get("project_id"), action=action)


def register_project(
    state_dir: Path,
    *,
    project: dict[str, Any],
    permission: dict[str, Any],
    public_safe: bool = False,
) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("permissions", []), "permission_set_id", permission)
        _append_unique(registry.setdefault("projects", []), "project_id", project)

    return mutate_registry(state_dir, operation="registry.project.register", mutator=mutator, public_safe=public_safe)


def add_destination(state_dir: Path, destination: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("destinations", []), "destination_id", destination)

    return mutate_registry(state_dir, operation="registry.destination.add", mutator=mutator, public_safe=public_safe)


def set_destination(state_dir: Path, destination: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("destinations", []), "destination_id", destination)

    result = mutate_registry(
        state_dir,
        operation="registry.destination.set",
        mutator=mutator,
        public_safe=public_safe,
    )
    return _set_result(result, entity="destination", identity=destination.get("destination_id"), action=action)


def bind_source_destination(
    state_dir: Path,
    source_destination: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("source_destinations", []), "source_destination_id", source_destination)
        source_id = source_destination.get("source_destination_id")
        project_id = source_destination.get("project_id")
        for project in registry.setdefault("projects", []):
            if not isinstance(project, dict) or project.get("project_id") != project_id:
                continue
            existing = project.setdefault("source_destination_ids", [])
            if not isinstance(existing, list):
                raise ValueError(f"project source_destination_ids must be a list: {project_id}")
            if source_id not in existing:
                existing.append(source_id)
            break

    return mutate_registry(
        state_dir,
        operation="registry.source-destination.bind",
        mutator=mutator,
        public_safe=public_safe,
    )


def set_source_destination(
    state_dir: Path,
    source_destination: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    action = "pending"
    project_linked = False

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action, project_linked
        source_id = source_destination.get("source_destination_id")
        project_id = source_destination.get("project_id")
        action = _set_unique(
            registry.setdefault("source_destinations", []),
            "source_destination_id",
            source_destination,
        )
        for project in registry.setdefault("projects", []):
            if not isinstance(project, dict):
                continue
            existing = project.setdefault("source_destination_ids", [])
            if not isinstance(existing, list):
                raise ValueError(f"project source_destination_ids must be a list: {project.get('project_id')}")
            if project.get("project_id") == project_id:
                if source_id not in existing:
                    existing.append(source_id)
                project_linked = source_id in existing
            else:
                while source_id in existing:
                    existing.remove(source_id)

    result = mutate_registry(
        state_dir,
        operation="registry.source-destination.set",
        mutator=mutator,
        public_safe=public_safe,
    )
    return _set_result(
        result,
        entity="source_destination",
        identity=source_destination.get("source_destination_id"),
        action=action,
        project_id=source_destination.get("project_id"),
        project_linked=project_linked,
    )


def add_replica(state_dir: Path, replica: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("replicas", []), "replica_id", replica)

    return mutate_registry(state_dir, operation="registry.replica.add", mutator=mutator, public_safe=public_safe)


def set_replica(state_dir: Path, replica: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("replicas", []), "replica_id", replica)

    result = mutate_registry(state_dir, operation="registry.replica.set", mutator=mutator, public_safe=public_safe)
    return _set_result(result, entity="replica", identity=replica.get("replica_id"), action=action)


def record_legacy_import(
    state_dir: Path,
    legacy_import: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("legacy_imports", []), "import_id", legacy_import)

    return mutate_registry(state_dir, operation="registry.legacy-import.record", mutator=mutator, public_safe=public_safe)


def set_legacy_import(state_dir: Path, legacy_import: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    action = "pending"

    def mutator(registry: dict[str, Any]) -> None:
        nonlocal action
        action = _set_unique(registry.setdefault("legacy_imports", []), "import_id", legacy_import)

    result = mutate_registry(
        state_dir,
        operation="registry.legacy-import.set",
        mutator=mutator,
        public_safe=public_safe,
    )
    return _set_result(result, entity="legacy_import", identity=legacy_import.get("import_id"), action=action)


def import_legacy_profile(
    state_dir: Path,
    legacy_profile_import: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    findings = model.validate_legacy_profile_import(legacy_profile_import, public_safe=public_safe)
    errors = [finding for finding in findings if finding.severity == "error"]
    if errors:
        detail = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"invalid legacy profile import: {detail}")

    collection_specs = (
        ("objects", "object_id"),
        ("permissions", "permission_set_id"),
        ("projects", "project_id"),
        ("destinations", "destination_id"),
        ("source_destinations", "source_destination_id"),
        ("replicas", "replica_id"),
        ("legacy_imports", "import_id"),
    )
    counts = {
        key: {"inserted": 0, "skipped_existing": 0}
        for key, _identity_key in collection_specs
    }

    def mutator(service_registry: dict[str, Any]) -> None:
        for key, identity_key in collection_specs:
            collection_counts = counts[key]
            for item in legacy_profile_import.get(key, []):
                _append_unique_or_same(
                    service_registry.setdefault(key, []),
                    identity_key,
                    item,
                    counts=collection_counts,
                )

    result = mutate_registry(
        state_dir,
        operation="registry.legacy-profile.import",
        mutator=mutator,
        public_safe=public_safe,
    )
    return {
        **result,
        "schema_version": "northroot.steward.legacy-profile-import-result.v0",
        "import_id": legacy_profile_import["import_id"],
        "source": legacy_profile_import["source"],
        "import_mode": legacy_profile_import["import_mode"],
        "imported_counts": counts,
    }
