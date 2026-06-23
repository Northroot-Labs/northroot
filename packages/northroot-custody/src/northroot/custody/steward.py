"""Steward profile helpers built on Northroot custody contracts."""

from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import hashlib
import datetime as dt
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any
from xml.sax.saxutils import escape as xml_escape

from . import model, registry
from .registry import authorize_operation


INSTALLATION_SCHEMA = "northroot.steward.installation.v0"
INSTALLATION_INDEX_FILENAME = "steward-installation-index.json"
INSTALLATION_INDEX_SCHEMA = "northroot.steward.installation-index.v0"
DEFAULT_PROFILE_NAME = "steward"
DEFAULT_RUNNER_COMMAND = "nr steward"
DEFAULT_DOGFOOD_AGENT_ID = "agent:codex"
DEFAULT_DOGFOOD_POLICY_ID = "agent-delegation/dogfood-default"
DEFAULT_DOGFOOD_REPOSITORY_REF = "repository://northroot"
RESTORE_DRILL_DIR = "restore-drills"
OPERATION_LOCK_FILENAME = "steward-operation.lock.json"
RUN_SUMMARY_INDEX_FILENAME = "index.json"
RUN_SUMMARY_INDEX_SCHEMA = "northroot.steward.run-summary-index.v0"
SCHEDULE_MANIFEST_FILENAME = "schedule.json"
SCHEDULE_INDEX_FILENAME = "schedule-index.json"
SCHEDULE_INDEX_SCHEMA = "northroot.steward.schedule-index.v0"
SCHEDULE_CONTEXT_DIRNAME = "contexts"
SCHEDULE_OPERATIONS = {"run", "verify", "restore-drill"}
OPERATIONS = {"run", "verify", "restore", "restore-drill"}
RECOVERABLE_OPERATION_SUMMARY_OPERATIONS = OPERATIONS | {"legacy-run-import"}
REGISTRY_TOPOLOGY_REQUIRED_OPERATIONS = OPERATIONS | {"schedule.create", "schedule.install"}
COMMAND_PLAN_OPERATIONS = {
    "status",
    "preflight",
    "verify-state",
    "capabilities",
    "report",
    "run",
    "verify",
    "restore",
    "restore-drill",
    "schedule.create",
    "schedule.status",
    "schedule.install",
    "schedule.uninstall",
    "schedule.delete",
    "retention.evaluate",
    "evidence.report",
    "evidence.record",
    "offsite.report",
    "import-legacy-runs",
    *model.AGENT_DELEGATION_OPERATIONS,
}


class ScheduleRegistryGateError(ValueError):
    """Raised when registry policy refuses a schedule state mutation."""

    def __init__(self, operation: str, gate: dict[str, Any]) -> None:
        decision = gate.get("decision") or "denied"
        super().__init__(f"registry {operation} denied: {decision}")
        self.operation = operation
        self.gate = gate


@dataclass(frozen=True)
class StewardInstallation:
    schema_version: str
    profile_name: str
    output_dir: str
    snapshot_plan_path: str
    resticprofile_path: str
    secret_bindings_path: str | None
    repository_bindings_path: str | None
    execution_mode: str
    delegated_tool: str
    custom_backup_engine: bool
    generated_artifacts: dict[str, dict[str, str]]
    commands: dict[str, str]

    def as_dict(self) -> dict[str, object]:
        return asdict(self)


def _quote_yaml(value: str) -> str:
    return json.dumps(value)


def _yaml_list(values: list[str], indent: str) -> list[str]:
    if not values:
        return [f"{indent}[]"]
    return [f"{indent}- {_quote_yaml(value)}" for value in values]


def _first_destination(plan: dict[str, Any]) -> dict[str, Any]:
    destinations = plan.get("destinations")
    if not isinstance(destinations, list) or not destinations:
        raise ValueError("snapshot plan must include at least one destination")
    destination = destinations[0]
    if not isinstance(destination, dict):
        raise ValueError("snapshot plan destination must be an object")
    return destination


def _destinations(plan: dict[str, Any]) -> list[dict[str, Any]]:
    destinations = plan.get("destinations")
    if not isinstance(destinations, list):
        return []
    return [destination for destination in destinations if isinstance(destination, dict)]


def _destination_ref(destination: dict[str, Any]) -> str:
    return str(destination.get("repository_ref") or f"repository://{destination.get('id')}")


def _destination_execution(plan: dict[str, Any]) -> dict[str, Any]:
    destinations = _destinations(plan)
    primary = destinations[0] if destinations else None
    additional = destinations[1:] if len(destinations) > 1 else []
    return {
        "primary_destination": {
            "id": str(primary.get("id")) if primary else None,
            "adapter": str(primary.get("adapter")) if primary else None,
            "repository_ref": _destination_ref(primary) if primary else None,
            "handled_by": "resticprofile",
        }
        if primary
        else None,
        "additional_destinations": [
            {
                "id": str(destination.get("id")),
                "adapter": str(destination.get("adapter")),
                "repository_ref": _destination_ref(destination),
                "handling": "external-evidence-required",
                "required_evidence": ["verified_offsite_copy"],
            }
            for destination in additional
        ],
        "additional_destination_handling": "external-evidence-required" if additional else "none",
        "required_external_evidence": ["verified_offsite_copy"] if additional else [],
    }


def _utc_stamp() -> str:
    return dt.datetime.now(tz=dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")


def _command_string(args: list[str]) -> str:
    return " ".join(shlex.quote(arg) for arg in args)


def _file_sha256(path: Path) -> str:
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
    return hashlib.sha256(data).hexdigest()


def _atomic_write_text(path: Path, text: str) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = text.encode("utf-8")
    temp_path = path.with_name(f".{path.name}.{os.getpid()}.{_utc_stamp()}.tmp")
    with temp_path.open("wb") as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temp_path, path)
    _fsync_directory(path.parent)
    return hashlib.sha256(data).hexdigest()


def _artifact(path: Path) -> dict[str, str]:
    return {
        "path": str(path),
        "sha256": _file_sha256(path),
    }


def _slug(value: str) -> str:
    return "".join(char if char.isalnum() else "-" for char in value).strip("-").lower() or "default"


def _schedule_scope_id(
    *,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> str | None:
    if registry_state is None and project_id is None and object_id is None:
        return None
    scope = {
        "registry_state": str(registry_state) if registry_state is not None else None,
        "project_id": project_id,
        "object_id": object_id,
    }
    digest = hashlib.sha256(_json_bytes(scope)).hexdigest()[:12]
    if object_id:
        prefix = f"{_slug(project_id or 'project')}-{_slug(object_id)}"
    elif project_id:
        prefix = _slug(project_id)
    else:
        prefix = "registry"
    return f"{prefix[:48]}-{digest}"


def _schedule_dir(output_dir: Path, *, schedule_scope_id: str | None = None) -> Path:
    if schedule_scope_id:
        return output_dir / "schedules" / SCHEDULE_CONTEXT_DIRNAME / schedule_scope_id
    return output_dir / "schedules"


def _scoped_schedule_manifest_paths(output_dir: Path) -> list[Path]:
    contexts_dir = output_dir / "schedules" / SCHEDULE_CONTEXT_DIRNAME
    if not contexts_dir.is_dir():
        return []
    return sorted(path for path in contexts_dir.glob(f"*/{SCHEDULE_MANIFEST_FILENAME}") if path.is_file())


def _schedule_manifest_path_for_scope(output_dir: Path, *, schedule_scope_id: str | None = None) -> Path:
    return _schedule_dir(output_dir, schedule_scope_id=schedule_scope_id) / SCHEDULE_MANIFEST_FILENAME


def _schedule_index_path_for_scope(output_dir: Path, *, schedule_scope_id: str | None = None) -> Path:
    return _schedule_dir(output_dir, schedule_scope_id=schedule_scope_id) / SCHEDULE_INDEX_FILENAME


def _write_schedule_manifest(output_dir: Path, schedule: dict[str, Any]) -> Path:
    scope_id = schedule.get("schedule_scope_id")
    schedule_scope_id = str(scope_id) if isinstance(scope_id, str) and scope_id else None
    manifest_path = _schedule_manifest_path_for_scope(output_dir, schedule_scope_id=schedule_scope_id)
    sha256 = _atomic_write_json(manifest_path, schedule)
    index = {
        "schema_version": SCHEDULE_INDEX_SCHEMA,
        "updated_at": _utc_stamp(),
        "manifest": {
            "path": SCHEDULE_MANIFEST_FILENAME,
            "sha256": sha256,
            "profile_name": schedule.get("profile_name"),
            "scheduler": schedule.get("scheduler"),
            "operation": schedule.get("operation"),
            "every_minutes": schedule.get("every_minutes"),
            "schedule_scope_id": schedule_scope_id,
        },
    }
    _atomic_write_json(_schedule_index_path_for_scope(output_dir, schedule_scope_id=schedule_scope_id), index)
    return manifest_path


def _schedule_orphan_paths(output_dir: Path, *, schedule_scope_id: str | None = None) -> list[str]:
    schedules_dir = _schedule_dir(output_dir, schedule_scope_id=schedule_scope_id)
    if not schedules_dir.is_dir():
        return []
    protected_names = {SCHEDULE_MANIFEST_FILENAME, SCHEDULE_INDEX_FILENAME}
    return sorted(str(path) for path in schedules_dir.iterdir() if path.is_file() and path.name not in protected_names)


def render_schedule_integrity(
    output_dir: Path,
    *,
    schedule_scope_id: str | None = None,
) -> dict[str, Any]:
    manifest_path = _schedule_manifest_path_for_scope(output_dir, schedule_scope_id=schedule_scope_id)
    index_path = _schedule_index_path_for_scope(output_dir, schedule_scope_id=schedule_scope_id)
    if not manifest_path.exists() and not index_path.exists():
        orphaned_paths = _schedule_orphan_paths(output_dir, schedule_scope_id=schedule_scope_id)
        if orphaned_paths:
            return {
                "schema_version": "northroot.steward.schedule-integrity.v0",
                "ok": False,
                "manifest_path": str(manifest_path),
                "index_path": str(index_path),
                "schedule_scope_id": schedule_scope_id,
                "status": "orphaned-artifacts",
                "detail": "schedule artifacts exist without a schedule manifest; use schedule delete --force after confirming platform registration state",
                "orphaned_paths": orphaned_paths,
            }
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": True,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "not-configured",
            "detail": "no schedule is configured",
        }
    if manifest_path.exists() and not index_path.exists():
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "unindexed",
            "detail": "schedule manifest is not present in the steward digest index",
        }
    if index_path.exists() and not manifest_path.exists():
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "missing-manifest",
            "detail": "indexed schedule manifest is missing",
        }
    try:
        index = model.load_json(index_path)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-index",
            "detail": str(err),
        }
    if index.get("schema_version") != SCHEDULE_INDEX_SCHEMA:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-index",
            "detail": f"schedule index schema_version must be {SCHEDULE_INDEX_SCHEMA}",
        }
    manifest = index.get("manifest")
    if not isinstance(manifest, dict) or manifest.get("path") != SCHEDULE_MANIFEST_FILENAME:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-index",
            "detail": "schedule index manifest entry must reference schedule.json",
        }
    expected_sha = manifest.get("sha256")
    if not isinstance(expected_sha, str) or not expected_sha:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-index",
            "detail": "schedule index manifest entry is missing sha256",
        }
    actual_sha = _file_sha256(manifest_path)
    if actual_sha != expected_sha:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "digest-mismatch",
            "detail": "schedule manifest digest does not match the steward digest index",
            "expected_sha256": expected_sha,
            "actual_sha256": actual_sha,
        }
    try:
        schedule = model.load_json(manifest_path)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-manifest",
            "detail": str(err),
        }
    if schedule.get("schema_version") != "northroot.steward.schedule.v0":
        return {
            "schema_version": "northroot.steward.schedule-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "schedule_scope_id": schedule_scope_id,
            "status": "invalid-manifest",
            "detail": "schedule manifest schema_version must be northroot.steward.schedule.v0",
        }
    return {
        "schema_version": "northroot.steward.schedule-integrity.v0",
        "ok": True,
        "manifest_path": str(manifest_path),
        "index_path": str(index_path),
        "schedule_scope_id": schedule_scope_id,
        "status": "ok",
        "detail": "schedule manifest matches the steward digest index",
        "sha256": actual_sha,
    }


def _xml_text(value: str) -> str:
    return xml_escape(value, {'"': "&quot;", "'": "&apos;"})


def _operation_command_args(
    installation: dict[str, Any],
    operation: str,
    *,
    restore_target: Path | None = None,
    snapshot_id: str | None = None,
) -> list[str]:
    profile_name = str(installation["profile_name"])
    resticprofile_path = str(installation["resticprofile_path"])
    if operation in {"restore", "restore-drill"}:
        if operation == "restore":
            if restore_target is None:
                raise ValueError("restore requires an explicit target")
            if not snapshot_id:
                raise ValueError("restore requires an explicit snapshot_id")
        target = restore_target or Path(str(installation["output_dir"])) / RESTORE_DRILL_DIR / "latest"
        snapshot = snapshot_id if operation == "restore" else "latest"
        return [
            "resticprofile",
            "--config",
            resticprofile_path,
            f"{profile_name}.restore",
            str(snapshot),
            "--target",
            str(target),
        ]
    resticprofile_operation = "backup" if operation == "run" else "check"
    return [
        "resticprofile",
        "--config",
        resticprofile_path,
        f"{profile_name}.{resticprofile_operation}",
    ]


def _observe_restore_target(target: Path) -> dict[str, Any]:
    files: list[Path] = []
    if target.exists():
        if target.is_file():
            files = [target]
        elif target.is_dir():
            files = sorted(path for path in target.rglob("*") if path.is_file())
    digest = hashlib.sha256()
    byte_count = 0
    for path in files:
        relative = path.relative_to(target) if target.is_dir() else Path(path.name)
        stat = path.stat()
        byte_count += stat.st_size
        digest.update(str(relative).encode("utf-8", "surrogateescape"))
        digest.update(b"\0")
        digest.update(str(stat.st_size).encode("ascii"))
        digest.update(b"\0")
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        digest.update(b"\0")
    file_count = len(files)
    return {
        "target": str(target),
        "exists": target.exists(),
        "file_count": file_count,
        "byte_count": byte_count,
        "manifest_sha256": digest.hexdigest() if file_count else None,
        "verified": target.exists() and file_count > 0,
    }


def render_resticprofile_config(
    plan: dict[str, Any],
    *,
    profile_name: str = DEFAULT_PROFILE_NAME,
    secret_bindings: dict[str, Any] | None = None,
    repository_bindings: dict[str, Any] | None = None,
) -> str:
    errors = [finding for finding in model.validate_snapshot_plan(plan) if finding.severity == "error"]
    if errors:
        details = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"cannot render resticprofile config from invalid plan: {details}")
    if secret_bindings is not None:
        binding_errors = [
            finding for finding in model.validate_secret_bindings(secret_bindings) if finding.severity == "error"
        ]
        if binding_errors:
            details = "; ".join(f"{finding.path}: {finding.detail}" for finding in binding_errors)
            raise ValueError(f"cannot render resticprofile config from invalid secret bindings: {details}")
    if repository_bindings is not None:
        repository_binding_errors = [
            finding for finding in model.validate_repository_bindings(repository_bindings) if finding.severity == "error"
        ]
        if repository_binding_errors:
            details = "; ".join(f"{finding.path}: {finding.detail}" for finding in repository_binding_errors)
            raise ValueError(f"cannot render resticprofile config from invalid repository bindings: {details}")

    destinations = _destinations(plan)
    destination = _first_destination(plan)
    repository_ref = _destination_ref(destination)
    repository = model.repository_binding_target(repository_ref, repository_bindings) or repository_ref
    secret_ref = destination.get("secret_ref")
    password_command = None
    if isinstance(secret_ref, str):
        password_command = model.secret_binding_command(secret_ref, secret_bindings)
    sources = [str(source["path"]) for source in plan.get("sources", []) if isinstance(source, dict)]
    excludes = [str(path) for path in plan.get("excludes", [])]
    retention = plan.get("retention") if isinstance(plan.get("retention"), dict) else {}

    lines = [
        "# Generated by Northroot steward. Execution is delegated to resticprofile.",
        "# Private deployments resolve symbolic repository and password refs outside this public config.",
        f"# Primary destination: {destination.get('id')} ({repository_ref}).",
    ]
    additional_destinations = destinations[1:]
    if additional_destinations:
        lines.append(
            "# Additional destinations are not silently backed up by this profile; "
            "record verified_offsite_copy evidence for each completed offsite copy."
        )
        for additional in additional_destinations:
            lines.append(f"# Additional destination: {additional.get('id')} ({_destination_ref(additional)}).")
    lines.extend([
        'version: "1"',
        "profiles:",
        f"  {profile_name}:",
        f"    repository: {_quote_yaml(repository)}",
    ])
    if password_command:
        lines.append(f"    password-command: {_quote_yaml(_command_string(password_command))}")
    lines.extend([
        "    backup:",
        "      source:",
        *_yaml_list(sources, "        "),
    ])
    if excludes:
        lines.extend(["      exclude:", *_yaml_list(excludes, "        ")])
    lines.append("    retention:")
    for resticprofile_key, policy_key in (
        ("keep-daily", "keep_daily"),
        ("keep-weekly", "keep_weekly"),
        ("keep-monthly", "keep_monthly"),
        ("keep-yearly", "keep_yearly"),
    ):
        if policy_key in retention:
            lines.append(f"      {resticprofile_key}: {int(retention[policy_key])}")
    lines.extend(["    check:", "      enabled: true"])
    return "\n".join(lines) + "\n"


def installation_path(output_dir: Path) -> Path:
    return output_dir / "steward-installation.json"


def installation_index_path(output_dir: Path) -> Path:
    return output_dir / INSTALLATION_INDEX_FILENAME


def _write_installation_manifest(output_dir: Path, installation: StewardInstallation) -> None:
    payload = installation.as_dict()
    digest = _atomic_write_json(installation_path(output_dir), payload)
    index = {
        "schema_version": INSTALLATION_INDEX_SCHEMA,
        "updated_at": _utc_stamp(),
        "manifest": {
            "path": installation_path(output_dir).name,
            "sha256": digest,
            "profile_name": payload.get("profile_name"),
            "execution_mode": payload.get("execution_mode"),
            "delegated_tool": payload.get("delegated_tool"),
        },
    }
    _atomic_write_json(installation_index_path(output_dir), index)


