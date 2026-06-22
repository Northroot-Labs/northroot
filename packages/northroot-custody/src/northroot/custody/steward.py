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

from . import model
from .registry import authorize_operation


INSTALLATION_SCHEMA = "northroot.steward.installation.v0"
DEFAULT_PROFILE_NAME = "steward"
DEFAULT_RUNNER_COMMAND = "nr steward"
RESTORE_DRILL_DIR = "restore-drills"
SCHEDULE_OPERATIONS = {"run", "verify", "restore-drill"}
OPERATIONS = {"run", "verify", "restore", "restore-drill"}
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
}


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


def _artifact(path: Path) -> dict[str, str]:
    return {
        "path": str(path),
        "sha256": _file_sha256(path),
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
    plan_path.write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    resticprofile_path.write_text(
        render_resticprofile_config(
            plan,
            profile_name=profile_name,
            secret_bindings=secret_bindings,
            repository_bindings=repository_bindings,
        ),
        encoding="utf-8",
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
    installation_path(output_dir).write_text(
        json.dumps(installation.as_dict(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
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


def render_preflight(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
    plan_path = Path(str(installation["snapshot_plan_path"]))
    resticprofile_path = Path(str(installation["resticprofile_path"]))
    checks: list[dict[str, Any]] = []

    checks.append(_check("installation", True, f"loaded {installation_path(output_dir)}"))
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

    schedule = schedule_status(output_dir)
    if schedule.get("configured"):
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
        "destination_execution": _destination_execution(plan),
        "schedule": schedule,
        "checks": checks,
    }


def render_status(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
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
                "force",
                "use_recorded_evidence",
                "skip_preflight",
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
    ]


def _append_snapshot(argv: list[str], snapshot_id: str | None) -> None:
    if snapshot_id:
        argv.extend(["--snapshot-id", snapshot_id])


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
    force: bool = False,
    use_recorded_evidence: bool = False,
    skip_preflight: bool = False,
    registry_state: Path | None = None,
    project_id: str | None = None,
    object_id: str | None = None,
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
    elif operation == "schedule.install":
        argv.extend(["schedule", "install", "--state", state])
        if execute:
            argv.append("--execute")
            delegated_platform_mutation = True
        if skip_preflight:
            argv.append("--skip-preflight")
            warnings.append("skip_preflight is an operator override and should not be used by autonomous agents")
        requires_preflight = not skip_preflight
    elif operation == "schedule.uninstall":
        argv.extend(["schedule", "uninstall", "--state", state])
        if execute:
            argv.append("--execute")
            delegated_platform_mutation = True
    elif operation == "schedule.delete":
        argv.extend(["schedule", "delete", "--state", state])
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

    preflight_ready: bool | None = None
    preflight_failed_codes: list[str] = []
    if execute and requires_preflight and operation not in {"schedule.install"}:
        preflight = render_preflight(output_dir)
        preflight_ready = bool(preflight["ready"])
        preflight_failed_codes = [
            str(check["code"])
            for check in preflight.get("checks", [])
            if isinstance(check, dict) and check.get("status") != "ok" and check.get("code")
        ]
        if not preflight_ready:
            refused_reasons.append("preflight is not ready for execute")
    elif execute and operation == "schedule.install" and requires_preflight:
        preflight = render_preflight(output_dir)
        preflight_ready = bool(preflight["ready"])
        preflight_failed_codes = [
            str(check["code"])
            for check in preflight.get("checks", [])
            if isinstance(check, dict) and check.get("status") != "ok" and check.get("code")
        ]
        if not preflight_ready:
            refused_reasons.append("preflight is not ready for schedule install")

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
        "side_effects": {
            "writes_run_summary": writes_run_summary,
            "mutates_backup_repository": mutates_backup_repository,
            "delegates_platform_scheduler_mutation": delegated_platform_mutation,
        },
        "agent_guidance": {
            "execute_requires_explicit_flag": True,
            "bind_placeholders_before_invocation": True,
            "do_not_shell_join_argv": True,
            "do_not_read_or_log_secret_values": True,
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
        ],
        "prohibited_operations": [
            "direct restic/resticprofile shell construction outside this manifest",
            "forget/prune without a retention decision that allows it",
            "reading, printing, storing, or echoing resolved secret values",
            "editing generated resticprofile config as the source of truth",
            "performing recovery restores outside the bounded restore command",
            "assuming additional destinations were copied because the primary backup ran",
            "recording repository-check or restore-drill evidence through external evidence import",
        ],
        "contracts": {
            "inventory_schema": model.INVENTORY_SCHEMA,
            "policy_schema": model.POLICY_SCHEMA,
            "snapshot_plan_schema": model.SNAPSHOT_PLAN_SCHEMA,
            "repository_bindings_schema": model.REPOSITORY_BINDINGS_SCHEMA,
            "secret_bindings_schema": model.SECRET_BINDINGS_SCHEMA,
            "verification_result_schema": model.VERIFICATION_RESULT_SCHEMA,
            "run_summary_schema": model.RUN_SUMMARY_SCHEMA,
            "retention_decision_schema": model.RETENTION_DECISION_SCHEMA,
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
            "generated_schedule_integrity_checked_by_preflight": True,
            "uninstall_required_before_delete_when_installed": True,
            "delete_force_is_operator_cleanup_only": True,
        },
        "verification_required": status["verification_required"],
        "retention_prune_requires": status["retention_prune_requires"],
        "schedule": status["schedule"],
    }


def render_state_verification(output_dir: Path, *, snapshot_id: str | None = None) -> dict[str, Any]:
    status = render_status(output_dir)
    preflight = render_preflight(output_dir)
    capabilities = render_capabilities(output_dir)
    evidence_report = render_evidence_report(output_dir, snapshot_id=snapshot_id)
    schedule = status["schedule"]
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
    ]
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
    return {
        "schema_version": "northroot.steward.state-verification.v0",
        "profile_name": status["profile_name"],
        "ready": ready,
        "safe_to_execute": bool(preflight["ready"]),
        "safe_to_install_schedule": bool(preflight["ready"] and schedule.get("configured")),
        "preflight_ready": bool(preflight["ready"]),
        "preflight_failed_codes": failed_codes,
        "schedule_configured": bool(schedule.get("configured")),
        "schedule_installed": bool(schedule.get("installed")),
        "latest_run_summary_path": status["latest_run_summary_path"],
        "snapshot_id": snapshot_id,
        "retention_evidence_ready": retention_decision["allowed"] if retention_decision is not None else None,
        "retention_decision": retention_decision,
        "evidence_report": evidence_report,
        "checks": checks,
    }


def render_report(output_dir: Path, *, snapshot_id: str | None = None) -> dict[str, Any]:
    status = render_status(output_dir)
    preflight = render_preflight(output_dir)
    evidence_report = render_evidence_report(output_dir, snapshot_id=snapshot_id)
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
    return sorted(summaries_dir.glob("*.json"), key=lambda path: path.stat().st_mtime)


def render_evidence_report(output_dir: Path, *, snapshot_id: str | None = None) -> dict[str, Any]:
    plan = load_plan(output_dir)
    observations: list[dict[str, Any]] = []
    evidence: set[str] = set()
    for summary_path in iter_run_summaries(output_dir):
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
    summaries = sorted(summaries_dir.glob("*.json"), key=lambda path: path.stat().st_mtime)
    if not summaries:
        return None
    return str(summaries[-1])


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
    if execute:
        if registry_state is not None and not project_id:
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
        elif registry_state is not None:
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
        if return_code is not None:
            pass
        else:
            preflight_result = render_preflight(output_dir)
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
    }
    summary_path = write_operation_summary(
        output_dir=output_dir,
        operation_payload=operation_payload,
    )
    operation_payload["run_summary_path"] = str(summary_path)
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
    summaries_dir = output_dir / "run-summaries"
    summaries_dir.mkdir(parents=True, exist_ok=True)
    summary_path = summaries_dir / f"{run_id}.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return {
        "schema_version": "northroot.steward.evidence-record.v0",
        "run_summary_path": str(summary_path),
        "recorded": True,
        "snapshot_id": snapshot_id,
        "evidence": list(dict.fromkeys(evidence)),
        "source": source,
        "artifact_ref": artifact_ref,
    }


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
    if execute_requested and not executed and operation_payload.get("failure_stage") == "authorization":
        status = "delegated-authorization-denied"
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
            }
        ],
    )
    summaries_dir = output_dir / "run-summaries"
    summaries_dir.mkdir(parents=True, exist_ok=True)
    summary_path = summaries_dir / f"{run_id}.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return summary_path


