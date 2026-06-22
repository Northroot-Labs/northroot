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
    _atomic_write_json(path, summary)
    return path


def _operation_summary_paths(state_dir: Path) -> list[Path]:
    out_dir = operation_dir(state_dir)
    if not out_dir.is_dir():
        return []
    return sorted(out_dir.glob("*.json"))


def load_registry(state_dir: Path) -> dict[str, Any]:
    return _read_json(registry_path(state_dir))


def _check(name: str, ok: bool, detail: str, *, code: str | None = None) -> dict[str, Any]:
    return {
        "name": name,
        "status": "ok" if ok else "error",
        "code": None if ok else code,
        "detail": detail,
    }


def registry_integrity_report(state_dir: Path, *, public_safe: bool = False) -> dict[str, Any]:
    path = registry_path(state_dir)
    lock = _load_lock(state_dir)
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
    operation_log_readable = not invalid_summaries
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
            "registry operation summaries are readable"
            if operation_log_readable
            else "one or more registry operation summaries are unreadable or invalid",
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
    registry = load_registry(state_dir)
    findings = model.validate_service_registry(registry, public_safe=public_safe)
    error_count = len([finding for finding in findings if finding.severity == "error"])
    integrity = registry_integrity_report(state_dir, public_safe=public_safe)
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
    if path.exists() and not overwrite:
        raise FileExistsError(f"service registry already exists: {path}")
    _validate_or_raise(registry, public_safe=public_safe)
    state_dir.mkdir(parents=True, exist_ok=True)
    digest = _atomic_write_json(path, registry)
    summary = {
        "schema_version": "northroot.steward.registry-operation.v0",
        "operation_id": _operation_id("registry.init"),
        "operation": "registry.init",
        "status": "completed",
        "registry_path": str(path),
        "registry_sha256": digest,
        "public_safe": public_safe,
        "completed_at": _utc_stamp(),
    }
    summary_path = _write_operation_summary(state_dir, summary)
    return {
        "initialized": True,
        "registry_path": str(path),
        "registry_sha256": digest,
        "operation_summary_path": str(summary_path),
    }


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
    lock = {
        "schema_version": "northroot.steward.registry-operation-lock.v0",
        "operation_id": operation_id,
        "operation": operation,
        "started_at": _utc_stamp(),
        "pid": os.getpid(),
        "failure_policy": "fail-closed-record-summary",
        "resume_hint": "run registry recover before applying another mutation",
    }
    _atomic_write_json(lock_path(state_dir), lock)
    before_sha256 = _file_sha256(registry_path(state_dir))
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
    registry = load_registry(state_dir)
    findings = model.validate_service_registry(registry, public_safe=public_safe)
    errors = [finding for finding in findings if finding.severity == "error"]
    operation_id = _operation_id("registry.recover")
    status = "recovered-after-interruption" if not errors else "blocked-invalid-registry"
    summary = {
        "schema_version": "northroot.steward.registry-operation.v0",
        "operation_id": operation_id,
        "operation": "registry.recover",
        "status": status,
        "interrupted_operation_lock": lock,
        "registry_path": str(registry_path(state_dir)),
        "registry_sha256": _file_sha256(registry_path(state_dir)),
        "public_safe": public_safe,
        "findings": [finding.as_dict() for finding in findings],
        "completed_at": _utc_stamp(),
    }
    summary_path = _write_operation_summary(state_dir, summary)
    if errors:
        return {
            "recovered": False,
            "resume_required": True,
            "operation_summary_path": str(summary_path),
            "error_count": len(errors),
            "findings": [finding.as_dict() for finding in findings],
        }
    lock_path(state_dir).unlink(missing_ok=True)
    _fsync_directory(state_dir)
    return {
        "recovered": True,
        "resume_required": False,
        "operation_summary_path": str(summary_path),
        "registry_sha256": summary["registry_sha256"],
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


def add_object(state_dir: Path, custody_object: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("objects", []), "object_id", custody_object)

    return mutate_registry(state_dir, operation="registry.object.add", mutator=mutator, public_safe=public_safe)


def add_permission(state_dir: Path, permission: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("permissions", []), "permission_set_id", permission)

    return mutate_registry(state_dir, operation="registry.permission.add", mutator=mutator, public_safe=public_safe)


def add_project(state_dir: Path, project: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("projects", []), "project_id", project)

    return mutate_registry(state_dir, operation="registry.project.add", mutator=mutator, public_safe=public_safe)


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


def bind_source_destination(
    state_dir: Path,
    source_destination: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("source_destinations", []), "source_destination_id", source_destination)

    return mutate_registry(
        state_dir,
        operation="registry.source-destination.bind",
        mutator=mutator,
        public_safe=public_safe,
    )


def add_replica(state_dir: Path, replica: dict[str, Any], *, public_safe: bool = False) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("replicas", []), "replica_id", replica)

    return mutate_registry(state_dir, operation="registry.replica.add", mutator=mutator, public_safe=public_safe)


def record_legacy_import(
    state_dir: Path,
    legacy_import: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    def mutator(registry: dict[str, Any]) -> None:
        _append_unique(registry.setdefault("legacy_imports", []), "import_id", legacy_import)

    return mutate_registry(state_dir, operation="registry.legacy-import.record", mutator=mutator, public_safe=public_safe)


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