def render_installation_integrity(output_dir: Path) -> dict[str, Any]:
    manifest_path = installation_path(output_dir)
    index_path = installation_index_path(output_dir)
    if not manifest_path.exists() and not index_path.exists():
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "missing-installation",
            "detail": "steward installation has not been initialized",
        }
    if manifest_path.exists() and not index_path.exists():
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "unindexed",
            "detail": "steward installation manifest is not present in the digest index",
        }
    if index_path.exists() and not manifest_path.exists():
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "missing-manifest",
            "detail": "indexed steward installation manifest is missing",
        }
    try:
        index = model.load_json(index_path)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-index",
            "detail": str(err),
        }
    if index.get("schema_version") != INSTALLATION_INDEX_SCHEMA:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-index",
            "detail": f"installation index schema_version must be {INSTALLATION_INDEX_SCHEMA}",
        }
    manifest = index.get("manifest")
    if not isinstance(manifest, dict) or manifest.get("path") != manifest_path.name:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-index",
            "detail": "installation index manifest entry must reference steward-installation.json",
        }
    expected_sha = manifest.get("sha256")
    if not isinstance(expected_sha, str) or not expected_sha:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-index",
            "detail": "installation index manifest entry is missing sha256",
        }
    actual_sha = _file_sha256(manifest_path)
    if actual_sha != expected_sha:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "digest-mismatch",
            "detail": "steward installation manifest digest does not match the digest index",
            "expected_sha256": expected_sha,
            "actual_sha256": actual_sha,
        }
    try:
        installation = model.load_json(manifest_path)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-manifest",
            "detail": str(err),
        }
    if installation.get("schema_version") != INSTALLATION_SCHEMA:
        return {
            "schema_version": "northroot.steward.installation-integrity.v0",
            "ok": False,
            "manifest_path": str(manifest_path),
            "index_path": str(index_path),
            "status": "invalid-manifest",
            "detail": f"installation manifest schema_version must be {INSTALLATION_SCHEMA}",
        }
    return {
        "schema_version": "northroot.steward.installation-integrity.v0",
        "ok": True,
        "manifest_path": str(manifest_path),
        "index_path": str(index_path),
        "status": "ok",
        "detail": "steward installation manifest matches the digest index",
        "sha256": actual_sha,
    }


def init_steward(
    *,
    inventory_path: Path,
    policy_path: Path,
    output_dir: Path,
    profile_name: str = DEFAULT_PROFILE_NAME,
    secret_bindings_path: Path | None = None,
    repository_bindings_path: Path | None = None,
) -> StewardInstallation:
    inventory = model.load_json(inventory_path)
    policy = model.load_json(policy_path)
    secret_bindings = model.load_json(secret_bindings_path) if secret_bindings_path is not None else None
    repository_bindings = model.load_json(repository_bindings_path) if repository_bindings_path is not None else None
    plan = model.render_snapshot_plan(inventory, policy)

    output_dir.mkdir(parents=True, exist_ok=True)
    plan_path = output_dir / "snapshot-plan.json"
    resticprofile_path = output_dir / "resticprofile.yaml"
    _atomic_write_json(plan_path, plan)
    _atomic_write_text(
        resticprofile_path,
        render_resticprofile_config(
            plan,
            profile_name=profile_name,
            secret_bindings=secret_bindings,
            repository_bindings=repository_bindings,
        ),
    )

    installation = StewardInstallation(
        schema_version=INSTALLATION_SCHEMA,
        profile_name=profile_name,
        output_dir=str(output_dir),
        snapshot_plan_path=str(plan_path),
        resticprofile_path=str(resticprofile_path),
        secret_bindings_path=str(secret_bindings_path) if secret_bindings_path is not None else None,
        repository_bindings_path=str(repository_bindings_path) if repository_bindings_path is not None else None,
        execution_mode="delegated",
        delegated_tool="resticprofile",
        custom_backup_engine=False,
        generated_artifacts={
            "snapshot_plan": _artifact(plan_path),
            "resticprofile_config": _artifact(resticprofile_path),
        },
        commands={
            "run": _command_string(["resticprofile", "--config", str(resticprofile_path), f"{profile_name}.backup"]),
            "verify": _command_string(["resticprofile", "--config", str(resticprofile_path), f"{profile_name}.check"]),
            "restore": f"{DEFAULT_RUNNER_COMMAND} restore --state {output_dir} --snapshot-id <snapshot-id> --target <target>",
            "status": f"{DEFAULT_RUNNER_COMMAND} status --state {output_dir}",
            "preflight": f"{DEFAULT_RUNNER_COMMAND} preflight --state {output_dir}",
            "verify_state": f"{DEFAULT_RUNNER_COMMAND} verify-state --state {output_dir}",
            "capabilities": f"{DEFAULT_RUNNER_COMMAND} capabilities --state {output_dir}",
            "evidence": f"{DEFAULT_RUNNER_COMMAND} evidence report --state {output_dir}",
        },
    )
    _write_installation_manifest(output_dir, installation)
    return installation


def load_installation(output_dir: Path) -> dict[str, Any]:
    return model.load_json(installation_path(output_dir))


def _check(name: str, ok: bool, detail: str, *, code: str | None = None) -> dict[str, Any]:
    return {
        "name": name,
        "status": "ok" if ok else "error",
        "code": code,
        "detail": detail,
    }


def _executable_available(command: list[str]) -> bool:
    if not command:
        return False
    executable = command[0]
    if os.path.isabs(executable):
        return os.path.isfile(executable) and os.access(executable, os.X_OK)
    return shutil.which(executable) is not None


def _runner_command_available(runner_command: str) -> bool:
    try:
        args = shlex.split(runner_command)
    except ValueError:
        return False
    return _executable_available(args)


def _run_probe_command(command: list[str], *, timeout_seconds: int) -> dict[str, Any]:
    if not _executable_available(command):
        return {
            "ok": False,
            "return_code": None,
            "detail": f"{command[0] if command else 'probe'} is not executable",
            "failure": "missing_probe_command",
        }
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired:
        return {
            "ok": False,
            "return_code": None,
            "detail": f"probe timed out after {timeout_seconds}s",
            "failure": "probe_timeout",
        }
    return {
        "ok": completed.returncode == 0,
        "return_code": completed.returncode,
        "detail": "probe succeeded" if completed.returncode == 0 else f"probe exited {completed.returncode}",
        "failure": None if completed.returncode == 0 else "probe_failed",
    }


def _runtime_env_for_execution(secret_bindings: dict[str, Any] | None) -> dict[str, str]:
    env = dict(os.environ)
    if not secret_bindings:
        return env
    for binding in secret_bindings.get("runtime_env", []):
        if not isinstance(binding, dict):
            continue
        name = binding.get("name")
        if not isinstance(name, str) or not name:
            continue
        if env.get(name):
            continue
        command = model.runtime_env_command(name, secret_bindings)
        if not command:
            continue
        completed = subprocess.run(command, check=False, capture_output=True, text=True)
        if completed.returncode != 0:
            raise RuntimeError(f"runtime environment binding for {name} failed with status {completed.returncode}")
        value = completed.stdout.rstrip("\r\n")
        if not value:
            raise RuntimeError(f"runtime environment binding for {name} returned an empty value")
        env[name] = value
    return env