def load_plan(output_dir: Path) -> dict[str, Any]:
    installation = load_installation(output_dir)
    return model.load_json(Path(str(installation["snapshot_plan_path"])))


def _scheduled_operation_command(*, runner_command: str, output_dir: Path, operation: str) -> str:
    if operation not in SCHEDULE_OPERATIONS:
        raise ValueError(f"unsupported scheduled operation: {operation}")
    try:
        runner_args = shlex.split(runner_command)
    except ValueError as err:
        raise ValueError(f"runner_command is not shell-parseable: {err}") from err
    if not runner_args:
        raise ValueError("runner_command must not be empty")
    return _command_string([*runner_args, operation, "--state", str(output_dir), "--execute"])


def render_launchd_template(
    *,
    label: str,
    runner_command: str,
    output_dir: Path,
    every_minutes: int,
    operation: str = "run",
) -> str:
    command = _scheduled_operation_command(
        runner_command=runner_command,
        output_dir=output_dir,
        operation=operation,
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


def render_systemd_service(*, runner_command: str, output_dir: Path, operation: str = "run") -> str:
    command = _scheduled_operation_command(
        runner_command=runner_command,
        output_dir=output_dir,
        operation=operation,
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
) -> dict[str, Any]:
    if every_minutes <= 0:
        raise ValueError("every_minutes must be greater than zero")
    if operation not in SCHEDULE_OPERATIONS:
        raise ValueError(f"unsupported scheduled operation: {operation}")
    installation = load_installation(output_dir)
    profile_name = str(installation["profile_name"])
    schedules_dir = output_dir / "schedules"
    schedules_dir.mkdir(parents=True, exist_ok=True)

    if scheduler == "launchd":
        label = f"org.northroot.{profile_name}"
        schedule_path = schedules_dir / f"{label}.plist"
        schedule_path.write_text(
            render_launchd_template(
                label=label,
                runner_command=runner_command,
                output_dir=output_dir,
                every_minutes=every_minutes,
                operation=operation,
            ),
            encoding="utf-8",
        )
        generated_paths = [str(schedule_path)]
        generated_artifacts = {"launchd_plist": _artifact(schedule_path)}
    elif scheduler == "systemd":
        service_name = f"northroot-{profile_name}"
        service_path = schedules_dir / f"{service_name}.service"
        timer_path = schedules_dir / f"{service_name}.timer"
        service_path.write_text(
            render_systemd_service(
                runner_command=runner_command,
                output_dir=output_dir,
                operation=operation,
            ),
            encoding="utf-8",
        )
        timer_path.write_text(
            render_systemd_timer(service_name=service_name, every_minutes=every_minutes),
            encoding="utf-8",
        )
        schedule_path = timer_path
        generated_paths = [str(service_path), str(timer_path)]
        generated_artifacts = {
            "systemd_service": _artifact(service_path),
            "systemd_timer": _artifact(timer_path),
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
        "installed": False,
        "execution_mode": "delegated",
    }
    (schedules_dir / "schedule.json").write_text(json.dumps(schedule, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return schedule


def schedule_status(output_dir: Path) -> dict[str, Any]:
    schedules_dir = output_dir / "schedules"
    schedule_path = schedules_dir / "schedule.json"
    if not schedule_path.exists():
        return {
            "schema_version": "northroot.steward.schedule-status.v0",
            "configured": False,
            "installed": False,
            "schedule_path": None,
        }
    schedule = model.load_json(schedule_path)
    schedule["schema_version"] = "northroot.steward.schedule-status.v0"
    schedule["configured"] = True
    return schedule


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


def install_schedule(output_dir: Path, *, execute: bool = False, require_preflight: bool = True) -> dict[str, Any]:
    status = schedule_status(output_dir)
    if not status.get("configured"):
        raise ValueError("schedule must be created before it can be installed")
    commands = _schedule_install_commands(status)
    executed = False
    return_code = None
    failed_command = None
    preflight_result = None
    if execute:
        if require_preflight:
            preflight_result = render_preflight(output_dir)
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
        schedule_file = output_dir / "schedules" / "schedule.json"
        persisted = dict(status)
        persisted["schema_version"] = "northroot.steward.schedule.v0"
        persisted.pop("configured", None)
        schedule_file.write_text(json.dumps(persisted, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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


def uninstall_schedule(output_dir: Path, *, execute: bool = False) -> dict[str, Any]:
    status = schedule_status(output_dir)
    if not status.get("configured"):
        raise ValueError("schedule must be created before it can be uninstalled")
    commands = _schedule_uninstall_commands(status)
    executed = False
    return_code = None
    failed_command = None
    if execute:
        executed = True
        ok, return_code, failed_command = _run_command_sequence(commands)
        if ok:
            status["installed"] = False
            schedule_file = output_dir / "schedules" / "schedule.json"
            persisted = dict(status)
            persisted["schema_version"] = "northroot.steward.schedule.v0"
            persisted.pop("configured", None)
            schedule_file.write_text(json.dumps(persisted, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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


def delete_schedule(output_dir: Path, *, force: bool = False) -> dict[str, Any]:
    status = schedule_status(output_dir)
    removed: list[str] = []
    schedules_dir = output_dir / "schedules"
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
    return {
        "schema_version": "northroot.steward.schedule-delete.v0",
        "configured_before_delete": bool(status.get("configured")),
        "removed_paths": removed,
        "installed": installed and not force,
        "deleted": True,
        "force": force,
        "error": None,
    }