def render_preflight(
    output_dir: Path,
    *,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    installation = load_installation(output_dir)
    installation_integrity = render_installation_integrity(output_dir)
    plan_path = Path(str(installation["snapshot_plan_path"]))
    resticprofile_path = Path(str(installation["resticprofile_path"]))
    checks: list[dict[str, Any]] = []

    checks.append(_check("installation", True, f"loaded {installation_path(output_dir)}"))
    checks.append(
        _check(
            "installation_manifest_integrity",
            bool(installation_integrity["ok"]),
            "steward installation manifest matches digest index"
            if installation_integrity["ok"]
            else "steward installation manifest failed digest index verification",
            code=None if installation_integrity["ok"] else "installation_manifest_integrity_failed",
        )
    )
    if not installation_integrity["ok"]:
        return {
            "schema_version": "northroot.steward.preflight.v0",
            "profile_name": installation.get("profile_name"),
            "ready": False,
            "execution_mode": installation.get("execution_mode"),
            "delegated_tool": installation.get("delegated_tool"),
            "custom_backup_engine": installation.get("custom_backup_engine"),
            "installation_integrity": installation_integrity,
            "checks": checks,
        }
    checks.append(
        _check(
            "resticprofile_config",
            resticprofile_path.is_file(),
            f"found {resticprofile_path}" if resticprofile_path.is_file() else f"missing {resticprofile_path}",
            code=None if resticprofile_path.is_file() else "missing_resticprofile_config",
        )
    )

    generated_artifacts = installation.get("generated_artifacts")
    if isinstance(generated_artifacts, dict):
        for artifact_name, artifact in generated_artifacts.items():
            if not isinstance(artifact, dict):
                checks.append(
                    _check(
                        f"generated_artifact:{artifact_name}",
                        False,
                        "generated artifact metadata must be an object",
                        code="invalid_generated_artifact_metadata",
                    )
                )
                continue
            artifact_path = Path(str(artifact.get("path", "")))
            expected_sha256 = artifact.get("sha256")
            if not artifact_path.is_file():
                checks.append(
                    _check(
                        f"generated_artifact:{artifact_name}",
                        False,
                        f"missing generated artifact {artifact_path}",
                        code="missing_generated_artifact",
                    )
                )
                continue
            actual_sha256 = _file_sha256(artifact_path)
            checks.append(
                _check(
                    f"generated_artifact:{artifact_name}",
                    actual_sha256 == expected_sha256,
                    "generated artifact matches installation metadata"
                    if actual_sha256 == expected_sha256
                    else f"generated artifact drifted: {artifact_path}",
                    code=None if actual_sha256 == expected_sha256 else "generated_artifact_drift",
                )
            )

    plan = model.load_json(plan_path) if plan_path.is_file() else {}
    plan_errors = [finding for finding in model.validate_snapshot_plan(plan) if finding.severity == "error"]
    checks.append(
        _check(
            "snapshot_plan",
            not plan_errors,
            "snapshot plan is valid" if not plan_errors else "; ".join(f"{f.path}: {f.detail}" for f in plan_errors),
            code=None if not plan_errors else "invalid_snapshot_plan",
        )
    )

    resticprofile_available = shutil.which("resticprofile") is not None
    checks.append(
        _check(
            "delegated_tool",
            resticprofile_available,
            "resticprofile found on PATH" if resticprofile_available else "resticprofile is not on PATH",
            code=None if resticprofile_available else "missing_delegated_tool",
        )
    )

    secret_bindings_path = installation.get("secret_bindings_path")
    secret_bindings_file = Path(str(secret_bindings_path)) if secret_bindings_path else None
    secret_bindings = model.load_json(secret_bindings_file) if secret_bindings_file and secret_bindings_file.is_file() else None
    repository_bindings_path = installation.get("repository_bindings_path")
    repository_bindings_file = Path(str(repository_bindings_path)) if repository_bindings_path else None
    repository_bindings = (
        model.load_json(repository_bindings_file)
        if repository_bindings_file and repository_bindings_file.is_file()
        else None
    )
    required_repository_refs = sorted(
        {
            str(destination["repository_ref"])
            for destination in plan.get("destinations", [])
            if isinstance(destination, dict) and isinstance(destination.get("repository_ref"), str)
        }
    )
    if required_repository_refs and not repository_bindings_path:
        checks.append(
            _check(
                "repository_bindings",
                False,
                f"missing repository bindings for {', '.join(required_repository_refs)}",
                code="missing_repository_bindings",
            )
        )
    elif repository_bindings_file and not repository_bindings_file.is_file():
        checks.append(
            _check(
                "repository_bindings",
                False,
                f"missing {repository_bindings_file}",
                code="missing_repository_bindings_file",
            )
        )
    elif repository_bindings is not None:
        repository_binding_errors = [
            finding for finding in model.validate_repository_bindings(repository_bindings) if finding.severity == "error"
        ]
        checks.append(
            _check(
                "repository_bindings",
                not repository_binding_errors,
                "repository bindings are valid"
                if not repository_binding_errors
                else "; ".join(f"{f.path}: {f.detail}" for f in repository_binding_errors),
                code=None if not repository_binding_errors else "invalid_repository_bindings",
            )
        )
        for repository_ref in required_repository_refs:
            target = model.repository_binding_target(repository_ref, repository_bindings)
            checks.append(
                _check(
                    f"repository_binding:{repository_ref}",
                    target is not None,
                    "repository target is configured" if target else "repository target is missing",
                    code=None if target else "missing_repository_binding",
                )
            )
            availability_check = model.repository_binding_availability_check(repository_ref, repository_bindings)
            if availability_check:
                probe = _run_probe_command(
                    availability_check["command"],
                    timeout_seconds=int(availability_check["timeout_seconds"]),
                )
                checks.append(
                    _check(
                        f"repository_availability:{repository_ref}",
                        bool(probe["ok"]),
                        f"{repository_ref} storage is available"
                        if probe["ok"]
                        else f"{repository_ref} storage is unavailable: {probe['detail']}",
                        code=None if probe["ok"] else "repository_storage_unavailable",
                    )
                )
    required_secret_refs = sorted(
        {
            str(destination["secret_ref"])
            for destination in plan.get("destinations", [])
            if isinstance(destination, dict) and isinstance(destination.get("secret_ref"), str)
        }
    )
    if required_secret_refs and not secret_bindings_path:
        checks.append(
            _check(
                "secret_bindings",
                False,
                f"missing secret bindings for {', '.join(required_secret_refs)}",
                code="missing_secret_bindings",
            )
        )
    elif secret_bindings_file and not secret_bindings_file.is_file():
        checks.append(
            _check(
                "secret_bindings",
                False,
                f"missing {secret_bindings_file}",
                code="missing_secret_bindings_file",
            )
        )
    elif secret_bindings is not None:
        binding_errors = [
            finding for finding in model.validate_secret_bindings(secret_bindings) if finding.severity == "error"
        ]
        checks.append(
            _check(
                "secret_bindings",
                not binding_errors,
                "secret bindings are valid"
                if not binding_errors
                else "; ".join(f"{f.path}: {f.detail}" for f in binding_errors),
                code=None if not binding_errors else "invalid_secret_bindings",
            )
        )
        for secret_ref in required_secret_refs:
            command = model.secret_binding_command(secret_ref, secret_bindings)
            checks.append(
                _check(
                    f"secret_binding:{secret_ref}",
                    command is not None,
                    "secret binding command is configured" if command else "secret binding command is missing",
                    code=None if command else "missing_secret_binding",
                )
            )
            if command:
                command_ok = _executable_available(command)
                checks.append(
                    _check(
                        f"secret_command:{secret_ref}",
                        command_ok,
                        f"{command[0]} is available" if command_ok else f"{command[0]} is not executable",
                        code=None if command_ok else "missing_secret_command",
                    )
                )
            binding = model.resolve_secret_binding(secret_ref, secret_bindings)
            required_env = binding.get("requires_env", []) if isinstance(binding, dict) else []
            for env_name in required_env:
                env_command = model.runtime_env_command(str(env_name), secret_bindings) if isinstance(env_name, str) else None
                env_command_ok = _executable_available(env_command) if env_command else False
                env_ok = isinstance(env_name, str) and (bool(os.environ.get(env_name)) or env_command_ok)
                if env_command and not os.environ.get(str(env_name)):
                    checks.append(
                        _check(
                            f"runtime_env:{env_name}",
                            env_command_ok,
                            f"{env_name} can be populated by {env_command[0]}"
                            if env_command_ok
                            else f"{env_command[0]} is not executable",
                            code=None if env_command_ok else "missing_runtime_env_command",
                        )
                    )
                checks.append(
                    _check(
                        f"secret_env:{secret_ref}:{env_name}",
                        env_ok,
                        f"{env_name} is set or has a runtime binding"
                        if env_ok
                        else f"{env_name} is not set and has no usable runtime binding",
                        code=None if env_ok else "missing_secret_env",
                    )
                )

    schedule = schedule_status(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    schedule_integrity = schedule.get("schedule_integrity")
    schedule_integrity_ok = isinstance(schedule_integrity, dict) and bool(schedule_integrity.get("ok"))
    if schedule.get("configured") or not schedule_integrity_ok:
        checks.append(
            _check(
                "schedule_manifest_integrity",
                schedule_integrity_ok,
                "schedule manifest matches schedule digest index"
                if schedule_integrity_ok
                else "schedule manifest failed steward digest index verification",
                code=None if schedule_integrity_ok else "schedule_manifest_integrity_failed",
            )
        )
        if not schedule_integrity_ok:
            return {
                "schema_version": "northroot.steward.preflight.v0",
                "profile_name": installation["profile_name"],
                "ready": False,
                "schedule": schedule,
                "checks": checks,
            }
        schedule_artifacts = schedule.get("generated_artifacts")
        if not isinstance(schedule_artifacts, dict):
            checks.append(
                _check(
                    "schedule_generated_artifacts",
                    False,
                    "schedule generated artifact metadata is missing; recreate the schedule",
                    code="missing_schedule_generated_artifacts",
                )
            )
        else:
            for artifact_name, artifact in schedule_artifacts.items():
                if not isinstance(artifact, dict):
                    checks.append(
                        _check(
                            f"schedule_generated_artifact:{artifact_name}",
                            False,
                            "schedule generated artifact metadata must be an object",
                            code="invalid_schedule_generated_artifact_metadata",
                        )
                    )
                    continue
                artifact_path = Path(str(artifact.get("path", "")))
                expected_sha256 = artifact.get("sha256")
                if not artifact_path.is_file():
                    checks.append(
                        _check(
                            f"schedule_generated_artifact:{artifact_name}",
                            False,
                            f"missing schedule generated artifact {artifact_path}",
                            code="missing_schedule_generated_artifact",
                        )
                    )
                    continue
                actual_sha256 = _file_sha256(artifact_path)
                checks.append(
                    _check(
                        f"schedule_generated_artifact:{artifact_name}",
                        actual_sha256 == expected_sha256,
                        "schedule generated artifact matches schedule metadata"
                        if actual_sha256 == expected_sha256
                        else f"schedule generated artifact drifted: {artifact_path}",
                        code=None if actual_sha256 == expected_sha256 else "schedule_generated_artifact_drift",
                    )
                )
        runner_command = str(schedule.get("runner_command") or DEFAULT_RUNNER_COMMAND)
        runner_ok = _runner_command_available(runner_command)
        checks.append(
            _check(
                "scheduled_runner_command",
                runner_ok,
                f"scheduled runner command is available: {runner_command}"
                if runner_ok
                else f"scheduled runner command is not executable on PATH: {runner_command}",
                code=None if runner_ok else "missing_scheduled_runner_command",
            )
        )

    ready = all(check["status"] == "ok" for check in checks)
    return {
        "schema_version": "northroot.steward.preflight.v0",
        "profile_name": installation["profile_name"],
        "ready": ready,
        "execution_mode": installation["execution_mode"],
        "delegated_tool": installation["delegated_tool"],
        "custom_backup_engine": installation["custom_backup_engine"],
        "installation_integrity": installation_integrity,
        "destination_execution": _destination_execution(plan),
        "schedule": schedule,
        "checks": checks,
    }


def render_status(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
    installation_integrity = render_installation_integrity(output_dir)
    plan_path = Path(str(installation["snapshot_plan_path"]))
    plan = model.load_json(plan_path)
    destination_execution = _destination_execution(plan)
    primary_destination = destination_execution["primary_destination"] or {}
    return {
        "schema_version": "northroot.steward.status.v0",
        "profile_name": installation["profile_name"],
        "execution_mode": installation["execution_mode"],
        "delegated_tool": installation["delegated_tool"],
        "custom_backup_engine": installation["custom_backup_engine"],
        "snapshot_plan_path": str(plan_path),
        "resticprofile_path": installation["resticprofile_path"],
        "installation_integrity": installation_integrity,
        "generated_artifacts": installation.get("generated_artifacts", {}),
        "source_count": len(plan.get("sources", [])),
        "destination_count": len(plan.get("destinations", [])),
        "primary_destination_id": primary_destination.get("id"),
        "primary_repository_ref": primary_destination.get("repository_ref"),
        "external_destination_count": len(destination_execution["additional_destinations"]),
        "external_destination_ids": [
            destination["id"] for destination in destination_execution["additional_destinations"]
        ],
        "external_destination_evidence_required": destination_execution["required_external_evidence"],
        "destination_execution": destination_execution,
        "verification_required": plan.get("verification_required", []),
        "retention_prune_requires": plan.get("retention_prune_requires", []),
        "latest_run_summary_path": latest_run_summary_path(output_dir),
        "schedule": schedule_status(output_dir),
        "commands": installation["commands"],
    }


def _agent_operation_contracts(output_dir: Path) -> list[dict[str, Any]]:
    state = str(output_dir)
    return [
        {
            "name": "status",
            "argv_template": ["nr", "steward", "status", "--state", state],
            "required_inputs": [],
            "optional_inputs": [],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.status.v0",
        },
        {
            "name": "preflight",
            "argv_template": ["nr", "steward", "preflight", "--state", state],
            "required_inputs": [],
            "optional_inputs": [],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.preflight.v0",
        },
        {
            "name": "capabilities",
            "argv_template": ["nr", "steward", "capabilities", "--state", state],
            "required_inputs": [],
            "optional_inputs": [],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.capabilities.v0",
        },
        {
            "name": "command-plan",
            "argv_template": ["nr", "steward", "command-plan", "--state", state, "--operation", "{operation}"],
            "required_inputs": ["operation"],
            "optional_inputs": [
                "snapshot_id",
                "target",
                "execute",
                "scheduler",
                "schedule_operation",
                "every_minutes",
                "runner_command",
                "evidence",
                "source",
                "detail",
                "artifact_ref",
                "json",
                "force",
                "use_recorded_evidence",
                "skip_preflight",
                "agent_id",
                "branch",
                "base_branch",
                "commit_message",
                "pr_title",
                "pr_body",
                "remote",
            ],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.command-plan.v0",
            "allowed_operations": sorted(COMMAND_PLAN_OPERATIONS),
        },
        {
            "name": "verify-state",
            "argv_template": ["nr", "steward", "verify-state", "--state", state],
            "snapshot_argv_template": ["nr", "steward", "verify-state", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.state-verification.v0",
        },
        {
            "name": "report",
            "argv_template": ["nr", "steward", "report", "--state", state],
            "snapshot_argv_template": ["nr", "steward", "report", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.report.v0",
        },
        {
            "name": "run",
            "argv_template": ["nr", "steward", "run", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "execute_argv_template": [
                "nr",
                "steward",
                "run",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--execute",
            ],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id"],
            "recommended_inputs": ["snapshot_id"],
            "side_effects": ["writes_run_summary", "delegates_backup_repository_mutation_when_execute_is_set"],
            "requires_preflight": True,
            "success_schema": "northroot.steward.operation.v0",
        },
        {
            "name": "verify",
            "argv_template": ["nr", "steward", "verify", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "execute_argv_template": [
                "nr",
                "steward",
                "verify",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--execute",
            ],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id"],
            "required_for_retention_inputs": ["snapshot_id"],
            "side_effects": ["writes_run_summary"],
            "requires_preflight": True,
            "success_schema": "northroot.steward.operation.v0",
        },
        {
            "name": "restore",
            "argv_template": [
                "nr",
                "steward",
                "restore",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--target",
                "{target}",
            ],
            "execute_argv_template": [
                "nr",
                "steward",
                "restore",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--target",
                "{target}",
                "--execute",
            ],
            "required_inputs": ["snapshot_id", "target"],
            "optional_inputs": [],
            "side_effects": ["writes_run_summary", "writes_restore_target_when_execute_is_set"],
            "requires_preflight": True,
            "success_schema": "northroot.steward.operation.v0",
            "satisfies_retention_evidence": [],
        },
        {
            "name": "restore-drill",
            "argv_template": ["nr", "steward", "restore-drill", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "execute_argv_template": [
                "nr",
                "steward",
                "restore-drill",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--execute",
            ],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id", "target"],
            "required_for_retention_inputs": ["snapshot_id"],
            "side_effects": ["writes_run_summary", "writes_restore_drill_directory_when_execute_is_set"],
            "requires_preflight": True,
            "success_schema": "northroot.steward.operation.v0",
            "satisfies_retention_evidence": ["restore_drill"],
        },
        {
            "name": "schedule.create",
            "argv_template": [
                "nr",
                "steward",
                "schedule",
                "create",
                "--state",
                state,
                "--scheduler",
                "{launchd|systemd}",
                "--operation",
                "{run|verify|restore-drill}",
                "--every-minutes",
                "{minutes}",
            ],
            "required_inputs": ["scheduler", "operation", "every_minutes"],
            "optional_inputs": ["runner_command"],
            "side_effects": ["writes_generated_schedule_template"],
            "requires_preflight": False,
            "success_schema": "northroot.steward.schedule.v0",
        },
        {
            "name": "schedule.status",
            "argv_template": ["nr", "steward", "schedule", "status", "--state", state],
            "required_inputs": [],
            "optional_inputs": [],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.schedule-status.v0",
        },
        {
            "name": "schedule.install",
            "argv_template": ["nr", "steward", "schedule", "install", "--state", state],
            "execute_argv_template": ["nr", "steward", "schedule", "install", "--state", state, "--execute"],
            "required_inputs": [],
            "optional_inputs": ["skip_preflight"],
            "side_effects": ["delegates_platform_scheduler_registration_when_execute_is_set"],
            "requires_preflight": True,
            "success_schema": "northroot.steward.schedule-install.v0",
        },
        {
            "name": "schedule.uninstall",
            "argv_template": ["nr", "steward", "schedule", "uninstall", "--state", state],
            "execute_argv_template": ["nr", "steward", "schedule", "uninstall", "--state", state, "--execute"],
            "required_inputs": [],
            "optional_inputs": [],
            "side_effects": ["delegates_platform_scheduler_removal_when_execute_is_set"],
            "requires_preflight": False,
            "success_schema": "northroot.steward.schedule-uninstall.v0",
        },
        {
            "name": "schedule.delete",
            "argv_template": ["nr", "steward", "schedule", "delete", "--state", state],
            "force_argv_template": ["nr", "steward", "schedule", "delete", "--state", state, "--force"],
            "required_inputs": [],
            "optional_inputs": ["force"],
            "side_effects": ["deletes_generated_schedule_artifacts"],
            "requires_preflight": False,
            "success_schema": "northroot.steward.schedule-delete.v0",
        },
        {
            "name": "retention.evaluate",
            "argv_template": ["nr", "steward", "retention", "evaluate", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "required_inputs": ["snapshot_id"],
            "optional_inputs": ["evidence", "use_recorded_evidence"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": model.RETENTION_DECISION_SCHEMA,
        },
        {
            "name": "evidence.report",
            "argv_template": ["nr", "steward", "evidence", "report", "--state", state],
            "snapshot_argv_template": ["nr", "steward", "evidence", "report", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "required_inputs": [],
            "optional_inputs": ["snapshot_id"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.evidence-report.v0",
        },
        {
            "name": "offsite.report",
            "argv_template": ["nr", "steward", "offsite", "report", "--state", state, "--snapshot-id", "{snapshot_id}"],
            "required_inputs": ["snapshot_id"],
            "optional_inputs": [],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "northroot.steward.offsite-report.v0",
        },
        {
            "name": "evidence.record",
            "argv_template": [
                "nr",
                "steward",
                "evidence",
                "record",
                "--state",
                state,
                "--snapshot-id",
                "{snapshot_id}",
                "--evidence",
                "verified_offsite_copy",
                "--source",
                "{source}",
            ],
            "required_inputs": ["snapshot_id", "source"],
            "optional_inputs": ["detail", "artifact_ref"],
            "side_effects": ["writes_run_summary"],
            "requires_preflight": False,
            "success_schema": "northroot.steward.evidence-record.v0",
            "allowed_evidence": sorted(model.EXTERNAL_RETENTION_EVIDENCE),
        },
        {
            "name": "import-legacy-runs",
            "argv_template": ["nr", "steward", "import-legacy-runs", "--state", state, "--json", "{json}", "--public-safe"],
            "required_inputs": ["json"],
            "optional_inputs": ["public_safe"],
            "side_effects": ["writes_run_summary", "uses_operation_lock"],
            "requires_preflight": False,
            "success_schema": "northroot.steward.legacy-run-import-result.v0",
        },
    ] + _agent_workflow_operation_contracts()


def default_dogfood_agent_delegation_policy() -> dict[str, Any]:
    return {
        "schema_version": model.AGENT_DELEGATION_POLICY_SCHEMA,
        "policy_id": DEFAULT_DOGFOOD_POLICY_ID,
        "default_for_dogfood": True,
        "scope": {
            "repository_ref": DEFAULT_DOGFOOD_REPOSITORY_REF,
            "protected_base_branch": "main",
            "delegated_branch_prefixes": ["codex/", "agent/"],
        },
        "registered_agents": [
            {
                "agent_id": DEFAULT_DOGFOOD_AGENT_ID,
                "display_name": "Codex",
                "branch_prefixes": ["codex/"],
                "allowed_operations": [
                    "branch.checkout",
                    "branch.create",
                    "commit.create",
                    "commit.verify",
                    "push.branch",
                    "pr.draft.open",
                    "pr.draft.update",
                    "pr.comment.follow-up",
                    "pr.check.verify",
                ],
                "required_metadata": {
                    "author_identity": DEFAULT_DOGFOOD_AGENT_ID,
                    "committer_identity": DEFAULT_DOGFOOD_AGENT_ID,
                    "coauthorship_policy": "agent-authored",
                    "provenance_headers": [
                        "Agent-Id",
                        "Agent-Policy-Id",
                        "Agent-Branch",
                        "Agent-Verification",
                        "Agent-Coauthorship",
                    ],
                    "commit_trailers": {
                        "Agent-Id": DEFAULT_DOGFOOD_AGENT_ID,
                        "Agent-Policy-Id": DEFAULT_DOGFOOD_POLICY_ID,
                        "Agent-Branch": "<delegated-branch>",
                        "Agent-Verification": "<verification-summary>",
                        "Agent-Coauthorship": "agent-authored",
                    },
                },
            }
        ],
        "draft_pr_policy": {
            "allow_open_draft_prs": True,
            "allow_update_draft_prs": True,
            "allow_ready_for_review_without_clearance": False,
            "final_human_review_required": True,
        },
        "verification_policy": {
            "required_before_push": [
                "git status --short --branch",
                "focused tests for touched surface",
                "public-safe scan for private deployment strings",
            ],
            "record_failed_checks_before_follow_up": True,
        },
        "prohibited_operations": [
            "merge.protected-branch",
            "push.protected-branch",
            "modify.branch-protection",
            "workflow.permission-escalation",
            "impersonate-human-author",
            "access.long-lived-signing-key",
        ],
    }


def _registered_agent(policy: dict[str, Any], agent_id: str) -> dict[str, Any] | None:
    agents = policy.get("registered_agents")
    if not isinstance(agents, list):
        return None
    for agent in agents:
        if isinstance(agent, dict) and agent.get("agent_id") == agent_id:
            return agent
    return None


def _is_protected_branch(branch: str, policy: dict[str, Any]) -> bool:
    scope = policy.get("scope") if isinstance(policy.get("scope"), dict) else {}
    protected = str(scope.get("protected_base_branch") or "main")
    protected_roots = {protected, "main", "master", "release"}
    return branch in protected_roots or branch.startswith(tuple(f"{root}/" for root in protected_roots))


def _agent_branch_allowed(branch: str, *, agent: dict[str, Any], policy: dict[str, Any]) -> bool:
    scope = policy.get("scope") if isinstance(policy.get("scope"), dict) else {}
    delegated_prefixes = scope.get("delegated_branch_prefixes")
    agent_prefixes = agent.get("branch_prefixes")
    prefixes = [str(item) for item in agent_prefixes if isinstance(item, str)] if isinstance(agent_prefixes, list) else []
    if not prefixes and isinstance(delegated_prefixes, list):
        prefixes = [str(item) for item in delegated_prefixes if isinstance(item, str)]
    return bool(prefixes) and any(branch.startswith(prefix) for prefix in prefixes) and not _is_protected_branch(branch, policy)


def _agent_commit_trailers(agent: dict[str, Any], *, branch: str, verification: str | None) -> dict[str, str]:
    metadata = agent.get("required_metadata") if isinstance(agent.get("required_metadata"), dict) else {}
    trailers = metadata.get("commit_trailers") if isinstance(metadata.get("commit_trailers"), dict) else {}
    return {
        "Agent-Id": str(trailers.get("Agent-Id") or agent.get("agent_id") or DEFAULT_DOGFOOD_AGENT_ID),
        "Agent-Policy-Id": str(trailers.get("Agent-Policy-Id") or DEFAULT_DOGFOOD_POLICY_ID),
        "Agent-Branch": branch,
        "Agent-Verification": verification or "<verification-summary>",
        "Agent-Coauthorship": str(trailers.get("Agent-Coauthorship") or "agent-authored"),
    }


def _agent_workflow_operation_contracts() -> list[dict[str, Any]]:
    return [
        {
            "name": "branch.create",
            "argv_template": ["git", "switch", "-c", "{branch}", "{base_branch}"],
            "required_inputs": ["branch"],
            "optional_inputs": ["base_branch", "agent_id"],
            "side_effects": ["mutates_git_worktree"],
            "requires_preflight": False,
            "success_schema": "external.git.branch-create",
        },
        {
            "name": "branch.checkout",
            "argv_template": ["git", "switch", "{branch}"],
            "required_inputs": ["branch"],
            "optional_inputs": ["agent_id"],
            "side_effects": ["mutates_git_worktree"],
            "requires_preflight": False,
            "success_schema": "external.git.branch-checkout",
        },
        {
            "name": "commit.create",
            "argv_template": ["git", "commit", "--message", "{commit_message}", "--trailer", "Agent-Id={agent_id}"],
            "required_inputs": ["branch", "commit_message"],
            "optional_inputs": ["verification", "agent_id"],
            "side_effects": ["writes_git_commit"],
            "requires_preflight": False,
            "success_schema": "external.git.commit",
        },
        {
            "name": "commit.verify",
            "argv_template": ["git", "show", "-s", "--format=%H%n%an%n%ae%n%cn%n%ce%n%B", "HEAD"],
            "required_inputs": ["branch"],
            "optional_inputs": ["agent_id"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "external.git.commit-inspection",
        },
        {
            "name": "push.branch",
            "argv_template": ["git", "push", "-u", "{remote}", "{branch}"],
            "required_inputs": ["branch"],
            "optional_inputs": ["remote", "agent_id"],
            "side_effects": ["pushes_git_remote"],
            "requires_preflight": False,
            "success_schema": "external.git.push",
        },
        {
            "name": "pr.draft.open",
            "argv_template": [
                "gh",
                "pr",
                "create",
                "--draft",
                "--base",
                "{base_branch}",
                "--head",
                "{branch}",
                "--title",
                "{pr_title}",
                "--body",
                "{pr_body}",
            ],
            "required_inputs": ["branch", "pr_title"],
            "optional_inputs": ["base_branch", "pr_body", "agent_id"],
            "side_effects": ["opens_draft_pull_request"],
            "requires_preflight": False,
            "success_schema": "external.github.draft-pr",
        },
        {
            "name": "pr.draft.update",
            "argv_template": ["gh", "pr", "edit", "{branch}", "--title", "{pr_title}", "--body", "{pr_body}"],
            "required_inputs": ["branch", "pr_title_or_pr_body"],
            "optional_inputs": ["pr_title", "pr_body", "agent_id"],
            "side_effects": ["updates_draft_pull_request"],
            "requires_preflight": False,
            "success_schema": "external.github.pr-edit",
        },
        {
            "name": "pr.comment.follow-up",
            "argv_template": ["gh", "pr", "comment", "{branch}", "--body", "{detail}"],
            "required_inputs": ["branch", "detail"],
            "optional_inputs": ["agent_id"],
            "side_effects": ["writes_pull_request_comment"],
            "requires_preflight": False,
            "success_schema": "external.github.pr-comment",
        },
        {
            "name": "pr.check.verify",
            "argv_template": ["gh", "pr", "checks", "{branch}"],
            "required_inputs": ["branch"],
            "optional_inputs": ["agent_id"],
            "side_effects": [],
            "requires_preflight": False,
            "success_schema": "external.github.pr-checks",
        },
    ]


def _append_snapshot(argv: list[str], snapshot_id: str | None) -> None:
    if snapshot_id:
        argv.extend(["--snapshot-id", snapshot_id])


def _operation_lock_status(output_dir: Path) -> dict[str, Any]:
    lock_path = operation_lock_path(output_dir)
    if not lock_path.exists():
        return {
            "schema_version": "northroot.steward.operation-lock-status.v0",
            "locked": False,
            "resume_required": False,
            "lock_path": str(lock_path),
            "lock": None,
            "error": None,
        }
    try:
        lock = model.load_json(lock_path)
        error = None
    except (OSError, ValueError, json.JSONDecodeError) as err:
        lock = None
        error = str(err)
    return {
        "schema_version": "northroot.steward.operation-lock-status.v0",
        "locked": True,
        "resume_required": True,
        "lock_path": str(lock_path),
        "lock": lock,
        "error": error,
    }


def _unreadable_operation_lock(lock_path: Path, error: str) -> dict[str, Any]:
    return {
        "schema_version": "northroot.steward.operation-lock.v0",
        "unreadable": True,
        "lock_path": str(lock_path),
        "lock_sha256": _file_sha256(lock_path) if lock_path.is_file() else None,
        "error": error,
        "failure_policy": "fail-closed-record-summary-before-retry",
        "resume_hint": "recover-operation will record and clear this unreadable lock before retry",
    }


def _dogfood_allowed_operations(output_dir: Path) -> list[dict[str, Any]]:
    del output_dir
    return [
        {
            "name": contract["name"],
            "command": " ".join(contract["argv_template"]),
            "mutates_backup_repository": False,
            "writes_run_summary": False,
            "requires_preflight": False,
            "governed_by_default_dogfood_policy": True,
        }
        for contract in _agent_workflow_operation_contracts()
    ]


def registry_topology_gate(
    registry_state: Path | None,
    *,
    operation: str,
    project_id: str | None,
    object_id: str | None = None,
) -> dict[str, Any] | None:
    if registry_state is None or not project_id or operation not in REGISTRY_TOPOLOGY_REQUIRED_OPERATIONS:
        return None
    topology = registry.registry_topology_report(
        registry_state,
        project_id=project_id,
        public_safe=True,
    )
    if topology["ready"]:
        return None
    return {
        "schema_version": "northroot.steward.registry-topology-gate.v0",
        "operation": operation,
        "registry_state": str(registry_state),
        "project_id": project_id,
        "object_id": object_id,
        "allowed": False,
        "decision": topology["decision"],
        "reason": topology.get("reason") or "registry topology is not ready for project execution",
        "topology": topology,
    }


def schedule_registry_gate(
    registry_state: Path | None,
    *,
    operation: str,
    project_id: str | None,
    object_id: str | None = None,
) -> dict[str, Any] | None:
    if registry_state is None and project_id is None and object_id is None:
        return None
    if registry_state is None:
        return {
            "schema_version": "northroot.steward.schedule-authorization.v0",
            "operation": operation,
            "allowed": False,
            "decision": "missing-registry-state",
            "reason": "registry_state is required when project_id or object_id is supplied",
        }
    if not project_id:
        return {
            "schema_version": "northroot.steward.schedule-authorization.v0",
            "operation": operation,
            "registry_state": str(registry_state),
            "object_id": object_id,
            "allowed": False,
            "decision": "missing-project-id",
            "reason": "project_id is required when registry_state is supplied",
        }
    authorization = authorize_operation(
        registry_state,
        operation=operation,
        project_id=project_id,
        object_id=object_id,
        public_safe=True,
    )
    if not authorization["allowed"]:
        return authorization
    topology_gate = registry_topology_gate(
        registry_state,
        operation=operation,
        project_id=project_id,
        object_id=object_id,
    )
    return topology_gate


def _schedule_registry_context_from_status(
    *,
    registry_state: Path | None,
    project_id: str | None,
    object_id: str | None,
    status: dict[str, Any],
) -> tuple[Path | None, str | None, str | None]:
    trusted_status = status if status.get("schedule_integrity", {}).get("ok") else {}
    if registry_state is None and trusted_status.get("registry_state"):
        registry_state = Path(str(trusted_status["registry_state"]))
    if project_id is None and trusted_status.get("project_id"):
        project_id = str(trusted_status["project_id"])
    if object_id is None and trusted_status.get("object_id"):
        object_id = str(trusted_status["object_id"])
    return registry_state, project_id, object_id


def _raise_for_schedule_registry_gate(
    *,
    operation: str,
    registry_state: Path | None,
    project_id: str | None,
    object_id: str | None,
    status: dict[str, Any],
) -> tuple[Path | None, str | None, str | None]:
    registry_state, project_id, object_id = _schedule_registry_context_from_status(
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
        status=status,
    )
    gate = schedule_registry_gate(
        registry_state,
        operation=operation,
        project_id=project_id,
        object_id=object_id,
    )
    if gate is not None:
        raise ScheduleRegistryGateError(operation, gate)
    return registry_state, project_id, object_id


def render_command_plan(
    output_dir: Path,
    *,
    operation: str,
    snapshot_id: str | None = None,
    target: str | None = None,
    execute: bool = False,
    scheduler: str | None = None,
    schedule_operation: str | None = None,
    every_minutes: int | None = None,
    runner_command: str | None = None,
    evidence: list[str] | None = None,
    source: str | None = None,
    detail: str | None = None,
    artifact_ref: str | None = None,
    json_path: Path | None = None,
    force: bool = False,
    use_recorded_evidence: bool = False,
    skip_preflight: bool = False,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
    agent_id: str = DEFAULT_DOGFOOD_AGENT_ID,
    branch: str | None = None,
    base_branch: str | None = None,
    commit_message: str | None = None,
    pr_title: str | None = None,
    pr_body: str | None = None,
    remote: str = "origin",
) -> dict[str, Any]:
    state = str(output_dir)
    missing_inputs: list[str] = []
    refused_reasons: list[str] = []
    warnings: list[str] = []
    evidence_items = list(evidence or [])
    argv: list[str] = ["nr", "steward"]
    requires_preflight = False
    writes_run_summary = False
    mutates_backup_repository = False
    delegated_platform_mutation = False
    authorization: dict[str, Any] | None = None
    registry_topology: dict[str, Any] | None = None
    agent_policy = default_dogfood_agent_delegation_policy()
    agent = _registered_agent(agent_policy, agent_id)
    agent_authorization: dict[str, Any] | None = None
    provenance: dict[str, Any] | None = None

    if operation not in COMMAND_PLAN_OPERATIONS:
        refused_reasons.append(f"unsupported steward operation: {operation}")
        argv = []
    elif operation in {"status", "preflight", "capabilities"}:
        argv.extend([operation, "--state", state])
    elif operation in {"verify-state", "report"}:
        argv.extend([operation, "--state", state])
        _append_snapshot(argv, snapshot_id)
    elif operation in {"run", "verify"}:
        argv.extend([operation, "--state", state])
        _append_snapshot(argv, snapshot_id)
        if registry_state is not None:
            argv.extend(["--registry-state", str(registry_state)])
        if project_id:
            argv.extend(["--project-id", project_id])
        if object_id:
            argv.extend(["--object-id", object_id])
        if execute:
            argv.append("--execute")
        requires_preflight = True
        writes_run_summary = True
        mutates_backup_repository = operation == "run" and execute
        if operation == "run" and not snapshot_id:
            warnings.append("snapshot_id is recommended when run evidence should be used for retention")
        if operation == "verify" and not snapshot_id:
            warnings.append("snapshot_id is required for verify evidence to satisfy retention")
    elif operation == "restore":
        argv.extend(["restore", "--state", state])
        if registry_state is not None:
            argv.extend(["--registry-state", str(registry_state)])
        if project_id:
            argv.extend(["--project-id", project_id])
        if object_id:
            argv.extend(["--object-id", object_id])
        if not snapshot_id:
            missing_inputs.append("snapshot_id")
        else:
            argv.extend(["--snapshot-id", snapshot_id])
        if not target:
            missing_inputs.append("target")
        else:
            argv.extend(["--target", target])
        if execute:
            argv.append("--execute")
        requires_preflight = True
        writes_run_summary = True
    elif operation == "restore-drill":
        argv.extend(["restore-drill", "--state", state])
        if registry_state is not None:
            argv.extend(["--registry-state", str(registry_state)])
        if project_id:
            argv.extend(["--project-id", project_id])
        if object_id:
            argv.extend(["--object-id", object_id])
        if target:
            argv.extend(["--target", target])
        _append_snapshot(argv, snapshot_id)
        if execute:
            argv.append("--execute")
        requires_preflight = True
        writes_run_summary = True
        if not snapshot_id:
            warnings.append("snapshot_id is required for restore-drill evidence to satisfy retention")
    elif operation == "schedule.create":
        argv.extend(["schedule", "create", "--state", state])
        _append_registry_context_args(
            argv,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if scheduler not in {"launchd", "systemd"}:
            missing_inputs.append("scheduler")
        else:
            argv.extend(["--scheduler", scheduler])
        if schedule_operation not in SCHEDULE_OPERATIONS:
            missing_inputs.append("schedule_operation")
        else:
            argv.extend(["--operation", schedule_operation])
        if every_minutes is None or every_minutes <= 0:
            missing_inputs.append("every_minutes")
        else:
            argv.extend(["--every-minutes", str(every_minutes)])
        if runner_command:
            argv.extend(["--runner-command", runner_command])
    elif operation == "schedule.status":
        argv.extend(["schedule", "status", "--state", state])
        _append_registry_context_args(
            argv,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
    elif operation == "schedule.install":
        argv.extend(["schedule", "install", "--state", state])
        _append_registry_context_args(
            argv,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if execute:
            argv.append("--execute")
            delegated_platform_mutation = True
        if skip_preflight:
            argv.append("--skip-preflight")
            warnings.append("skip_preflight is an operator override and should not be used by autonomous agents")
        requires_preflight = not skip_preflight
    elif operation == "schedule.uninstall":
        argv.extend(["schedule", "uninstall", "--state", state])
        _append_registry_context_args(
            argv,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if execute:
            argv.append("--execute")
            delegated_platform_mutation = True
    elif operation == "schedule.delete":
        argv.extend(["schedule", "delete", "--state", state])
        _append_registry_context_args(
            argv,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if force:
            argv.append("--force")
            warnings.append("force is operator cleanup for stale generated files")
    elif operation == "retention.evaluate":
        argv.extend(["retention", "evaluate", "--state", state])
        if not snapshot_id:
            missing_inputs.append("snapshot_id")
        else:
            argv.extend(["--snapshot-id", snapshot_id])
        for evidence_item in evidence_items:
            argv.extend(["--evidence", evidence_item])
        if use_recorded_evidence:
            argv.append("--use-recorded-evidence")
    elif operation == "evidence.report":
        argv.extend(["evidence", "report", "--state", state])
        _append_snapshot(argv, snapshot_id)
    elif operation == "evidence.record":
        argv.extend(["evidence", "record", "--state", state])
        if not snapshot_id:
            missing_inputs.append("snapshot_id")
        else:
            argv.extend(["--snapshot-id", snapshot_id])
        invalid_evidence = sorted(set(evidence_items) - model.EXTERNAL_RETENTION_EVIDENCE)
        if invalid_evidence:
            refused_reasons.append(f"unsupported external evidence: {', '.join(invalid_evidence)}")
        if not evidence_items:
            missing_inputs.append("evidence")
        for evidence_item in evidence_items:
            argv.extend(["--evidence", evidence_item])
        if not source:
            missing_inputs.append("source")
        else:
            argv.extend(["--source", source])
        if detail:
            argv.extend(["--detail", detail])
        if artifact_ref:
            argv.extend(["--artifact-ref", artifact_ref])
        writes_run_summary = True
    elif operation == "offsite.report":
        argv.extend(["offsite", "report", "--state", state])
        if not snapshot_id:
            missing_inputs.append("snapshot_id")
        else:
            argv.extend(["--snapshot-id", snapshot_id])
    elif operation == "import-legacy-runs":
        argv.extend(["import-legacy-runs", "--state", state])
        if json_path is None:
            missing_inputs.append("json")
        else:
            argv.extend(["--json", str(json_path), "--public-safe"])
        writes_run_summary = True
        lock_status = _operation_lock_status(output_dir)
        if lock_status["locked"]:
            refused_reasons.append("steward operation lock requires recover-operation before legacy run import")
    elif operation in model.AGENT_DELEGATION_OPERATIONS:
        argv = []
        if agent is None:
            refused_reasons.append(f"agent is not registered in default dogfood policy: {agent_id}")
        else:
            allowed_operations = set(str(item) for item in agent.get("allowed_operations", []))
            if operation not in allowed_operations:
                refused_reasons.append(f"agent operation not allowed by default dogfood policy: {operation}")
        branch_required = operation in {
            "branch.checkout",
            "branch.create",
            "commit.create",
            "commit.verify",
            "push.branch",
            "pr.draft.open",
            "pr.draft.update",
            "pr.comment.follow-up",
            "pr.check.verify",
        }
        if branch_required and not branch:
            missing_inputs.append("branch")
        if branch and agent is not None and not _agent_branch_allowed(branch, agent=agent, policy=agent_policy):
            refused_reasons.append(f"branch is outside delegated agent prefixes or protected: {branch}")
        base = base_branch or str(agent_policy["scope"]["protected_base_branch"])
        body = pr_body or "Draft PR opened by registered agent under dogfood delegation policy."
        verification = detail or "verification pending"
        if branch and agent is not None:
            trailers = _agent_commit_trailers(agent, branch=branch, verification=verification)
            provenance = {
                "policy_id": agent_policy["policy_id"],
                "agent_id": agent_id,
                "author_identity": agent["required_metadata"]["author_identity"],
                "committer_identity": agent["required_metadata"]["committer_identity"],
                "coauthorship_policy": agent["required_metadata"]["coauthorship_policy"],
                "required_commit_trailers": trailers,
            }
            agent_authorization = {
                "allowed": not refused_reasons,
                "policy_id": agent_policy["policy_id"],
                "agent_id": agent_id,
                "operation": operation,
                "branch": branch,
            }
        if operation == "branch.create":
            if branch:
                argv = ["git", "switch", "-c", branch, base]
        elif operation == "branch.checkout":
            if branch:
                argv = ["git", "switch", branch]
        elif operation == "commit.create":
            if not commit_message:
                missing_inputs.append("commit_message")
            if branch and commit_message and provenance:
                argv = ["git", "commit", "--message", commit_message]
                for key, value in provenance["required_commit_trailers"].items():
                    argv.extend(["--trailer", f"{key}={value}"])
        elif operation == "commit.verify":
            if branch:
                argv = ["git", "show", "-s", "--format=%H%n%an%n%ae%n%cn%n%ce%n%B", "HEAD"]
        elif operation == "push.branch":
            if branch:
                argv = ["git", "push", "-u", remote, branch]
        elif operation == "pr.draft.open":
            if not pr_title:
                missing_inputs.append("pr_title")
            if branch and pr_title:
                argv = [
                    "gh",
                    "pr",
                    "create",
                    "--draft",
                    "--base",
                    base,
                    "--head",
                    branch,
                    "--title",
                    pr_title,
                    "--body",
                    body,
                ]
        elif operation == "pr.draft.update":
            if not pr_title and not pr_body:
                missing_inputs.append("pr_title_or_pr_body")
            if branch and (pr_title or pr_body):
                argv = ["gh", "pr", "edit", branch]
                if pr_title:
                    argv.extend(["--title", pr_title])
                if pr_body:
                    argv.extend(["--body", pr_body])
        elif operation == "pr.comment.follow-up":
            if not detail:
                missing_inputs.append("detail")
            if branch and detail:
                argv = ["gh", "pr", "comment", branch, "--body", detail]
        elif operation == "pr.check.verify":
            if branch:
                argv = ["gh", "pr", "checks", branch]

    if registry_state is not None:
        if not project_id:
            missing_inputs.append("project_id")
        elif operation in model.SERVICE_PERMISSION_OPERATIONS:
            authorization = authorize_operation(
                registry_state,
                operation=operation,
                project_id=project_id,
                object_id=object_id,
                public_safe=True,
            )
            if not authorization["allowed"]:
                refused_reasons.append(f"registry authorization denied: {authorization['decision']}")
            registry_topology = registry_topology_gate(
                registry_state,
                operation=operation,
                project_id=project_id,
                object_id=object_id,
            )
            if registry_topology is not None:
                refused_reasons.append(f"registry topology denied: {registry_topology['decision']}")

    preflight_ready: bool | None = None
    preflight_failed_codes: list[str] = []
    if execute and requires_preflight and operation not in {"schedule.install"}:
        preflight = render_preflight(
            output_dir,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        preflight_ready = bool(preflight["ready"])
        preflight_failed_codes = [
            str(check["code"])
            for check in preflight.get("checks", [])
            if isinstance(check, dict) and check.get("status") != "ok" and check.get("code")
        ]
        if not preflight_ready:
            refused_reasons.append("preflight is not ready for execute")
    elif execute and operation == "schedule.install" and requires_preflight:
        preflight = render_preflight(
            output_dir,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        preflight_ready = bool(preflight["ready"])
        preflight_failed_codes = [
            str(check["code"])
            for check in preflight.get("checks", [])
            if isinstance(check, dict) and check.get("status") != "ok" and check.get("code")
        ]
        if not preflight_ready:
            refused_reasons.append("preflight is not ready for schedule install")
    if execute and operation in OPERATIONS and _operation_lock_status(output_dir)["locked"]:
        refused_reasons.append("steward operation lock requires recover-operation before execute")

    ok = not missing_inputs and not refused_reasons
    return {
        "schema_version": "northroot.steward.command-plan.v0",
        "operation": operation,
        "ok": ok,
        "argv": argv if argv else None,
        "argv_style": "argv",
        "shell_required": False,
        "execute_requested": execute,
        "force_requested": force,
        "skip_preflight_requested": skip_preflight,
        "requires_preflight": requires_preflight,
        "preflight_ready": preflight_ready,
        "preflight_failed_codes": preflight_failed_codes,
        "missing_inputs": sorted(set(missing_inputs)),
        "refused_reasons": refused_reasons,
        "warnings": warnings,
        "authorization": authorization,
        "registry_topology": registry_topology,
        "agent_authorization": agent_authorization,
        "agent_provenance": provenance,
        "default_agent_policy_id": agent_policy["policy_id"],
        "side_effects": {
            "writes_run_summary": writes_run_summary,
            "mutates_backup_repository": mutates_backup_repository,
            "delegates_platform_scheduler_mutation": delegated_platform_mutation,
            "mutates_git_repository": operation in {"branch.checkout", "branch.create", "commit.create"},
            "pushes_git_remote": operation == "push.branch",
            "opens_or_updates_draft_pr": operation in {"pr.draft.open", "pr.draft.update", "pr.comment.follow-up"},
        },
        "agent_guidance": {
            "execute_requires_explicit_flag": True,
            "bind_placeholders_before_invocation": True,
            "do_not_shell_join_argv": True,
            "do_not_read_or_log_secret_values": True,
            "registered_agents_use_default_dogfood_policy": True,
            "default_dogfood_policy_selected_without_policy_file": True,
            "final_review_clearance_required_before_ready_pr_or_merge": True,
        },
    }


def render_capabilities(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
    status = render_status(output_dir)
    return {
        "schema_version": "northroot.steward.capabilities.v0",
        "profile_name": installation["profile_name"],
        "execution_model": {
            "kind": "delegated",
            "delegated_tool": installation["delegated_tool"],
            "custom_backup_engine": False,
            "preflight_required_before_execute": True,
            "installation_manifest_integrity_checked_by_preflight": True,
            "generated_artifact_integrity_checked_by_preflight": True,
        },
        "restore_verification": {
            "requires_delegated_restore_success": True,
            "requires_observed_restored_files": True,
            "records_manifest_sha256": True,
            "restore_operation_requires_explicit_snapshot_id": True,
            "restore_operation_requires_explicit_target": True,
            "actual_restore_does_not_satisfy_restore_drill_retention_evidence": True,
        },
        "evidence_scope": {
            "snapshot_id_supported_on_operations": [
                "run",
                "verify",
                "restore",
                "restore-drill",
                "verify-state",
                "evidence.record",
            ],
            "retention_recorded_evidence_is_snapshot_bound": True,
            "snapshot_filtered_reports_ignore_unscoped_operation_evidence": True,
        },
        "agent_contract": {
            "schema_version": "northroot.steward.agent-contract.v0",
            "default_dogfood_policy": default_dogfood_agent_delegation_policy(),
            "default_policy_resolution": {
                "registered_agents_use_default_dogfood_policy": True,
                "policy_file_required_for_dogfood_agent_workflow": False,
                "registered_agent_operations": sorted(model.AGENT_DELEGATION_OPERATIONS),
            },
            "invocation": {
                "argument_style": "argv",
                "shell_required": False,
                "template_placeholders_must_be_bound": True,
                "execute_requires_explicit_flag": True,
                "command_plan_schema": "northroot.steward.command-plan.v0",
                "command_plan_command": f"{DEFAULT_RUNNER_COMMAND} command-plan --state {output_dir} --operation <operation>",
            },
            "exit_semantics": {
                "zero": "JSON output satisfied the command contract",
                "nonzero": "JSON output describes a blocked or failed command and must not be treated as success",
            },
            "secret_handling": {
                "secret_values_returned": False,
                "runtime_env_materialized_only_for_child_process": True,
                "agents_must_not_read_or_log_secret_values": True,
            },
        },
        "operation_contracts": _agent_operation_contracts(output_dir),
        "allowed_operations": [
            {
                "name": "status",
                "command": status["commands"]["status"],
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "preflight",
                "command": status["commands"]["preflight"],
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "capabilities",
                "command": f"{DEFAULT_RUNNER_COMMAND} capabilities --state {output_dir}",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "command-plan",
                "command": f"{DEFAULT_RUNNER_COMMAND} command-plan --state {output_dir} --operation <operation>",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
                "returns_argv": True,
                "allowed_operations": sorted(COMMAND_PLAN_OPERATIONS),
            },
            {
                "name": "verify-state",
                "command": f"{DEFAULT_RUNNER_COMMAND} verify-state --state {output_dir}",
                "snapshot_command": f"{DEFAULT_RUNNER_COMMAND} verify-state --state {output_dir} --snapshot-id <snapshot-id>",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "report",
                "command": f"{DEFAULT_RUNNER_COMMAND} report --state {output_dir}",
                "snapshot_command": f"{DEFAULT_RUNNER_COMMAND} report --state {output_dir} --snapshot-id <snapshot-id>",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "run",
                "command": f"{DEFAULT_RUNNER_COMMAND} run --state {output_dir} --snapshot-id <snapshot-id>",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} run --state {output_dir} --snapshot-id <snapshot-id> --execute",
                "mutates_backup_repository": True,
                "writes_run_summary": True,
                "requires_preflight": True,
                "snapshot_id_recommended_for_retention_evidence": True,
            },
            {
                "name": "verify",
                "command": f"{DEFAULT_RUNNER_COMMAND} verify --state {output_dir} --snapshot-id <snapshot-id>",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} verify --state {output_dir} --snapshot-id <snapshot-id> --execute",
                "mutates_backup_repository": False,
                "writes_run_summary": True,
                "requires_preflight": True,
                "snapshot_id_required_for_retention_evidence": True,
            },
            {
                "name": "restore",
                "command": f"{DEFAULT_RUNNER_COMMAND} restore --state {output_dir} --snapshot-id <snapshot-id> --target <target>",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} restore --state {output_dir} --snapshot-id <snapshot-id> --target <target> --execute",
                "mutates_backup_repository": False,
                "writes_run_summary": True,
                "requires_preflight": True,
                "snapshot_id_required": True,
                "target_required": True,
                "satisfies_retention_evidence": [],
            },
            {
                "name": "restore-drill",
                "command": f"{DEFAULT_RUNNER_COMMAND} restore-drill --state {output_dir} --snapshot-id <snapshot-id>",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} restore-drill --state {output_dir} --snapshot-id <snapshot-id> --execute",
                "mutates_backup_repository": False,
                "writes_run_summary": True,
                "requires_preflight": True,
                "snapshot_id_required_for_retention_evidence": True,
            },
            {
                "name": "schedule.create",
                "command": f"{DEFAULT_RUNNER_COMMAND} schedule create --state {output_dir} --scheduler <launchd|systemd> --operation <run|verify|restore-drill> --every-minutes <minutes>",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "schedule.status",
                "command": f"{DEFAULT_RUNNER_COMMAND} schedule status --state {output_dir}",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "schedule.install",
                "command": f"{DEFAULT_RUNNER_COMMAND} schedule install --state {output_dir}",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} schedule install --state {output_dir} --execute",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": True,
            },
            {
                "name": "schedule.uninstall",
                "command": f"{DEFAULT_RUNNER_COMMAND} schedule uninstall --state {output_dir}",
                "execute_command": f"{DEFAULT_RUNNER_COMMAND} schedule uninstall --state {output_dir} --execute",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "schedule.delete",
                "command": f"{DEFAULT_RUNNER_COMMAND} schedule delete --state {output_dir}",
                "force_command": f"{DEFAULT_RUNNER_COMMAND} schedule delete --state {output_dir} --force",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
                "blocked_when_installed": True,
                "operator_force_required_when_installed": True,
            },
            {
                "name": "retention.evaluate",
                "command": f"{DEFAULT_RUNNER_COMMAND} retention evaluate --state {output_dir}",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "evidence.report",
                "command": f"{DEFAULT_RUNNER_COMMAND} evidence report --state {output_dir}",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
            },
            {
                "name": "offsite.report",
                "command": f"{DEFAULT_RUNNER_COMMAND} offsite report --state {output_dir} --snapshot-id <snapshot-id>",
                "mutates_backup_repository": False,
                "writes_run_summary": False,
                "requires_preflight": False,
                "snapshot_id_required": True,
            },
            {
                "name": "evidence.record",
                "command": f"{DEFAULT_RUNNER_COMMAND} evidence record --state {output_dir} --snapshot-id <snapshot-id> --evidence verified_offsite_copy --source <external-tool-or-monitor>",
                "mutates_backup_repository": False,
                "writes_run_summary": True,
                "requires_preflight": False,
            },
            {
                "name": "import-legacy-runs",
                "command": f"{DEFAULT_RUNNER_COMMAND} import-legacy-runs --state {output_dir} --json <legacy-run-import.json> --public-safe",
                "mutates_backup_repository": False,
                "writes_run_summary": True,
                "requires_preflight": False,
                "uses_operation_lock": True,
            },
        ] + _dogfood_allowed_operations(output_dir),
        "prohibited_operations": [
            "direct restic/resticprofile shell construction outside this manifest",
            "forget/prune without a retention decision that allows it",
            "reading, printing, storing, or echoing resolved secret values",
            "editing generated resticprofile config as the source of truth",
            "performing recovery restores outside the bounded restore command",
            "assuming additional destinations were copied because the primary backup ran",
            "recording repository-check or restore-drill evidence through external evidence import",
            "readying or merging agent draft PRs without final review clearance",
        ],
        "contracts": {
            "inventory_schema": model.INVENTORY_SCHEMA,
            "policy_schema": model.POLICY_SCHEMA,
            "snapshot_plan_schema": model.SNAPSHOT_PLAN_SCHEMA,
            "repository_bindings_schema": model.REPOSITORY_BINDINGS_SCHEMA,
            "secret_bindings_schema": model.SECRET_BINDINGS_SCHEMA,
            "verification_result_schema": model.VERIFICATION_RESULT_SCHEMA,
            "run_summary_schema": model.RUN_SUMMARY_SCHEMA,
            "run_summary_index_schema": RUN_SUMMARY_INDEX_SCHEMA,
            "retention_decision_schema": model.RETENTION_DECISION_SCHEMA,
            "agent_delegation_policy_schema": model.AGENT_DELEGATION_POLICY_SCHEMA,
        },
        "secret_binding_providers": sorted(model.SECRET_BINDING_PROVIDERS),
        "runtime_environment_providers": sorted(model.RUNTIME_ENV_PROVIDERS),
        "external_recordable_evidence": sorted(model.EXTERNAL_RETENTION_EVIDENCE),
        "destination_execution": status["destination_execution"],
        "offsite_execution_model": {
            "kind": "external-delegated",
            "recordable_evidence": ["verified_offsite_copy"],
            "copy_execution_owned_by": "private-deployment-tooling",
            "suggested_tools": ["rclone", "restic copy", "external monitor"],
        },
        "schedule_lifecycle": {
            "install_delegates_to_platform_scheduler": True,
            "scheduled_runner_command_checked_by_preflight": True,
            "schedule_manifest_integrity_checked_by_preflight": True,
            "generated_schedule_integrity_checked_by_preflight": True,
            "uninstall_required_before_delete_when_installed": True,
            "delete_force_is_operator_cleanup_only": True,
        },
        "verification_required": status["verification_required"],
        "retention_prune_requires": status["retention_prune_requires"],
        "schedule": status["schedule"],
    }


def _render_registry_context(
    registry_state: Path | None,
    *,
    project_id: str | None,
    object_id: str | None,
    operation: str,
) -> dict[str, Any] | None:
    if registry_state is None:
        return None
    status = registry.registry_status(registry_state, public_safe=True)
    authorization = None
    topology = None
    missing_project_id = bool(object_id and not project_id)
    if project_id is not None:
        authorization = authorize_operation(
            registry_state,
            operation=operation,
            project_id=project_id,
            object_id=object_id,
            public_safe=True,
        )
        topology = registry.registry_topology_report(
            registry_state,
            project_id=project_id,
            public_safe=True,
        )
    topology_ready = topology is None or bool(topology["ready"])
    ready = bool(
        status["ready"]
        and not missing_project_id
        and (authorization is None or authorization["allowed"])
        and topology_ready
    )
    if not status["ready"]:
        decision = "invalid-registry"
    elif missing_project_id:
        decision = "missing-project-id"
    elif authorization is not None and not authorization["allowed"]:
        decision = str(authorization["decision"])
    elif topology is not None and not topology["ready"]:
        decision = str(topology["decision"])
    elif authorization is not None:
        decision = str(authorization["decision"])
    else:
        decision = "registry-ready"
    return {
        "schema_version": "northroot.steward.registry-context.v0",
        "operation": operation,
        "ready": ready,
        "decision": decision,
        "registry_state": str(registry_state),
        "project_id": project_id,
        "object_id": object_id,
        "registry_ready": bool(status["ready"]),
        "protected_state_ok": bool(status.get("protected_state_ok")),
        "resume_required": bool(status.get("resume_required")),
        "status": status,
        "authorization": authorization,
        "topology": topology,
        "topology_ready": topology_ready if topology is not None else None,
    }


def render_state_verification(
    output_dir: Path,
    *,
    snapshot_id: str | None = None,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    status = render_status(output_dir)
    preflight = render_preflight(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    capabilities = render_capabilities(output_dir)
    evidence_report = render_evidence_report(output_dir, snapshot_id=snapshot_id)
    run_summary_integrity = evidence_report["run_summary_integrity"]
    operation_lock = _operation_lock_status(output_dir)
    registry_context = _render_registry_context(
        registry_state,
        project_id=project_id,
        object_id=object_id,
        operation="verify-state",
    )
    schedule = preflight.get("schedule", status["schedule"])
    failed_codes = sorted(
        str(check["code"])
        for check in preflight["checks"]
        if check.get("status") != "ok" and check.get("code")
    )
    generated_artifact_failures = [
        code for code in failed_codes if code in {"missing_generated_artifact", "generated_artifact_drift"}
    ]
    schedule_artifact_failures = [
        code for code in failed_codes if code in {"missing_schedule_generated_artifact", "schedule_generated_artifact_drift"}
    ]
    missing_schedule_runner = "missing_scheduled_runner_command" in failed_codes
    retention_decision = (
        evaluate_retention(
            output_dir,
            snapshot_id=snapshot_id,
            available_evidence=[],
            use_recorded_evidence=True,
        )
        if snapshot_id
        else None
    )
    capability_names = {
        str(operation.get("name"))
        for operation in capabilities.get("allowed_operations", [])
        if isinstance(operation, dict)
    }
    required_capabilities = {
        "status",
        "preflight",
        "capabilities",
        "command-plan",
        "verify-state",
        "report",
        "run",
        "verify",
        "restore-drill",
        "restore",
        "schedule.install",
        "schedule.status",
        "retention.evaluate",
        "evidence.report",
        "offsite.report",
        "evidence.record",
        "import-legacy-runs",
    }
    missing_capabilities = sorted(required_capabilities - capability_names)
    checks = [
        _check("profile_status", True, f"loaded steward state at {output_dir}"),
        _check(
            "capability_manifest",
            not missing_capabilities,
            "required steward capabilities are declared"
            if not missing_capabilities
            else f"missing steward capabilities: {', '.join(missing_capabilities)}",
            code=None if not missing_capabilities else "missing_steward_capability",
        ),
        _check(
            "preflight",
            bool(preflight["ready"]),
            "preflight is ready" if preflight["ready"] else f"preflight failed: {', '.join(failed_codes)}",
            code=None if preflight["ready"] else "preflight_not_ready",
        ),
        _check(
            "generated_artifact_integrity",
            not generated_artifact_failures,
            "generated custody artifacts match installation metadata"
            if not generated_artifact_failures
            else f"generated custody artifacts failed: {', '.join(generated_artifact_failures)}",
            code=None if not generated_artifact_failures else "generated_artifact_integrity_failed",
        ),
        _check(
            "schedule_generated_artifact_integrity",
            not schedule_artifact_failures,
            "schedule artifacts match metadata or no schedule is configured"
            if not schedule_artifact_failures
            else f"schedule artifacts failed: {', '.join(schedule_artifact_failures)}",
            code=None if not schedule_artifact_failures else "schedule_artifact_integrity_failed",
        ),
        _check(
            "scheduled_runner_command",
            not missing_schedule_runner,
            "scheduled runner command is available or no schedule is configured"
            if not missing_schedule_runner
            else "scheduled runner command is not available",
            code=None if not missing_schedule_runner else "missing_scheduled_runner_command",
        ),
        _check(
            "steward_operation_lock",
            not operation_lock["locked"],
            "no unresolved steward operation lock"
            if not operation_lock["locked"]
            else "unresolved steward operation lock requires recover-operation",
            code=None if not operation_lock["locked"] else "steward_operation_lock_present",
        ),
        _check(
            "run_summary_integrity",
            bool(run_summary_integrity["ok"]),
            "run summaries match the steward digest index"
            if run_summary_integrity["ok"]
            else "run summaries failed steward digest index verification",
            code=None if run_summary_integrity["ok"] else "run_summary_integrity_failed",
        ),
    ]
    if registry_context is not None:
        registry_code = None
        if not registry_context["registry_ready"]:
            registry_code = "registry_not_ready"
        elif registry_context["decision"] == "missing-project-id":
            registry_code = "registry_project_required"
        elif registry_context["topology_ready"] is False:
            registry_code = "registry_topology_not_ready"
        elif not registry_context["ready"]:
            registry_code = "registry_authorization_failed"
        checks.append(
            _check(
                "registry_context",
                bool(registry_context["ready"]),
                "service registry is ready for verify-state"
                if registry_context["ready"]
                else f"service registry proof failed: {registry_context['decision']}",
                code=registry_code,
            )
        )
    if retention_decision is not None:
        checks.append(
            _check(
                "snapshot_retention_evidence",
                bool(retention_decision["allowed"]),
                "snapshot has required retention evidence"
                if retention_decision["allowed"]
                else f"snapshot missing retention evidence: {', '.join(retention_decision['missing_evidence'])}",
                code=None if retention_decision["allowed"] else "missing_snapshot_retention_evidence",
            )
        )
    ready = all(check["status"] == "ok" for check in checks)
    registry_allows_execution = registry_context is None or bool(registry_context["ready"])
    return {
        "schema_version": "northroot.steward.state-verification.v0",
        "profile_name": status["profile_name"],
        "ready": ready,
        "safe_to_execute": bool(preflight["ready"] and not operation_lock["locked"] and registry_allows_execution),
        "safe_to_install_schedule": bool(
            preflight["ready"] and schedule.get("configured") and not operation_lock["locked"] and registry_allows_execution
        ),
        "preflight_ready": bool(preflight["ready"]),
        "preflight_failed_codes": failed_codes,
        "operation_lock": operation_lock,
        "operation_resume_required": bool(operation_lock["resume_required"]),
        "registry_context": registry_context,
        "registry_ready": registry_context["ready"] if registry_context is not None else None,
        "schedule_configured": bool(schedule.get("configured")),
        "schedule_installed": bool(schedule.get("installed")),
        "latest_run_summary_path": status["latest_run_summary_path"],
        "snapshot_id": snapshot_id,
        "retention_evidence_ready": retention_decision["allowed"] if retention_decision is not None else None,
        "retention_decision": retention_decision,
        "evidence_report": evidence_report,
        "run_summary_integrity": run_summary_integrity,
        "checks": checks,
    }


def render_report(
    output_dir: Path,
    *,
    snapshot_id: str | None = None,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    status = render_status(output_dir)
    preflight = render_preflight(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    evidence_report = render_evidence_report(output_dir, snapshot_id=snapshot_id)
    operation_lock = _operation_lock_status(output_dir)
    registry_context = _render_registry_context(
        registry_state,
        project_id=project_id,
        object_id=object_id,
        operation="report",
    )
    offsite_report = render_offsite_report(output_dir, snapshot_id=snapshot_id) if snapshot_id else None
    retention_decision = (
        evaluate_retention(
            output_dir,
            snapshot_id=snapshot_id,
            available_evidence=[],
            use_recorded_evidence=True,
        )
        if snapshot_id
        else None
    )
    recommended_actions: list[str] = []
    failed_preflight = [
        str(check["code"])
        for check in preflight.get("checks", [])
        if isinstance(check, dict) and check.get("status") != "ok" and check.get("code")
    ]
    if failed_preflight:
        recommended_actions.append("fix preflight failures before executing scheduled custody operations")
    if operation_lock["locked"]:
        recommended_actions.append("run steward recover-operation before retrying steward execution")
    if registry_context is not None and not registry_context["ready"]:
        if registry_context["resume_required"]:
            recommended_actions.append("recover the service registry before relying on steward policy authorization")
        elif not registry_context["registry_ready"]:
            recommended_actions.append("repair service registry integrity before relying on steward policy authorization")
        elif registry_context["decision"] == "missing-project-id":
            recommended_actions.append("provide project_id when checking object-scoped steward registry authorization")
        elif registry_context["topology_ready"] is False:
            recommended_actions.append("repair service registry destination topology before relying on steward project execution")
        else:
            recommended_actions.append("update service registry permissions or request human clearance before steward operations")
    if retention_decision is not None and retention_decision.get("missing_evidence"):
        missing = ", ".join(str(item) for item in retention_decision["missing_evidence"])
        recommended_actions.append(f"collect required snapshot evidence before prune/offload: {missing}")
    if offsite_report is not None and offsite_report.get("missing_evidence"):
        recommended_actions.append("record verified_offsite_copy evidence after delegated offsite copy verification")
    if not recommended_actions:
        recommended_actions.append("no custody action required by this report")
    return {
        "schema_version": "northroot.steward.report.v0",
        "profile_name": status["profile_name"],
        "snapshot_id": snapshot_id,
        "generated_at": _utc_stamp(),
        "execution_model": status["execution_mode"],
        "custom_backup_engine": status["custom_backup_engine"],
        "delegated_tool": status["delegated_tool"],
        "preflight_ready": bool(preflight["ready"]),
        "operation_lock": operation_lock,
        "operation_resume_required": bool(operation_lock["resume_required"]),
        "registry_context": registry_context,
        "registry_ready": registry_context["ready"] if registry_context is not None else None,
        "schedule": status["schedule"],
        "latest_run_summary_path": status["latest_run_summary_path"],
        "retention_evidence_ready": retention_decision["allowed"] if retention_decision is not None else None,
        "retention_decision": retention_decision,
        "evidence_report": evidence_report,
        "offsite_report": offsite_report,
        "status": status,
        "preflight": preflight,
        "recommended_actions": recommended_actions,
    }


def evaluate_retention(
    output_dir: Path,
    *,
    snapshot_id: str,
    available_evidence: list[str],
    use_recorded_evidence: bool = False,
) -> dict[str, Any]:
    plan = load_plan(output_dir)
    evidence = list(available_evidence)
    if use_recorded_evidence:
        recorded = render_evidence_report(output_dir, snapshot_id=snapshot_id)
        evidence.extend(str(item) for item in recorded.get("available_evidence", []))
    evidence = list(dict.fromkeys(evidence))
    required = list(plan.get("retention_prune_requires", []))
    missing = [item for item in required if item not in evidence]
    return {
        "schema_version": model.RETENTION_DECISION_SCHEMA,
        "policy_id": str(plan.get("policy_id", "unknown")),
        "workspace_id": str(plan.get("workspace_id", "unknown")),
        "snapshot_id": snapshot_id,
        "allowed": not missing,
        "required_evidence": required,
        "available_evidence": evidence,
        "missing_evidence": missing,
    }


def iter_run_summaries(output_dir: Path) -> list[Path]:
    summaries_dir = output_dir / "run-summaries"
    if not summaries_dir.is_dir():
        return []
    return sorted(
        (path for path in summaries_dir.glob("*.json") if path.name != RUN_SUMMARY_INDEX_FILENAME),
        key=lambda path: path.stat().st_mtime,
    )


def _run_summary_index_path(output_dir: Path) -> Path:
    return output_dir / "run-summaries" / RUN_SUMMARY_INDEX_FILENAME


def _load_run_summary_index(output_dir: Path) -> dict[str, Any]:
    index_path = _run_summary_index_path(output_dir)
    if not index_path.exists():
        return {
            "schema_version": RUN_SUMMARY_INDEX_SCHEMA,
            "updated_at": None,
            "entries": [],
        }
    index = model.load_json(index_path)
    if index.get("schema_version") != RUN_SUMMARY_INDEX_SCHEMA:
        raise ValueError(f"run summary index schema_version must be {RUN_SUMMARY_INDEX_SCHEMA}")
    if not isinstance(index.get("entries"), list):
        raise ValueError("run summary index entries must be a list")
    return index


def _write_run_summary_index(output_dir: Path, entries: list[dict[str, Any]]) -> None:
    payload = {
        "schema_version": RUN_SUMMARY_INDEX_SCHEMA,
        "updated_at": _utc_stamp(),
        "entries": sorted(entries, key=lambda entry: str(entry.get("run_id", ""))),
    }
    _atomic_write_json(_run_summary_index_path(output_dir), payload)


def _index_run_summary(output_dir: Path, summary_path: Path, *, run_id: str, sha256: str | None = None) -> None:
    index = _load_run_summary_index(output_dir)
    entries = [
        entry
        for entry in index.get("entries", [])
        if isinstance(entry, dict) and entry.get("path") != summary_path.name and entry.get("run_id") != run_id
    ]
    entries.append(
        {
            "run_id": run_id,
            "path": summary_path.name,
            "sha256": sha256 or _file_sha256(summary_path),
            "indexed_at": _utc_stamp(),
        }
    )
    _write_run_summary_index(output_dir, entries)


def _write_indexed_run_summary(output_dir: Path, summary: dict[str, Any]) -> Path:
    findings = model.validate_run_summary(summary)
    errors = [finding for finding in findings if finding.severity == "error"]
    if errors:
        detail = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"invalid run summary: {detail}")
    run_id = str(summary["run_id"])
    summary_path = _run_summary_path(output_dir, run_id)
    sha256 = _atomic_write_json(summary_path, summary)
    _index_run_summary(output_dir, summary_path, run_id=run_id, sha256=sha256)
    return summary_path


def render_run_summary_integrity(output_dir: Path) -> dict[str, Any]:
    summaries = iter_run_summaries(output_dir)
    index_path = _run_summary_index_path(output_dir)
    observations: list[dict[str, Any]] = []
    if not summaries and not index_path.exists():
        return {
            "schema_version": "northroot.steward.run-summary-integrity.v0",
            "ok": True,
            "index_path": str(index_path),
            "observations": [],
        }
    try:
        index = _load_run_summary_index(output_dir)
    except (OSError, ValueError, json.JSONDecodeError) as err:
        return {
            "schema_version": "northroot.steward.run-summary-integrity.v0",
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
                    "detail": "run summary index entries must include path strings",
                }
            )
            continue
        path_name = str(entry["path"])
        if path_name in entries_by_path:
            duplicate_paths.add(path_name)
        entries_by_path[path_name] = entry
    summary_names = {path.name for path in summaries}
    for path_name, entry in sorted(entries_by_path.items()):
        if path_name == RUN_SUMMARY_INDEX_FILENAME:
            observations.append(
                {
                    "summary_path": str(index_path),
                    "status": "invalid-index-entry",
                    "detail": "run summary index may not index itself",
                }
            )
            continue
        if path_name not in summary_names:
            observations.append(
                {
                    "summary_path": str(output_dir / "run-summaries" / path_name),
                    "run_id": entry.get("run_id"),
                    "status": "missing-summary",
                    "detail": "indexed run summary is missing",
                }
            )
    for summary_path in summaries:
        entry = entries_by_path.get(summary_path.name)
        if entry is None:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": None,
                    "status": "unindexed",
                    "detail": "run summary is not present in the steward digest index",
                }
            )
            continue
        if summary_path.name in duplicate_paths:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": entry.get("run_id"),
                    "status": "duplicate-index-entry",
                    "detail": "run summary has duplicate digest index entries",
                }
            )
            continue
        expected_sha = entry.get("sha256")
        if not isinstance(expected_sha, str) or not expected_sha:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": entry.get("run_id"),
                    "status": "invalid-index-entry",
                    "detail": "run summary index entry is missing sha256",
                }
            )
            continue
        try:
            actual_sha = _file_sha256(summary_path)
        except OSError as err:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": entry.get("run_id"),
                    "status": "unreadable",
                    "detail": str(err),
                }
            )
            continue
        if actual_sha != expected_sha:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": entry.get("run_id"),
                    "status": "digest-mismatch",
                    "detail": "run summary digest does not match the steward digest index",
                    "expected_sha256": expected_sha,
                    "actual_sha256": actual_sha,
                }
            )
            continue
        try:
            summary = model.load_json(summary_path)
        except (OSError, ValueError, json.JSONDecodeError) as err:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": entry.get("run_id"),
                    "status": "invalid-json",
                    "detail": str(err),
                }
            )
            continue
        findings = model.validate_run_summary(summary)
        errors = [finding for finding in findings if finding.severity == "error"]
        indexed_run_id = entry.get("run_id")
        summary_run_id = summary.get("run_id")
        if indexed_run_id != summary_run_id:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": summary_run_id,
                    "status": "run-id-mismatch",
                    "detail": "run summary run_id does not match the steward digest index",
                    "indexed_run_id": indexed_run_id,
                }
            )
        elif errors:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": summary_run_id,
                    "status": "invalid-summary",
                    "detail": "; ".join(f"{finding.path}: {finding.detail}" for finding in errors),
                }
            )
        else:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": summary_run_id,
                    "status": "ok",
                    "sha256": actual_sha,
                }
            )
    ok = all(observation.get("status") == "ok" for observation in observations)
    return {
        "schema_version": "northroot.steward.run-summary-integrity.v0",
        "ok": ok,
        "index_path": str(index_path),
        "observations": observations,
    }


def render_evidence_report(output_dir: Path, *, snapshot_id: str | None = None) -> dict[str, Any]:
    plan = load_plan(output_dir)
    integrity = render_run_summary_integrity(output_dir)
    integrity_by_path = {
        str(observation.get("summary_path")): observation
        for observation in integrity.get("observations", [])
        if isinstance(observation, dict) and observation.get("summary_path")
    }
    observations: list[dict[str, Any]] = []
    evidence: set[str] = set()
    for summary_path in iter_run_summaries(output_dir):
        integrity_observation = integrity_by_path.get(str(summary_path))
        if integrity_observation and integrity_observation.get("status") != "ok":
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "run_id": integrity_observation.get("run_id"),
                    "status": integrity_observation.get("status"),
                    "evidence": [],
                    "integrity_detail": integrity_observation.get("detail"),
                }
            )
            continue
        try:
            summary = model.load_json(summary_path)
        except (OSError, ValueError, json.JSONDecodeError) as err:
            observations.append(
                {
                    "summary_path": str(summary_path),
                    "status": "unreadable",
                    "evidence": [],
                    "error": str(err),
                }
            )
            continue
        produced: list[str] = []
        snapshot_result = summary.get("snapshot_result") if isinstance(summary.get("snapshot_result"), dict) else {}
        verification_result = (
            summary.get("verification_result") if isinstance(summary.get("verification_result"), dict) else {}
        )
        evidence_snapshot_id = snapshot_result.get("snapshot_id")
        snapshot_matches = snapshot_id is None or evidence_snapshot_id == snapshot_id
        if (
            snapshot_matches
            and summary.get("status") == "delegated-ok"
            and verification_result.get("repository_check") == "ok"
        ):
            produced.append("verified_snapshot")
        if (
            snapshot_matches
            and snapshot_result.get("operation") == "restore-drill"
            and verification_result.get("restore_verified") is True
        ):
            produced.append("restore_drill")
        external_evidence = verification_result.get("external_evidence", [])
        if summary.get("status") == "external-evidence-recorded" and isinstance(external_evidence, list):
            if snapshot_matches:
                for item in external_evidence:
                    if item in model.EXTERNAL_RETENTION_EVIDENCE:
                        produced.append(str(item))
        evidence.update(produced)
        observations.append(
            {
                "summary_path": str(summary_path),
                "run_id": summary.get("run_id"),
                "operation": snapshot_result.get("operation"),
                "status": summary.get("status"),
                "evidence": produced,
                "restore_observation": verification_result.get("restore_observation"),
                "external_evidence_source": snapshot_result.get("external_evidence_source"),
                "artifact_ref": snapshot_result.get("artifact_ref"),
                "snapshot_id": evidence_snapshot_id,
            }
        )
    required = list(plan.get("retention_prune_requires", []))
    available = sorted(evidence)
    return {
        "schema_version": "northroot.steward.evidence-report.v0",
        "workspace_id": str(plan.get("workspace_id", "unknown")),
        "policy_id": str(plan.get("policy_id", "unknown")),
        "snapshot_id": snapshot_id,
        "available_evidence": available,
        "required_evidence": required,
        "missing_evidence": [item for item in required if item not in available],
        "run_summary_integrity": integrity,
        "observations": observations,
    }


def render_offsite_report(output_dir: Path, *, snapshot_id: str) -> dict[str, Any]:
    if not snapshot_id.strip():
        raise ValueError("snapshot_id is required for offsite reports")
    installation = load_installation(output_dir)
    plan = load_plan(output_dir)
    destination_execution = _destination_execution(plan)
    evidence_report = render_evidence_report(output_dir, snapshot_id=snapshot_id)
    repository_bindings_path = installation.get("repository_bindings_path")
    repository_bindings_file = Path(str(repository_bindings_path)) if repository_bindings_path else None
    repository_bindings = (
        model.load_json(repository_bindings_file)
        if repository_bindings_file is not None and repository_bindings_file.is_file()
        else None
    )
    evidence_available = "verified_offsite_copy" in evidence_report["available_evidence"]
    destinations = []
    for destination in destination_execution["additional_destinations"]:
        repository_ref = str(destination.get("repository_ref"))
        destinations.append(
            {
                "id": str(destination.get("id")),
                "adapter": str(destination.get("adapter")),
                "repository_ref": repository_ref,
                "repository_target": model.repository_binding_target(repository_ref, repository_bindings),
                "handling": "external-copy-required",
                "copy_execution_model": "external-delegated",
                "required_evidence": ["verified_offsite_copy"],
                "evidence_available": evidence_available,
                "record_command": (
                    f"{DEFAULT_RUNNER_COMMAND} evidence record --state {output_dir} "
                    f"--snapshot-id {snapshot_id} --evidence verified_offsite_copy "
                    "--source <external-tool-or-monitor>"
                ),
            }
        )
    required = bool(destinations)
    complete = (not required) or evidence_available
    return {
        "schema_version": "northroot.steward.offsite-report.v0",
        "workspace_id": str(plan.get("workspace_id", "unknown")),
        "policy_id": str(plan.get("policy_id", "unknown")),
        "snapshot_id": snapshot_id,
        "required": required,
        "complete": complete,
        "missing_evidence": [] if complete else ["verified_offsite_copy"],
        "available_evidence": evidence_report["available_evidence"],
        "destinations": destinations,
        "execution_model": {
            "kind": "external-delegated",
            "custom_storage_transport": False,
            "copy_execution_owned_by": "private-deployment-tooling",
            "suggested_tools": ["rclone", "restic copy", "external monitor"],
        },
    }


def latest_run_summary_path(output_dir: Path) -> str | None:
    summaries_dir = output_dir / "run-summaries"
    if not summaries_dir.is_dir():
        return None
    summaries = sorted(
        (path for path in summaries_dir.glob("*.json") if path.name != RUN_SUMMARY_INDEX_FILENAME),
        key=lambda path: path.stat().st_mtime,
    )
    if not summaries:
        return None
    return str(summaries[-1])


def operation_lock_path(output_dir: Path) -> Path:
    return output_dir / OPERATION_LOCK_FILENAME


def _operation_registry_context(
    registry_state: Path,
    *,
    operation: str,
    project_id: str | None,
    object_id: str | None,
) -> dict[str, Any]:
    try:
        status = registry.registry_status(registry_state, public_safe=True)
        topology_gate = registry_topology_gate(
            registry_state,
            operation=operation,
            project_id=project_id,
            object_id=object_id,
        )
        topology_checked = bool(project_id and operation in REGISTRY_TOPOLOGY_REQUIRED_OPERATIONS)
        topology_decision = (
            topology_gate["decision"]
            if topology_gate is not None
            else "ready"
            if topology_checked and bool(status.get("ready"))
            else None
        )
        topology_allowed = None
        if topology_checked:
            topology_allowed = topology_gate is None and bool(status.get("ready"))
        return {
            "schema_version": "northroot.steward.operation-registry-context.v0",
            "checked": True,
            "registry_state": str(registry_state),
            "registry_path": status.get("registry_path"),
            "registry_sha256": status.get("registry_sha256"),
            "registry_ready": bool(status.get("ready")),
            "protected_state_ok": bool(status.get("protected_state_ok")),
            "resume_required": bool(status.get("resume_required")),
            "project_id": project_id,
            "object_id": object_id,
            "operation": operation,
            "finding_count": status.get("finding_count"),
            "error_count": status.get("error_count"),
            "topology_checked": topology_checked,
            "topology_allowed": topology_allowed,
            "topology_decision": topology_decision,
        }
    except Exception as exc:  # noqa: BLE001 - recovery context must fail closed as data
        return {
            "schema_version": "northroot.steward.operation-registry-context.v0",
            "checked": False,
            "registry_state": str(registry_state),
            "project_id": project_id,
            "object_id": object_id,
            "operation": operation,
            "registry_sha256": None,
            "registry_ready": False,
            "protected_state_ok": False,
            "resume_required": True,
            "error": str(exc),
        }


def _operation_recovery_registry_context(lock: dict[str, Any], *, operation: str) -> dict[str, Any]:
    registry_state_value = lock.get("registry_state")
    project_id = lock.get("project_id") if isinstance(lock.get("project_id"), str) else None
    object_id = lock.get("object_id") if isinstance(lock.get("object_id"), str) else None
    locked_context = lock.get("registry_context_at_lock")
    locked_sha256 = lock.get("registry_sha256")
    if not isinstance(locked_sha256, str) and isinstance(locked_context, dict):
        locked_sha256 = locked_context.get("registry_sha256")
    if not isinstance(registry_state_value, str) or not registry_state_value:
        return {
            "schema_version": "northroot.steward.operation-registry-recovery.v0",
            "checked": False,
            "resume_state": "registry-not-bound-to-operation",
            "registry_changed_since_lock": None,
            "registry_sha256_at_lock": locked_sha256 if isinstance(locked_sha256, str) else None,
            "registry_sha256_at_recovery": None,
            "current_registry_context": None,
        }
    current_context = _operation_registry_context(
        Path(registry_state_value),
        operation=operation,
        project_id=project_id,
        object_id=object_id,
    )
    current_sha256 = current_context.get("registry_sha256")
    if isinstance(locked_sha256, str) and isinstance(current_sha256, str):
        registry_changed = locked_sha256 != current_sha256
        resume_state = (
            "registry-changed-after-operation-lock"
            if registry_changed
            else "registry-unchanged-after-operation-lock"
        )
    elif isinstance(locked_sha256, str) and current_sha256 is None:
        registry_changed = None
        resume_state = "registry-missing-or-unreadable-at-recovery"
    else:
        registry_changed = None
        resume_state = "registry-change-unknown"
    return {
        "schema_version": "northroot.steward.operation-registry-recovery.v0",
        "checked": True,
        "resume_state": resume_state,
        "registry_changed_since_lock": registry_changed,
        "registry_sha256_at_lock": locked_sha256 if isinstance(locked_sha256, str) else None,
        "registry_sha256_at_recovery": current_sha256 if isinstance(current_sha256, str) else None,
        "current_registry_context": current_context,
    }


def _operation_lock_payload(
    *,
    output_dir: Path,
    operation: str,
    command_args: list[str],
    snapshot_id: str | None,
    restore_target: Path | None,
    registry_state: Path | None,
    project_id: str | None,
    object_id: str | None,
) -> dict[str, Any]:
    registry_context = (
        _operation_registry_context(
            registry_state,
            operation=operation,
            project_id=project_id,
            object_id=object_id,
        )
        if registry_state is not None
        else None
    )
    return {
        "schema_version": "northroot.steward.operation-lock.v0",
        "operation_id": f"{_utc_stamp()}-{operation}",
        "operation": operation,
        "state": str(output_dir),
        "command": _command_string(command_args),
        "command_args": command_args,
        "snapshot_id": snapshot_id,
        "restore_target": str(restore_target) if restore_target is not None else None,
        "registry_state": str(registry_state) if registry_state is not None else None,
        "registry_sha256": registry_context.get("registry_sha256") if registry_context is not None else None,
        "registry_context_at_lock": registry_context,
        "project_id": project_id,
        "object_id": object_id,
        "pid": os.getpid(),
        "started_at": _utc_stamp(),
        "failure_policy": "fail-closed-record-summary-before-retry",
        "resume_hint": "run steward recover-operation before retrying delegated execution",
    }


def _clear_operation_lock(lock_path: Path, *, expected_operation_id: str | None = None) -> None:
    if not lock_path.exists():
        return
    if expected_operation_id is not None:
        lock = model.load_json(lock_path)
        if lock.get("operation_id") != expected_operation_id:
            return
    lock_path.unlink()
    _fsync_directory(lock_path.parent)


def _canonical_json(value: dict[str, Any]) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def _run_summary_path(output_dir: Path, run_id: str) -> Path:
    if not model.RUN_ID_PATTERN.match(run_id):
        raise ValueError(f"run_id is not filesystem-safe: {run_id}")
    return output_dir / "run-summaries" / f"{run_id}.json"


def recover_operation(output_dir: Path) -> dict[str, Any]:
    lock_path = operation_lock_path(output_dir)
    if not lock_path.exists():
        return {
            "schema_version": "northroot.steward.operation-recovery.v0",
            "recovered": False,
            "resume_required": False,
            "lock_path": str(lock_path),
            "lock": None,
            "run_summary_path": None,
            "cleared_lock": False,
        }
    installation = load_installation(output_dir)
    lock_status = _operation_lock_status(output_dir)
    lock_error = lock_status.get("error") if isinstance(lock_status.get("error"), str) else None
    lock = (
        _unreadable_operation_lock(lock_path, lock_error)
        if lock_error is not None
        else lock_status.get("lock")
    )
    if not isinstance(lock, dict):
        lock = _unreadable_operation_lock(lock_path, "operation lock did not parse as an object")
    operation = str(lock.get("operation") or "run")
    if operation not in RECOVERABLE_OPERATION_SUMMARY_OPERATIONS:
        operation = "run"
    failure_stage = "invalid-operation-lock" if lock.get("unreadable") else "interrupted-operation-lock"
    error = (
        f"previous delegated operation lock was unreadable: {lock.get('error')}"
        if lock.get("unreadable")
        else "previous delegated operation was interrupted before completion"
    )
    registry_recovery = _operation_recovery_registry_context(lock, operation=operation)
    retry_policy = (
        "rerun only after recover-operation records this interruption and fresh preflight, "
        "authorization, and topology checks pass"
    )
    operation_payload = {
        "schema_version": "northroot.steward.operation.v0",
        "profile_name": installation["profile_name"],
        "operation": operation,
        "execution_mode": "delegated",
        "delegated_tool": installation["delegated_tool"],
        "command": str(lock.get("command") or ""),
        "execute_requested": True,
        "executed": False,
        "snapshot_id": lock.get("snapshot_id"),
        "return_code": 75,
        "error": error,
        "failure_stage": failure_stage,
        "restore_observation": None,
        "preflight": None,
        "authorization": lock.get("authorization"),
        "registry_topology": lock.get("registry_topology"),
        "registry_recovery": registry_recovery,
        "side_effect_state": "unknown-after-interrupted-delegated-operation",
        "retry_policy": retry_policy,
        "operation_lock": lock,
        "operation_lock_path": str(lock_path),
    }
    summary_path = write_operation_summary(output_dir=output_dir, operation_payload=operation_payload)
    expected_operation_id = str(lock.get("operation_id")) if lock.get("operation_id") else None
    if lock.get("unreadable"):
        lock_path.unlink(missing_ok=True)
        _fsync_directory(lock_path.parent)
    else:
        _clear_operation_lock(lock_path, expected_operation_id=expected_operation_id)
    return {
        "schema_version": "northroot.steward.operation-recovery.v0",
        "recovered": True,
        "resume_required": False,
        "lock_path": str(lock_path),
        "lock": lock,
        "run_summary_path": str(summary_path),
        "cleared_lock": not lock_path.exists(),
    }


def render_operation(
    output_dir: Path,
    operation: str,
    *,
    execute: bool = False,
    restore_target: Path | None = None,
    snapshot_id: str | None = None,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    if operation not in OPERATIONS:
        raise ValueError(f"unsupported steward operation: {operation}")
    installation = load_installation(output_dir)
    if operation == "restore" and restore_target is None:
        raise ValueError("restore requires --target")
    if operation == "restore" and not snapshot_id:
        raise ValueError("restore requires --snapshot-id")
    command_args = _operation_command_args(
        installation,
        operation,
        restore_target=restore_target,
        snapshot_id=snapshot_id,
    )
    executed = False
    return_code = None
    error = None
    failure_stage = None
    restore_observation = None
    preflight_result = None
    authorization = None
    registry_topology = None
    operation_lock = None
    acquired_lock_path = None
    if execute:
        current_lock_path = operation_lock_path(output_dir)
        if current_lock_path.exists():
            lock_status = _operation_lock_status(output_dir)
            lock_error = lock_status.get("error") if isinstance(lock_status.get("error"), str) else None
            operation_lock = (
                _unreadable_operation_lock(current_lock_path, lock_error)
                if lock_error is not None
                else lock_status.get("lock")
            )
            return_code = 75
            error = (
                "delegated operation lock is unreadable; run steward recover-operation before retrying"
                if lock_error is not None
                else "delegated operation lock exists; run steward recover-operation before retrying"
            )
            failure_stage = "operation-lock"
        else:
            operation_lock = _operation_lock_payload(
                output_dir=output_dir,
                operation=operation,
                command_args=command_args,
                snapshot_id=snapshot_id,
                restore_target=restore_target,
                registry_state=registry_state,
                project_id=project_id,
                object_id=object_id,
            )
            _atomic_write_json(current_lock_path, operation_lock)
            acquired_lock_path = current_lock_path
        if return_code is None and registry_state is not None and not project_id:
            return_code = 77
            error = "registry authorization requires --project-id"
            failure_stage = "authorization"
            authorization = {
                "schema_version": "northroot.steward.authorization.v0",
                "operation": operation,
                "project_id": project_id,
                "object_id": object_id,
                "registry_path": str(registry_state),
                "registry_sha256": None,
                "allowed": False,
                "decision": "missing-project-id",
                "reason": "registry authorization requires project_id",
                "requires_human_clearance": False,
                "matched_permission_sets": [],
            }
            if operation_lock is not None:
                operation_lock["authorization"] = authorization
                if acquired_lock_path is not None:
                    _atomic_write_json(acquired_lock_path, operation_lock)
        elif return_code is None and registry_state is not None:
            authorization = authorize_operation(
                registry_state,
                operation=operation,
                project_id=str(project_id),
                object_id=object_id,
                public_safe=True,
            )
            if not authorization["allowed"]:
                return_code = 77
                error = f"registry authorization denied: {authorization['decision']}"
                failure_stage = "authorization"
            else:
                registry_topology = registry_topology_gate(
                    registry_state,
                    operation=operation,
                    project_id=project_id,
                    object_id=object_id,
                )
                if registry_topology is not None:
                    return_code = 77
                    error = f"registry topology denied: {registry_topology['decision']}"
                    failure_stage = "registry-topology"
            if operation_lock is not None:
                operation_lock["authorization"] = authorization
                if registry_topology is not None:
                    operation_lock["registry_topology"] = registry_topology
                if acquired_lock_path is not None:
                    _atomic_write_json(acquired_lock_path, operation_lock)
        if return_code is not None:
            pass
        else:
            preflight_result = render_preflight(
                output_dir,
                registry_state=registry_state,
                project_id=project_id,
                object_id=object_id,
            )
        if preflight_result is not None and not preflight_result["ready"]:
            failed_codes = sorted(
                {
                    str(check["code"])
                    for check in preflight_result["checks"]
                    if check.get("status") != "ok" and check.get("code")
                }
            )
            return_code = 78
            error = "preflight failed" if not failed_codes else f"preflight failed: {', '.join(failed_codes)}"
            failure_stage = "preflight"
        elif return_code is None and operation in {"restore", "restore-drill"}:
            try:
                secret_bindings_path = installation.get("secret_bindings_path")
                secret_bindings = (
                    model.load_json(Path(str(secret_bindings_path))) if secret_bindings_path is not None else None
                )
                execution_env = _runtime_env_for_execution(secret_bindings)
                target = restore_target or output_dir / RESTORE_DRILL_DIR / "latest"
                target.parent.mkdir(parents=True, exist_ok=True)
                completed = subprocess.run(command_args, check=False, env=execution_env)
                executed = True
                return_code = completed.returncode
                if return_code != 0:
                    error = f"delegated command exited with status {return_code}"
                else:
                    restore_observation = _observe_restore_target(target)
                    if not restore_observation["verified"]:
                        return_code = 70
                        error = "restore verification failed: restored target had no observable files"
                        failure_stage = "restore-observation"
            except RuntimeError as err:
                return_code = 78
                error = str(err)
                failure_stage = "runtime-env"
        elif return_code is None:
            try:
                secret_bindings_path = installation.get("secret_bindings_path")
                secret_bindings = (
                    model.load_json(Path(str(secret_bindings_path))) if secret_bindings_path is not None else None
                )
                execution_env = _runtime_env_for_execution(secret_bindings)
                completed = subprocess.run(command_args, check=False, env=execution_env)
                executed = True
                return_code = completed.returncode
                if return_code != 0:
                    error = f"delegated command exited with status {return_code}"
            except RuntimeError as err:
                return_code = 78
                error = str(err)
                failure_stage = "runtime-env"

    operation_payload = {
        "schema_version": "northroot.steward.operation.v0",
        "profile_name": installation["profile_name"],
        "operation": operation,
        "execution_mode": "delegated",
        "delegated_tool": installation["delegated_tool"],
        "command": _command_string(command_args),
        "execute_requested": execute,
        "executed": executed,
        "snapshot_id": snapshot_id,
        "return_code": return_code,
        "error": error,
        "failure_stage": failure_stage,
        "restore_observation": restore_observation,
        "preflight": preflight_result,
        "authorization": authorization,
        "registry_topology": registry_topology,
        "operation_lock": operation_lock,
        "operation_lock_path": str(operation_lock_path(output_dir)) if execute else None,
    }
    summary_path = write_operation_summary(
        output_dir=output_dir,
        operation_payload=operation_payload,
    )
    operation_payload["run_summary_path"] = str(summary_path)
    if acquired_lock_path is not None and operation_lock is not None:
        expected_operation_id = (
            str(operation_lock.get("operation_id")) if operation_lock.get("operation_id") else None
        )
        _clear_operation_lock(acquired_lock_path, expected_operation_id=expected_operation_id)
    return operation_payload


def record_external_evidence(
    output_dir: Path,
    *,
    snapshot_id: str,
    evidence: list[str],
    source: str,
    detail: str | None = None,
    artifact_ref: str | None = None,
) -> dict[str, Any]:
    if not snapshot_id.strip():
        raise ValueError("snapshot_id is required for external evidence")
    if not evidence:
        raise ValueError("at least one external evidence item is required")
    invalid = sorted(item for item in evidence if item not in model.EXTERNAL_RETENTION_EVIDENCE)
    if invalid:
        raise ValueError(
            "external steward evidence can only record "
            f"{sorted(model.EXTERNAL_RETENTION_EVIDENCE)}; invalid: {', '.join(invalid)}"
        )
    if not source.strip():
        raise ValueError("external evidence source is required")
    run_id = f"{_utc_stamp()}-evidence-record"
    snapshot_result = {
        "operation": "evidence-record",
        "snapshot_id": snapshot_id,
        "external_evidence_source": source,
        "detail": detail,
        "artifact_ref": artifact_ref,
        "evidence": list(dict.fromkeys(evidence)),
    }
    verification_result = {
        "schema_version": model.VERIFICATION_RESULT_SCHEMA,
        "repository_check": "not-run",
        "restore_verified": False,
        "restore_observation": None,
        "external_evidence": list(dict.fromkeys(evidence)),
    }
    summary = model.build_run_summary(
        run_id=run_id,
        workspace_id=str(load_plan(output_dir).get("workspace_id", "unknown")),
        status="external-evidence-recorded",
        snapshot_result=snapshot_result,
        verification_result=verification_result,
        tool_invocations=[
            {
                "tool": "external",
                "operation": "evidence-record",
                "source": source,
                "snapshot_id": snapshot_id,
                "artifact_ref": artifact_ref,
                "evidence": list(dict.fromkeys(evidence)),
            }
        ],
    )
    summary_path = _write_indexed_run_summary(output_dir, summary)
    return {
        "schema_version": "northroot.steward.evidence-record.v0",
        "run_summary_path": str(summary_path),
        "recorded": True,
        "snapshot_id": snapshot_id,
        "evidence": list(dict.fromkeys(evidence)),
        "source": source,
        "artifact_ref": artifact_ref,
    }


def import_legacy_run_summaries(
    output_dir: Path,
    legacy_run_import: dict[str, Any],
    *,
    public_safe: bool = False,
) -> dict[str, Any]:
    findings = model.validate_legacy_run_import(legacy_run_import, public_safe=public_safe)
    errors = [finding for finding in findings if finding.severity == "error"]
    if errors:
        detail = "; ".join(f"{finding.path}: {finding.detail}" for finding in errors)
        raise ValueError(f"invalid legacy run import: {detail}")

    installation = load_installation(output_dir)
    lock_path = operation_lock_path(output_dir)
    if lock_path.exists():
        lock_status = _operation_lock_status(output_dir)
        return {
            "schema_version": "northroot.steward.legacy-run-import-result.v0",
            "imported": False,
            "locked": True,
            "resume_required": True,
            "lock": lock_status.get("lock"),
            "lock_error": lock_status.get("error"),
            "detail": "steward has an unresolved operation lock; run steward recover-operation first",
        }

    operation_id = f"{_utc_stamp()}-legacy-run-import"
    lock = {
        "schema_version": "northroot.steward.operation-lock.v0",
        "operation_id": operation_id,
        "operation": "legacy-run-import",
        "state": str(output_dir),
        "command": "steward import-legacy-runs",
        "import_id": legacy_run_import["import_id"],
        "pid": os.getpid(),
        "started_at": _utc_stamp(),
        "failure_policy": "fail-closed-record-summary-before-retry",
        "resume_hint": "run steward recover-operation before retrying legacy run import",
    }
    _atomic_write_json(lock_path, lock)
    inserted: list[str] = []
    skipped_existing: list[str] = []
    written_paths: list[str] = []
    try:
        summaries_dir = output_dir / "run-summaries"
        summaries_dir.mkdir(parents=True, exist_ok=True)
        pending_writes: list[tuple[Path, dict[str, Any]]] = []
        existing_to_index: list[tuple[Path, str]] = []
        for summary in legacy_run_import["run_summaries"]:
            run_id = str(summary["run_id"])
            summary_path = _run_summary_path(output_dir, run_id)
            if summary_path.exists():
                existing = model.load_json(summary_path)
                if _canonical_json(existing) != _canonical_json(summary):
                    raise ValueError(f"conflicting run_id: {run_id}")
                existing_to_index.append((summary_path, run_id))
                skipped_existing.append(run_id)
                written_paths.append(str(summary_path))
                continue
            pending_writes.append((summary_path, summary))

        for summary_path, run_id in existing_to_index:
            _index_run_summary(output_dir, summary_path, run_id=run_id)

        for summary_path, summary in pending_writes:
            run_id = str(summary["run_id"])
            _write_indexed_run_summary(output_dir, summary)
            inserted.append(run_id)
            written_paths.append(str(summary_path))

        operation_summary_path = None
        if inserted:
            import_summary = model.build_run_summary(
                run_id=f"{_utc_stamp()}-legacy-run-import",
                workspace_id=str(load_plan(output_dir).get("workspace_id", "unknown")),
                status="legacy-run-imported",
                snapshot_result={
                    "operation": "legacy-run-import",
                    "import_id": legacy_run_import["import_id"],
                    "source": legacy_run_import["source"],
                    "import_mode": legacy_run_import["import_mode"],
                    "legacy_import_ref": legacy_run_import.get("legacy_import_ref"),
                    "runner_state_ref": legacy_run_import.get("runner_state_ref"),
                    "per_run_state_ref": legacy_run_import.get("per_run_state_ref"),
                    "inserted_run_ids": inserted,
                    "skipped_existing_run_ids": skipped_existing,
                },
                verification_result={
                    "schema_version": model.VERIFICATION_RESULT_SCHEMA,
                    "repository_check": "not-run",
                    "restore_verified": False,
                    "restore_observation": None,
                    "external_evidence": [],
                },
                tool_invocations=[
                    {
                        "tool": "steward",
                        "operation": "legacy-run-import",
                        "import_id": legacy_run_import["import_id"],
                        "inserted_run_ids": inserted,
                        "skipped_existing_run_ids": skipped_existing,
                    }
                ],
            )
            operation_summary_path = _run_summary_path(output_dir, str(import_summary["run_id"]))
            _write_indexed_run_summary(output_dir, import_summary)
        _clear_operation_lock(lock_path, expected_operation_id=operation_id)
        return {
            "schema_version": "northroot.steward.legacy-run-import-result.v0",
            "imported": True,
            "locked": False,
            "resume_required": False,
            "profile_name": installation["profile_name"],
            "import_id": legacy_run_import["import_id"],
            "source": legacy_run_import["source"],
            "import_mode": legacy_run_import["import_mode"],
            "inserted_run_ids": inserted,
            "skipped_existing_run_ids": skipped_existing,
            "run_summary_paths": written_paths,
            "operation_summary_path": str(operation_summary_path) if operation_summary_path else None,
        }
    except Exception:
        _clear_operation_lock(lock_path, expected_operation_id=operation_id)
        raise


def write_operation_summary(*, output_dir: Path, operation_payload: dict[str, Any]) -> Path:
    operation = str(operation_payload["operation"])
    execute_requested = bool(operation_payload.get("execute_requested"))
    executed = bool(operation_payload["executed"])
    return_code = operation_payload.get("return_code")
    ok = (not executed) or return_code == 0
    run_id = f"{_utc_stamp()}-{operation}"
    restore_observation = (
        operation_payload.get("restore_observation")
        if isinstance(operation_payload.get("restore_observation"), dict)
        else None
    )
    restore_verified = bool(
        operation in {"restore", "restore-drill"}
        and ok
        and executed
        and restore_observation
        and restore_observation.get("verified")
    )
    if execute_requested and not executed and operation_payload.get("failure_stage") == "operation-lock":
        status = "delegated-operation-locked"
    elif execute_requested and not executed and operation_payload.get("failure_stage") == "interrupted-operation-lock":
        status = "delegated-interrupted-recovered"
    elif execute_requested and not executed and operation_payload.get("failure_stage") == "invalid-operation-lock":
        status = "delegated-invalid-lock-recovered"
    elif execute_requested and not executed and operation_payload.get("failure_stage") == "authorization":
        status = "delegated-authorization-denied"
    elif execute_requested and not executed and operation_payload.get("failure_stage") == "registry-topology":
        status = "delegated-registry-topology-denied"
    elif execute_requested and not executed and operation_payload.get("failure_stage") == "runtime-env":
        status = "delegated-runtime-env-failed"
    elif execute_requested and operation_payload.get("failure_stage") == "restore-observation":
        status = "delegated-restore-unverified"
    elif execute_requested and not executed:
        status = "delegated-preflight-failed"
    elif not execute_requested:
        status = "delegated-rendered"
    else:
        status = "delegated-ok" if ok else "delegated-failed"
    repository_check = "not-run"
    if operation == "verify" and executed:
        repository_check = "ok" if ok else "failed"
    verification_result = {
        "schema_version": model.VERIFICATION_RESULT_SCHEMA,
        "repository_check": repository_check,
        "restore_verified": restore_verified,
        "restore_observation": restore_observation,
        "external_evidence": [],
    }
    snapshot_result = {
        "operation": operation,
        "delegated_tool": operation_payload["delegated_tool"],
        "command": operation_payload["command"],
        "execute_requested": execute_requested,
        "executed": executed,
        "snapshot_id": operation_payload.get("snapshot_id"),
        "return_code": return_code,
        "failure_stage": operation_payload.get("failure_stage"),
        "preflight_ready": operation_payload.get("preflight", {}).get("ready")
        if isinstance(operation_payload.get("preflight"), dict)
        else None,
        "authorization": operation_payload.get("authorization"),
        "registry_topology": operation_payload.get("registry_topology"),
        "registry_recovery": operation_payload.get("registry_recovery"),
        "side_effect_state": operation_payload.get("side_effect_state"),
        "retry_policy": operation_payload.get("retry_policy"),
        "operation_lock": operation_payload.get("operation_lock"),
        "operation_lock_path": operation_payload.get("operation_lock_path"),
    }
    summary = model.build_run_summary(
        run_id=run_id,
        workspace_id=str(load_plan(output_dir).get("workspace_id", "unknown")),
        status=status,
        snapshot_result=snapshot_result,
        verification_result=verification_result,
        tool_invocations=[
            {
                "tool": operation_payload["delegated_tool"],
                "operation": operation,
                "command": operation_payload["command"],
                "execute_requested": execute_requested,
                "executed": executed,
                "return_code": return_code,
                "failure_stage": operation_payload.get("failure_stage"),
                "preflight": operation_payload.get("preflight"),
                "authorization": operation_payload.get("authorization"),
                "registry_recovery": operation_payload.get("registry_recovery"),
                "side_effect_state": operation_payload.get("side_effect_state"),
                "retry_policy": operation_payload.get("retry_policy"),
                "operation_lock": operation_payload.get("operation_lock"),
                "operation_lock_path": operation_payload.get("operation_lock_path"),
            }
        ],
    )
    return _write_indexed_run_summary(output_dir, summary)


def load_plan(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
    return model.load_json(Path(str(installation["snapshot_plan_path"])))


def _append_registry_context_args(
    argv: list[str],
    *,
    registry_state: Path | None,
    project_id: str | None,
    object_id: str | None,
) -> None:
    if registry_state is not None:
        argv.extend(["--registry-state", str(registry_state)])
    if project_id:
        argv.extend(["--project-id", project_id])
    if object_id:
        argv.extend(["--object-id", object_id])


def _scheduled_operation_command(
    *,
    runner_command: str,
    output_dir: Path,
    operation: str,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> str:
    if operation not in SCHEDULE_OPERATIONS:
        raise ValueError(f"unsupported scheduled operation: {operation}")
    try:
        runner_args = shlex.split(runner_command)
    except ValueError as err:
        raise ValueError(f"runner_command is not shell-parseable: {err}") from err
    if not runner_args:
        raise ValueError("runner_command must not be empty")
    command = [*runner_args, operation, "--state", str(output_dir)]
    _append_registry_context_args(
        command,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    command.append("--execute")
    return _command_string(command)


def render_launchd_template(
    *,
    label: str,
    runner_command: str,
    output_dir: Path,
    every_minutes: int,
    operation: str = "run",
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> str:
    command = _scheduled_operation_command(
        runner_command=runner_command,
        output_dir=output_dir,
        operation=operation,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    return "\n".join(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" '
            '"http://www.apple.com/DTDs/PropertyList-1.0.dtd">',
            '<plist version="1.0">',
            '<dict>',
            '  <key>Label</key>',
            f'  <string>{_xml_text(label)}</string>',
            '  <key>ProgramArguments</key>',
            '  <array>',
            '    <string>/bin/sh</string>',
            '    <string>-lc</string>',
            f'    <string>{_xml_text(command)}</string>',
            '  </array>',
            '  <key>StartInterval</key>',
            f'  <integer>{every_minutes * 60}</integer>',
            '  <key>RunAtLoad</key>',
            '  <true/>',
            '</dict>',
            '</plist>',
            '',
        ]
    )


def render_systemd_service(
    *,
    runner_command: str,
    output_dir: Path,
    operation: str = "run",
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> str:
    command = _scheduled_operation_command(
        runner_command=runner_command,
        output_dir=output_dir,
        operation=operation,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    return "\n".join(
        [
            "[Unit]",
            f"Description=Northroot steward custody {operation}",
            "",
            "[Service]",
            "Type=oneshot",
            f"ExecStart={command}",
            "",
        ]
    )


def render_systemd_timer(*, service_name: str, every_minutes: int) -> str:
    return "\n".join(
        [
            "[Unit]",
            "Description=Run Northroot steward custody profile",
            "",
            "[Timer]",
            f"OnBootSec={every_minutes}min",
            f"OnUnitActiveSec={every_minutes}min",
            f"Unit={service_name}.service",
            "",
            "[Install]",
            "WantedBy=timers.target",
            "",
        ]
    )


def create_schedule(
    *,
    output_dir: Path,
    scheduler: str,
    every_minutes: int,
    runner_command: str = DEFAULT_RUNNER_COMMAND,
    operation: str = "run",
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    if every_minutes <= 0:
        raise ValueError("every_minutes must be greater than zero")
    if operation not in SCHEDULE_OPERATIONS:
        raise ValueError(f"unsupported scheduled operation: {operation}")
    registry_gate = schedule_registry_gate(
        registry_state,
        operation="schedule.create",
        project_id=project_id,
        object_id=object_id,
    )
    if registry_gate is not None:
        raise ScheduleRegistryGateError("schedule.create", registry_gate)
    installation = load_installation(output_dir)
    profile_name = str(installation["profile_name"])
    schedule_scope_id = _schedule_scope_id(
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    schedules_dir = _schedule_dir(output_dir, schedule_scope_id=schedule_scope_id)
    schedules_dir.mkdir(parents=True, exist_ok=True)
    name_suffix = f"-{schedule_scope_id}" if schedule_scope_id else ""
    label_suffix = f".{schedule_scope_id}" if schedule_scope_id else ""

    if scheduler == "launchd":
        label = f"org.northroot.{profile_name}{label_suffix}"
        schedule_path = schedules_dir / f"{label}.plist"
        schedule_sha256 = _atomic_write_text(
            schedule_path,
            render_launchd_template(
                label=label,
                runner_command=runner_command,
                output_dir=output_dir,
                every_minutes=every_minutes,
                operation=operation,
                registry_state=registry_state,
                project_id=project_id,
                object_id=object_id,
            ),
        )
        generated_paths = [str(schedule_path)]
        generated_artifacts = {"launchd_plist": {"path": str(schedule_path), "sha256": schedule_sha256}}
    elif scheduler == "systemd":
        service_name = f"northroot-{profile_name}{name_suffix}"
        service_path = schedules_dir / f"{service_name}.service"
        timer_path = schedules_dir / f"{service_name}.timer"
        service_sha256 = _atomic_write_text(
            service_path,
            render_systemd_service(
                runner_command=runner_command,
                output_dir=output_dir,
                operation=operation,
                registry_state=registry_state,
                project_id=project_id,
                object_id=object_id,
            ),
        )
        timer_sha256 = _atomic_write_text(
            timer_path,
            render_systemd_timer(service_name=service_name, every_minutes=every_minutes),
        )
        schedule_path = timer_path
        generated_paths = [str(service_path), str(timer_path)]
        generated_artifacts = {
            "systemd_service": {"path": str(service_path), "sha256": service_sha256},
            "systemd_timer": {"path": str(timer_path), "sha256": timer_sha256},
        }
    else:
        raise ValueError(f"unsupported scheduler: {scheduler}")

    schedule = {
        "schema_version": "northroot.steward.schedule.v0",
        "profile_name": profile_name,
        "scheduler": scheduler,
        "operation": operation,
        "every_minutes": every_minutes,
        "schedule_path": str(schedule_path),
        "generated_paths": generated_paths,
        "generated_artifacts": generated_artifacts,
        "runner_command": runner_command,
        "registry_state": str(registry_state) if registry_state is not None else None,
        "project_id": project_id,
        "object_id": object_id,
        "schedule_scope_id": schedule_scope_id,
        "installed": False,
        "execution_mode": "delegated",
    }
    _write_schedule_manifest(output_dir, schedule)
    return schedule


def _status_for_schedule_scope(output_dir: Path, *, schedule_scope_id: str | None = None) -> dict[str, Any]:
    schedule_path = _schedule_manifest_path_for_scope(output_dir, schedule_scope_id=schedule_scope_id)
    integrity = render_schedule_integrity(output_dir, schedule_scope_id=schedule_scope_id)
    if not schedule_path.exists():
        return {
            "schema_version": "northroot.steward.schedule-status.v0",
            "configured": False,
            "installed": False,
            "schedule_path": None,
            "schedule_scope_id": schedule_scope_id,
            "schedule_integrity": integrity,
        }
    try:
        schedule = model.load_json(schedule_path)
    except (OSError, ValueError, json.JSONDecodeError):
        return {
            "schema_version": "northroot.steward.schedule-status.v0",
            "configured": True,
            "installed": False,
            "schedule_path": str(schedule_path),
            "schedule_scope_id": schedule_scope_id,
            "schedule_integrity": integrity,
        }
    schedule["schema_version"] = "northroot.steward.schedule-status.v0"
    schedule["configured"] = True
    schedule["schedule_integrity"] = integrity
    return schedule


def schedule_status(
    output_dir: Path,
    *,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    schedule_scope_id = _schedule_scope_id(
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    if schedule_scope_id is not None:
        return _status_for_schedule_scope(output_dir, schedule_scope_id=schedule_scope_id)

    default_status = _status_for_schedule_scope(output_dir)
    if default_status.get("configured") or not default_status.get("schedule_integrity", {}).get("ok"):
        return default_status

    scoped_paths = _scoped_schedule_manifest_paths(output_dir)
    if len(scoped_paths) == 1:
        return _status_for_schedule_scope(output_dir, schedule_scope_id=scoped_paths[0].parent.name)
    if len(scoped_paths) > 1:
        return {
            "schema_version": "northroot.steward.schedule-status.v0",
            "configured": True,
            "installed": False,
            "schedule_path": None,
            "schedule_scope_id": None,
            "requires_schedule_context": True,
            "schedule_count": len(scoped_paths),
            "scoped_schedules": [
                {
                    "schedule_scope_id": path.parent.name,
                    "manifest_path": str(path),
                }
                for path in scoped_paths
            ],
            "schedule_integrity": {
                "schema_version": "northroot.steward.schedule-integrity.v0",
                "ok": False,
                "manifest_path": None,
                "index_path": None,
                "schedule_scope_id": None,
                "status": "schedule-context-required",
                "detail": "multiple scoped schedules exist; pass registry/project/object context",
            },
        }
    return default_status


def _schedule_integrity_error_result(
    *,
    schema_version: str,
    status: dict[str, Any],
    execute: bool = False,
    force: bool = False,
) -> dict[str, Any]:
    integrity = status.get("schedule_integrity")
    base = {
        "schema_version": schema_version,
        "scheduler": status.get("scheduler"),
        "schedule_integrity": integrity,
        "error": "schedule manifest integrity failed; recreate the schedule before platform mutation",
    }
    if schema_version == "northroot.steward.schedule-delete.v0":
        return {
            **base,
            "configured_before_delete": bool(status.get("configured")),
            "removed_paths": [],
            "installed": bool(status.get("installed")),
            "deleted": False,
            "force": force,
        }
    return {
        **base,
        "execute_requested": execute,
        "executed": False,
        "installed": bool(status.get("installed")),
        "return_code": 78,
        "failed_command": None,
        "commands": [],
    }


def _schedule_install_commands(schedule: dict[str, Any]) -> list[list[str]]:
    scheduler = schedule.get("scheduler")
    schedule_path = str(schedule.get("schedule_path"))
    if scheduler == "launchd":
        return [["launchctl", "bootstrap", f"gui/{os.getuid()}", schedule_path]]
    if scheduler == "systemd":
        timer_path = Path(schedule_path)
        service_path = timer_path.with_suffix(".service")
        return [
            ["systemctl", "--user", "link", str(service_path), str(timer_path)],
            ["systemctl", "--user", "enable", "--now", timer_path.name],
        ]
    raise ValueError(f"unsupported scheduler: {scheduler}")


def _schedule_uninstall_commands(schedule: dict[str, Any]) -> list[list[str]]:
    scheduler = schedule.get("scheduler")
    schedule_path = str(schedule.get("schedule_path"))
    if scheduler == "launchd":
        return [["launchctl", "bootout", f"gui/{os.getuid()}", schedule_path]]
    if scheduler == "systemd":
        timer_name = Path(schedule_path).name
        return [["systemctl", "--user", "disable", "--now", timer_name]]
    raise ValueError(f"unsupported scheduler: {scheduler}")


def _run_command_sequence(commands: list[list[str]]) -> tuple[bool, int | None, str | None]:
    for command in commands:
        completed = subprocess.run(command, check=False)
        if completed.returncode != 0:
            return False, completed.returncode, _command_string(command)
    return True, 0, None


def install_schedule(
    output_dir: Path,
    *,
    execute: bool = False,
    require_preflight: bool = True,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    status = schedule_status(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    if not status.get("configured"):
        raise ValueError("schedule must be created before it can be installed")
    if not status.get("schedule_integrity", {}).get("ok"):
        result = _schedule_integrity_error_result(
            schema_version="northroot.steward.schedule-install.v0",
            status=status,
            execute=execute,
        )
        result["preflight_required"] = require_preflight
        result["preflight"] = None
        return result
    registry_state, project_id, object_id = _raise_for_schedule_registry_gate(
        operation="schedule.install",
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
        status=status,
    )
    commands = _schedule_install_commands(status)
    executed = False
    return_code = None
    failed_command = None
    preflight_result = None
    if execute:
        if require_preflight:
            preflight_result = render_preflight(
                output_dir,
                registry_state=registry_state,
                project_id=project_id,
                object_id=object_id,
            )
            if not preflight_result["ready"]:
                return {
                    "schema_version": "northroot.steward.schedule-install.v0",
                    "scheduler": status.get("scheduler"),
                    "execute_requested": execute,
                    "executed": False,
                    "installed": False,
                    "return_code": 78,
                    "failed_command": None,
                    "commands": [_command_string(command) for command in commands],
                    "preflight_required": require_preflight,
                    "preflight": preflight_result,
                    "error": "preflight failed",
                }
        executed = True
        ok, return_code, failed_command = _run_command_sequence(commands)
        status["installed"] = ok
        persisted = dict(status)
        persisted["schema_version"] = "northroot.steward.schedule.v0"
        persisted.pop("configured", None)
        persisted.pop("schedule_integrity", None)
        _write_schedule_manifest(output_dir, persisted)
    return {
        "schema_version": "northroot.steward.schedule-install.v0",
        "scheduler": status.get("scheduler"),
        "execute_requested": execute,
        "executed": executed,
        "installed": bool(status.get("installed")),
        "return_code": return_code,
        "failed_command": failed_command,
        "commands": [_command_string(command) for command in commands],
        "preflight_required": require_preflight,
        "preflight": preflight_result,
        "error": None if return_code in (None, 0) else "schedule install command failed",
    }


def uninstall_schedule(
    output_dir: Path,
    *,
    execute: bool = False,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    status = schedule_status(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    if not status.get("configured"):
        raise ValueError("schedule must be created before it can be uninstalled")
    if not status.get("schedule_integrity", {}).get("ok"):
        return _schedule_integrity_error_result(
            schema_version="northroot.steward.schedule-uninstall.v0",
            status=status,
            execute=execute,
        )
    _raise_for_schedule_registry_gate(
        operation="schedule.uninstall",
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
        status=status,
    )
    commands = _schedule_uninstall_commands(status)
    executed = False
    return_code = None
    failed_command = None
    if execute:
        executed = True
        ok, return_code, failed_command = _run_command_sequence(commands)
        if ok:
            status["installed"] = False
            persisted = dict(status)
            persisted["schema_version"] = "northroot.steward.schedule.v0"
            persisted.pop("configured", None)
            persisted.pop("schedule_integrity", None)
            _write_schedule_manifest(output_dir, persisted)
    return {
        "schema_version": "northroot.steward.schedule-uninstall.v0",
        "scheduler": status.get("scheduler"),
        "execute_requested": execute,
        "executed": executed,
        "installed": bool(status.get("installed")),
        "return_code": return_code,
        "failed_command": failed_command,
        "commands": [_command_string(command) for command in commands],
    }


def delete_schedule(
    output_dir: Path,
    *,
    force: bool = False,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
) -> dict[str, Any]:
    status = schedule_status(
        output_dir,
        registry_state=registry_state,
        project_id=project_id,
        object_id=object_id,
    )
    removed: list[str] = []
    schedule_scope_id = status.get("schedule_scope_id")
    schedules_dir = _schedule_dir(
        output_dir,
        schedule_scope_id=str(schedule_scope_id) if isinstance(schedule_scope_id, str) and schedule_scope_id else None,
    )
    if not status.get("schedule_integrity", {}).get("ok") and not force:
        return _schedule_integrity_error_result(
            schema_version="northroot.steward.schedule-delete.v0",
            status=status,
            force=force,
        )
    if status.get("schedule_integrity", {}).get("ok"):
        _raise_for_schedule_registry_gate(
            operation="schedule.delete",
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
            status=status,
        )
    else:
        gate = schedule_registry_gate(
            registry_state,
            operation="schedule.delete",
            project_id=project_id,
            object_id=object_id,
        )
        if gate is not None:
            raise ScheduleRegistryGateError("schedule.delete", gate)
    installed = bool(status.get("installed"))
    if installed and not force:
        return {
            "schema_version": "northroot.steward.schedule-delete.v0",
            "configured_before_delete": bool(status.get("configured")),
            "removed_paths": removed,
            "installed": installed,
            "deleted": False,
            "force": force,
            "error": "schedule is marked installed; run schedule uninstall --execute before delete or pass --force",
        }
    if schedules_dir.is_dir():
        for path in sorted(schedules_dir.iterdir()):
            if path.is_file():
                path.unlink()
                removed.append(str(path))
        try:
            if schedule_scope_id:
                schedules_dir.rmdir()
        except OSError:
            pass
    return {
        "schema_version": "northroot.steward.schedule-delete.v0",
        "configured_before_delete": bool(status.get("configured")),
        "removed_paths": removed,
        "installed": installed and not force,
        "deleted": True,
        "force": force,
        "error": None,
    }
