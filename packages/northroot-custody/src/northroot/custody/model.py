"""Northroot custody V0 contracts and validators.

This package intentionally delegates backup execution, scheduling, secrets,
storage transport, and monitoring to established tools. It only owns the
public-safe custody vocabulary, validation, and delegated plan rendering.
"""

from __future__ import annotations

import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


INVENTORY_SCHEMA = "northroot.custody.workspace-inventory.v0"
POLICY_SCHEMA = "northroot.custody.policy.v0"
SNAPSHOT_PLAN_SCHEMA = "northroot.custody.snapshot-plan.v0"
VERIFICATION_RESULT_SCHEMA = "northroot.custody.verification-result.v0"
RETENTION_DECISION_SCHEMA = "northroot.custody.retention-decision.v0"
RUN_SUMMARY_SCHEMA = "northroot.custody.run-summary.v0"
SECRET_BINDINGS_SCHEMA = "northroot.custody.secret-bindings.v0"
REPOSITORY_BINDINGS_SCHEMA = "northroot.custody.repository-bindings.v0"
COMMAND_PLAN_SCHEMA = "northroot.steward.command-plan.v0"

STATE_ROLES = {"authoritative", "generated", "ignored"}
BOUNDARY_TYPES = {"sqlite-online-backup", "postgres-native-backup", "journal-seal", "filesystem"}
ADAPTERS = {"resticprofile", "restic"}
VERIFY_MODES = {"repository-check", "sample-restore", "app-restore-drill", "journal-replay"}
RETENTION_EVIDENCE = {"verified_snapshot", "verified_offsite_copy", "restore_drill"}
EXTERNAL_RETENTION_EVIDENCE = {"verified_offsite_copy"}
OBJECT_TYPES = {
    "artifact-dir",
    "cache",
    "env-file",
    "generated-state",
    "journal",
    "postgres",
    "repo",
    "secret-file",
    "sqlite",
}
VISIBILITY_CLASSES = {"public", "private", "secret", "regulated", "ephemeral"}
RESTORE_CLASSES = {"full-restore", "metadata-only", "rehydrate-from-provider", "never-export"}
STORAGE_BINDING_PREFIXES = (
    "artifact://",
    "cache://",
    "env://",
    "journal://",
    "provider://",
    "repository://",
    "secret://",
    "workspace://",
)
SECRET_REF_PREFIXES = ("secret://", "env://")
REPOSITORY_REF_PREFIXES = ("repository://",)
SECRET_BINDING_PROVIDERS = {"onepassword-cli", "macos-keychain", "env-command"}
RUNTIME_ENV_PROVIDERS = {"macos-keychain", "env-command"}
ENV_NAME_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
OBJECT_ID_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.:/-]*$")
REPOSITORY_CHECK_STATUSES = {"ok", "failed", "not-run"}

PUBLIC_FORBIDDEN_PATTERNS = (
    (re.compile(r"(^|[\"'\s])/(Users|Volumes|var|private|srv|home)/"), "real_machine_path"),
    (re.compile(r"\bop://[^\"'\s]+"), "op_secret_reference"),
    (re.compile(r"\b(s3|b2|gs|az|rest|sftp):[^\"'\s]+"), "backup_repository_url"),
    (re.compile(r"https?://[^\"'\s]+"), "external_service_url"),
    (re.compile(r"(?i)(backup_password|credential_value|private_key)"), "secret_field"),
)


@dataclass(frozen=True)
class Finding:
    severity: str
    code: str
    path: str
    detail: str

    def as_dict(self) -> dict[str, str]:
        return asdict(self)


def load_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return payload


def findings_to_json(findings: list[Finding]) -> str:
    return json.dumps([finding.as_dict() for finding in findings], indent=2, sort_keys=True) + "\n"


def _finding(code: str, path: str, detail: str, severity: str = "error") -> Finding:
    return Finding(severity, code, path, detail)


def _is_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _is_object_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, dict) for item in value)


def _is_string_list(value: Any) -> bool:
    return isinstance(value, list) and bool(value) and all(_is_string(item) for item in value)


def _require_string(payload: dict[str, Any], key: str, path: str) -> list[Finding]:
    if not _is_string(payload.get(key)):
        return [_finding("missing_string", f"{path}.{key}", f"{key} must be a non-empty string")]
    return []


def _validate_object_custody(objects: Any, *, path: str) -> list[Finding]:
    findings: list[Finding] = []
    if objects is None:
        return findings
    if not _is_object_list(objects):
        return [_finding("objects", path, "objects must be a list of custody objects when present")]

    seen_ids: set[str] = set()
    for index, obj in enumerate(objects):
        object_path = f"{path}[{index}]"
        object_id = obj.get("object_id")
        if not _is_string(object_id):
            findings.append(_finding("missing_string", f"{object_path}.object_id", "object_id is required"))
        elif not OBJECT_ID_PATTERN.match(str(object_id)):
            findings.append(
                _finding(
                    "invalid_object_id",
                    f"{object_path}.object_id",
                    "object_id must be a stable symbolic identifier",
                )
            )
        elif str(object_id) in seen_ids:
            findings.append(
                _finding("duplicate_object_id", f"{object_path}.object_id", f"duplicate object_id: {object_id}")
            )
        else:
            seen_ids.add(str(object_id))

        if obj.get("object_type") not in OBJECT_TYPES:
            findings.append(
                _finding(
                    "invalid_object_type",
                    f"{object_path}.object_type",
                    f"object_type must be one of {sorted(OBJECT_TYPES)}",
                )
            )
        if obj.get("visibility") not in VISIBILITY_CLASSES:
            findings.append(
                _finding(
                    "invalid_visibility",
                    f"{object_path}.visibility",
                    f"visibility must be one of {sorted(VISIBILITY_CLASSES)}",
                )
            )
        storage_binding = obj.get("storage_binding")
        if not _is_string(storage_binding):
            findings.append(
                _finding("missing_string", f"{object_path}.storage_binding", "storage_binding is required")
            )
        elif not str(storage_binding).startswith(STORAGE_BINDING_PREFIXES):
            findings.append(
                _finding(
                    "invalid_storage_binding",
                    f"{object_path}.storage_binding",
                    f"storage_binding must use one of {STORAGE_BINDING_PREFIXES}",
                )
            )
        if not isinstance(obj.get("custody_policy"), dict):
            findings.append(
                _finding(
                    "custody_policy",
                    f"{object_path}.custody_policy",
                    "custody_policy must be an object",
                )
            )
        if not isinstance(obj.get("redaction_policy"), dict):
            findings.append(
                _finding(
                    "redaction_policy",
                    f"{object_path}.redaction_policy",
                    "redaction_policy must be an object",
                )
            )
        restore_class = obj.get("restore_class")
        if restore_class not in RESTORE_CLASSES:
            findings.append(
                _finding(
                    "invalid_restore_class",
                    f"{object_path}.restore_class",
                    f"restore_class must be one of {sorted(RESTORE_CLASSES)}",
                )
            )
        if obj.get("visibility") == "ephemeral" and restore_class == "full-restore":
            findings.append(
                _finding(
                    "ephemeral_full_restore",
                    f"{object_path}.restore_class",
                    "ephemeral objects cannot require full restore",
                )
            )
    return findings


def _object_restore_classes(objects: list[dict[str, Any]]) -> dict[str, list[str]]:
    classes = {restore_class: [] for restore_class in sorted(RESTORE_CLASSES)}
    for obj in objects:
        classes[str(obj["restore_class"])].append(str(obj["object_id"]))
    return classes


def find_public_private_bindings(payload: Any, path: str = "$") -> list[Finding]:
    findings: list[Finding] = []
    if isinstance(payload, dict):
        for key, value in payload.items():
            findings.extend(find_public_private_bindings(value, f"{path}.{key}"))
    elif isinstance(payload, list):
        for index, value in enumerate(payload):
            findings.extend(find_public_private_bindings(value, f"{path}[{index}]"))
    elif isinstance(payload, str):
        for pattern, code in PUBLIC_FORBIDDEN_PATTERNS:
            if pattern.search(payload):
                findings.append(
                    _finding(
                        "public_private_binding",
                        path,
                        f"public-safe custody documents must not contain {code}",
                    )
                )
    return findings


def validate_workspace_inventory(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != INVENTORY_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {INVENTORY_SCHEMA}"))
    findings.extend(_require_string(payload, "workspace_id", "$"))

    state_roots = payload.get("state_roots")
    if not _is_object_list(state_roots):
        findings.append(_finding("state_roots", "$.state_roots", "state_roots must be a list of objects"))
    else:
        seen_ids: set[str] = set()
        for index, root in enumerate(state_roots):
            root_path = f"$.state_roots[{index}]"
            root_id = root.get("id")
            if not _is_string(root_id):
                findings.append(_finding("missing_string", f"{root_path}.id", "state root id is required"))
            elif str(root_id) in seen_ids:
                findings.append(_finding("duplicate_id", f"{root_path}.id", f"duplicate state root id: {root_id}"))
            else:
                seen_ids.add(str(root_id))
            if not _is_string(root.get("path")):
                findings.append(_finding("missing_string", f"{root_path}.path", "state root path is required"))
            if root.get("role") not in STATE_ROLES:
                findings.append(
                    _finding("invalid_state_role", f"{root_path}.role", f"role must be one of {sorted(STATE_ROLES)}")
                )

    boundaries = payload.get("consistency_boundaries")
    if not _is_object_list(boundaries):
        findings.append(
            _finding(
                "consistency_boundaries",
                "$.consistency_boundaries",
                "consistency_boundaries must be a list of objects",
            )
        )
    else:
        state_root_ids = {str(root.get("id")) for root in state_roots or [] if isinstance(root, dict)}
        for index, boundary in enumerate(boundaries):
            boundary_path = f"$.consistency_boundaries[{index}]"
            if not _is_string(boundary.get("id")):
                findings.append(_finding("missing_string", f"{boundary_path}.id", "boundary id is required"))
            if boundary.get("type") not in BOUNDARY_TYPES:
                findings.append(
                    _finding(
                        "invalid_boundary_type",
                        f"{boundary_path}.type",
                        f"type must be one of {sorted(BOUNDARY_TYPES)}",
                    )
                )
            source_ids = boundary.get("source_root_ids")
            if not isinstance(source_ids, list) or not source_ids:
                findings.append(
                    _finding(
                        "missing_source_roots",
                        f"{boundary_path}.source_root_ids",
                        "boundary must name at least one source root",
                    )
                )
            else:
                for source_id in source_ids:
                    if source_id not in state_root_ids:
                        findings.append(
                            _finding(
                                "unknown_source_root",
                                f"{boundary_path}.source_root_ids",
                                f"unknown state root id: {source_id}",
                            )
                        )

    findings.extend(_validate_object_custody(payload.get("objects"), path="$.objects"))

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_custody_policy(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != POLICY_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {POLICY_SCHEMA}"))
    findings.extend(_require_string(payload, "policy_id", "$"))
    findings.extend(_require_string(payload, "workspace_id", "$"))

    destinations = payload.get("destinations")
    if not _is_object_list(destinations):
        findings.append(_finding("destinations", "$.destinations", "destinations must be a list of objects"))
    else:
        for index, destination in enumerate(destinations):
            dest_path = f"$.destinations[{index}]"
            if not _is_string(destination.get("id")):
                findings.append(_finding("missing_string", f"{dest_path}.id", "destination id is required"))
            if destination.get("adapter") not in ADAPTERS:
                findings.append(_finding("invalid_adapter", f"{dest_path}.adapter", f"adapter must be one of {sorted(ADAPTERS)}"))
            if destination.get("repository_ref") and not _is_string(destination.get("repository_ref")):
                findings.append(
                    _finding(
                        "invalid_repository_ref",
                        f"{dest_path}.repository_ref",
                        "repository_ref must be a non-empty string when present",
                    )
                )
            if destination.get("secret_ref"):
                secret_ref = destination.get("secret_ref")
                if not _is_string(secret_ref):
                    findings.append(
                        _finding(
                            "invalid_secret_ref",
                            f"{dest_path}.secret_ref",
                            "secret_ref must be a non-empty string when present",
                        )
                    )
                elif not str(secret_ref).startswith(SECRET_REF_PREFIXES):
                    findings.append(
                        _finding(
                            "invalid_secret_ref",
                            f"{dest_path}.secret_ref",
                            f"secret_ref must use one of {SECRET_REF_PREFIXES}",
                        )
                    )

    verification = payload.get("verification")
    if not isinstance(verification, dict):
        findings.append(_finding("verification", "$.verification", "verification must be an object"))
    else:
        modes = verification.get("required_modes")
        if not isinstance(modes, list) or not modes:
            findings.append(
                _finding(
                    "verification_modes",
                    "$.verification.required_modes",
                    "required_modes must list at least one verification mode",
                )
            )
        else:
            for mode in modes:
                if mode not in VERIFY_MODES:
                    findings.append(
                        _finding(
                            "invalid_verification_mode",
                            "$.verification.required_modes",
                            f"mode must be one of {sorted(VERIFY_MODES)}",
                        )
                    )

    retention = payload.get("retention")
    if not isinstance(retention, dict):
        findings.append(_finding("retention", "$.retention", "retention must be an object"))
    else:
        evidence = retention.get("prune_requires")
        if not isinstance(evidence, list) or not evidence:
            findings.append(
                _finding(
                    "prune_requires",
                    "$.retention.prune_requires",
                    "prune_requires must list evidence required before forget/prune",
                )
            )
        else:
            for item in evidence:
                if item not in RETENTION_EVIDENCE:
                    findings.append(
                        _finding(
                            "invalid_retention_evidence",
                            "$.retention.prune_requires",
                            f"evidence must be one of {sorted(RETENTION_EVIDENCE)}",
                        )
                    )

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def render_snapshot_plan(inventory: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    findings = validate_workspace_inventory(inventory) + validate_custody_policy(policy)
    if findings:
        details = "; ".join(f"{finding.path}: {finding.detail}" for finding in findings)
        raise ValueError(f"cannot render snapshot plan from invalid custody documents: {details}")
    if inventory["workspace_id"] != policy["workspace_id"]:
        raise ValueError("inventory and policy workspace_id values must match")

    authoritative_roots = [
        {"id": root["id"], "path": root["path"]}
        for root in inventory["state_roots"]
        if root["role"] == "authoritative"
    ]
    ignored_paths = [root["path"] for root in inventory["state_roots"] if root["role"] in {"generated", "ignored"}]
    object_custody = list(inventory.get("objects", []))

    plan = {
        "schema_version": SNAPSHOT_PLAN_SCHEMA,
        "workspace_id": inventory["workspace_id"],
        "policy_id": policy["policy_id"],
        "adapter": "resticprofile",
        "sources": authoritative_roots,
        "excludes": ignored_paths,
        "consistency_boundaries": inventory["consistency_boundaries"],
        "destinations": policy["destinations"],
        "verification_required": policy["verification"]["required_modes"],
        "retention": policy["retention"],
        "retention_prune_requires": policy["retention"]["prune_requires"],
        "execution": {
            "kind": "delegated",
            "tool": "resticprofile",
            "custom_backup_engine": False,
        },
    }
    if object_custody:
        plan["object_custody"] = object_custody
        plan["object_restore_classes"] = _object_restore_classes(object_custody)
    return plan


def validate_snapshot_plan(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != SNAPSHOT_PLAN_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {SNAPSHOT_PLAN_SCHEMA}"))
    findings.extend(_require_string(payload, "workspace_id", "$"))
    findings.extend(_require_string(payload, "policy_id", "$"))
    if payload.get("adapter") not in ADAPTERS:
        findings.append(_finding("invalid_adapter", "$.adapter", f"adapter must be one of {sorted(ADAPTERS)}"))
    execution = payload.get("execution")
    if not isinstance(execution, dict):
        findings.append(_finding("execution", "$.execution", "execution must be an object"))
    elif execution.get("custom_backup_engine") is not False:
        findings.append(
            _finding(
                "custom_backup_engine",
                "$.execution.custom_backup_engine",
                "snapshot plans must delegate execution instead of declaring a custom backup engine",
            )
        )
    if not _is_object_list(payload.get("sources")):
        findings.append(_finding("sources", "$.sources", "sources must be a list of objects"))
    object_custody = payload.get("object_custody")
    findings.extend(_validate_object_custody(object_custody, path="$.object_custody"))
    if _is_object_list(object_custody) and isinstance(payload.get("object_restore_classes"), dict):
        expected_classes = _object_restore_classes(list(object_custody))
        if payload["object_restore_classes"] != expected_classes:
            findings.append(
                _finding(
                    "object_restore_classes",
                    "$.object_restore_classes",
                    "object_restore_classes must match object_custody restore classes",
                )
            )
    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_verification_result(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != VERIFICATION_RESULT_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {VERIFICATION_RESULT_SCHEMA}"))
    if payload.get("repository_check") not in REPOSITORY_CHECK_STATUSES:
        findings.append(
            _finding(
                "repository_check",
                "$.repository_check",
                f"repository_check must be one of {sorted(REPOSITORY_CHECK_STATUSES)}",
            )
        )
    if not isinstance(payload.get("restore_verified"), bool):
        findings.append(
            _finding(
                "restore_verified",
                "$.restore_verified",
                "restore_verified must be a boolean",
            )
        )
    elif payload.get("restore_verified") is True and not isinstance(payload.get("restore_observation"), dict):
        findings.append(
            _finding(
                "missing_restore_observation",
                "$.restore_observation",
                "verified restores must include a restore observation",
            )
        )
    elif (
        payload.get("restore_verified") is True
        and isinstance(payload.get("restore_observation"), dict)
        and payload["restore_observation"].get("verified") is not True
    ):
        findings.append(
            _finding(
                "unverified_restore_observation",
                "$.restore_observation.verified",
                "verified restores must include an observed restored target",
            )
        )
    elif (
        payload.get("restore_verified") is True
        and isinstance(payload.get("restore_observation"), dict)
        and not _is_string(payload["restore_observation"].get("manifest_sha256"))
    ):
        findings.append(
            _finding(
                "missing_restore_manifest_hash",
                "$.restore_observation.manifest_sha256",
                "verified restores must include a deterministic manifest hash",
            )
        )
    external_evidence = payload.get("external_evidence", [])
    if not isinstance(external_evidence, list):
        findings.append(_finding("external_evidence", "$.external_evidence", "external_evidence must be a list"))
    else:
        invalid_evidence = sorted(str(item) for item in external_evidence if item not in EXTERNAL_RETENTION_EVIDENCE)
        if invalid_evidence:
            findings.append(
                _finding(
                    "invalid_external_evidence",
                    "$.external_evidence",
                    f"external_evidence can only include {sorted(EXTERNAL_RETENTION_EVIDENCE)}; invalid: {invalid_evidence}",
                )
            )
    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_run_summary(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != RUN_SUMMARY_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {RUN_SUMMARY_SCHEMA}"))
    findings.extend(_require_string(payload, "run_id", "$"))
    findings.extend(_require_string(payload, "workspace_id", "$"))
    findings.extend(_require_string(payload, "status", "$"))
    if not isinstance(payload.get("snapshot_result"), dict):
        findings.append(_finding("snapshot_result", "$.snapshot_result", "snapshot_result must be an object"))
    verification = payload.get("verification_result")
    if not isinstance(verification, dict):
        findings.append(_finding("verification_result", "$.verification_result", "verification_result must be an object"))
    else:
        findings.extend(validate_verification_result(verification, public_safe=public_safe))
        if verification.get("restore_verified") is not True:
            findings.append(
                _finding(
                    "restore_not_verified",
                    "$.verification_result.restore_verified",
                    "restore verification must be explicit and true before a run summary is considered verified",
                    severity="warning",
                )
            )
    retention_decision = payload.get("retention_decision")
    if retention_decision is not None:
        if not isinstance(retention_decision, dict):
            findings.append(
                _finding(
                    "retention_decision",
                    "$.retention_decision",
                    "retention_decision must be an object when present",
                )
            )
        else:
            findings.extend(validate_retention_decision(retention_decision, public_safe=public_safe))
    if not isinstance(payload.get("tool_invocations", []), list):
        findings.append(_finding("tool_invocations", "$.tool_invocations", "tool_invocations must be a list"))
    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_secret_bindings(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != SECRET_BINDINGS_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {SECRET_BINDINGS_SCHEMA}"))

    bindings = payload.get("bindings")
    if not _is_object_list(bindings):
        findings.append(_finding("bindings", "$.bindings", "bindings must be a list of objects"))
    else:
        seen_refs: set[str] = set()
        for index, binding in enumerate(bindings):
            binding_path = f"$.bindings[{index}]"
            secret_ref = binding.get("secret_ref")
            if not _is_string(secret_ref):
                findings.append(_finding("missing_string", f"{binding_path}.secret_ref", "secret_ref is required"))
            elif not str(secret_ref).startswith(SECRET_REF_PREFIXES):
                findings.append(
                    _finding(
                        "invalid_secret_ref",
                        f"{binding_path}.secret_ref",
                        f"secret_ref must use one of {SECRET_REF_PREFIXES}",
                    )
                )
            elif str(secret_ref) in seen_refs:
                findings.append(
                    _finding("duplicate_secret_ref", f"{binding_path}.secret_ref", f"duplicate secret_ref: {secret_ref}")
                )
            else:
                seen_refs.add(str(secret_ref))

            if binding.get("provider") not in SECRET_BINDING_PROVIDERS:
                findings.append(
                    _finding(
                        "invalid_secret_provider",
                        f"{binding_path}.provider",
                        f"provider must be one of {sorted(SECRET_BINDING_PROVIDERS)}",
                    )
                )
            if not _is_string_list(binding.get("command")):
                findings.append(
                    _finding(
                        "invalid_secret_command",
                        f"{binding_path}.command",
                        "command must be a non-empty list of non-empty strings",
                    )
                )
            if "requires_env" in binding and not isinstance(binding.get("requires_env"), list):
                findings.append(
                    _finding("invalid_requires_env", f"{binding_path}.requires_env", "requires_env must be a list")
                )
            elif "requires_env" in binding:
                for env_index, env_name in enumerate(binding.get("requires_env", [])):
                    if not _is_string(env_name):
                        findings.append(
                            _finding(
                                "invalid_requires_env",
                                f"{binding_path}.requires_env[{env_index}]",
                                "requires_env entries must be non-empty strings",
                            )
                )
            if binding.get("interactive") is not False:
                findings.append(
                    _finding(
                        "interactive_secret_binding",
                        f"{binding_path}.interactive",
                        "secret bindings used by scheduled steward runs must explicitly set interactive to false",
                    )
                )

    runtime_env = payload.get("runtime_env", [])
    if not isinstance(runtime_env, list):
        findings.append(_finding("runtime_env", "$.runtime_env", "runtime_env must be a list when present"))
    else:
        seen_names: set[str] = set()
        for index, binding in enumerate(runtime_env):
            binding_path = f"$.runtime_env[{index}]"
            if not isinstance(binding, dict):
                findings.append(_finding("runtime_env", binding_path, "runtime_env entries must be objects"))
                continue
            name = binding.get("name")
            if not _is_string(name) or not ENV_NAME_PATTERN.match(str(name)):
                findings.append(
                    _finding(
                        "invalid_runtime_env_name",
                        f"{binding_path}.name",
                        "runtime env name must be a valid environment variable name",
                    )
                )
            elif str(name) in seen_names:
                findings.append(_finding("duplicate_runtime_env_name", f"{binding_path}.name", f"duplicate name: {name}"))
            else:
                seen_names.add(str(name))
            if binding.get("provider") not in RUNTIME_ENV_PROVIDERS:
                findings.append(
                    _finding(
                        "invalid_runtime_env_provider",
                        f"{binding_path}.provider",
                        f"provider must be one of {sorted(RUNTIME_ENV_PROVIDERS)}",
                    )
                )
            if not _is_string_list(binding.get("command")):
                findings.append(
                    _finding(
                        "invalid_runtime_env_command",
                        f"{binding_path}.command",
                        "command must be a non-empty list of non-empty strings",
                    )
                )
            if binding.get("interactive") is not False:
                findings.append(
                    _finding(
                        "interactive_runtime_env_binding",
                        f"{binding_path}.interactive",
                        "runtime env bindings used by scheduled steward runs must explicitly set interactive to false",
                    )
                )

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_repository_bindings(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != REPOSITORY_BINDINGS_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {REPOSITORY_BINDINGS_SCHEMA}"))

    bindings = payload.get("bindings")
    if not _is_object_list(bindings):
        findings.append(_finding("bindings", "$.bindings", "bindings must be a list of objects"))
    else:
        seen_refs: set[str] = set()
        for index, binding in enumerate(bindings):
            binding_path = f"$.bindings[{index}]"
            repository_ref = binding.get("repository_ref")
            if not _is_string(repository_ref):
                findings.append(
                    _finding("missing_string", f"{binding_path}.repository_ref", "repository_ref is required")
                )
            elif not str(repository_ref).startswith(REPOSITORY_REF_PREFIXES):
                findings.append(
                    _finding(
                        "invalid_repository_ref",
                        f"{binding_path}.repository_ref",
                        f"repository_ref must use one of {REPOSITORY_REF_PREFIXES}",
                    )
                )
            elif str(repository_ref) in seen_refs:
                findings.append(
                    _finding(
                        "duplicate_repository_ref",
                        f"{binding_path}.repository_ref",
                        f"duplicate repository_ref: {repository_ref}",
                    )
                )
            else:
                seen_refs.add(str(repository_ref))
            if not _is_string(binding.get("target")):
                findings.append(_finding("missing_string", f"{binding_path}.target", "target is required"))

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def resolve_secret_binding(secret_ref: str, bindings: dict[str, Any] | None) -> dict[str, Any] | None:
    if not bindings:
        return None
    for binding in bindings.get("bindings", []):
        if isinstance(binding, dict) and binding.get("secret_ref") == secret_ref:
            return binding
    return None


def secret_binding_command(secret_ref: str, bindings: dict[str, Any] | None) -> list[str] | None:
    binding = resolve_secret_binding(secret_ref, bindings)
    if not binding:
        return None
    command = binding.get("command")
    if not _is_string_list(command):
        return None
    return [str(part) for part in command]


def resolve_runtime_env_binding(name: str, bindings: dict[str, Any] | None) -> dict[str, Any] | None:
    if not bindings:
        return None
    for binding in bindings.get("runtime_env", []):
        if isinstance(binding, dict) and binding.get("name") == name:
            return binding
    return None


def runtime_env_command(name: str, bindings: dict[str, Any] | None) -> list[str] | None:
    binding = resolve_runtime_env_binding(name, bindings)
    if not binding:
        return None
    command = binding.get("command")
    if not _is_string_list(command):
        return None
    return [str(part) for part in command]


def resolve_repository_binding(repository_ref: str, bindings: dict[str, Any] | None) -> dict[str, Any] | None:
    if not bindings:
        return None
    for binding in bindings.get("bindings", []):
        if isinstance(binding, dict) and binding.get("repository_ref") == repository_ref:
            return binding
    return None


def repository_binding_target(repository_ref: str, bindings: dict[str, Any] | None) -> str | None:
    binding = resolve_repository_binding(repository_ref, bindings)
    if not binding:
        return None
    target = binding.get("target")
    if not _is_string(target):
        return None
    return str(target)


def evaluate_retention(
    policy: dict[str, Any],
    *,
    snapshot_id: str,
    available_evidence: list[str],
) -> dict[str, Any]:
    findings = validate_custody_policy(policy)
    if findings:
        details = "; ".join(f"{finding.path}: {finding.detail}" for finding in findings)
        raise ValueError(f"cannot evaluate retention from invalid policy: {details}")
    required = list(policy["retention"]["prune_requires"])
    missing = [item for item in required if item not in available_evidence]
    return {
        "schema_version": RETENTION_DECISION_SCHEMA,
        "policy_id": policy["policy_id"],
        "workspace_id": policy["workspace_id"],
        "snapshot_id": snapshot_id,
        "allowed": not missing,
        "required_evidence": required,
        "available_evidence": list(available_evidence),
        "missing_evidence": missing,
    }


def validate_retention_decision(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != RETENTION_DECISION_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {RETENTION_DECISION_SCHEMA}"))
    findings.extend(_require_string(payload, "policy_id", "$"))
    findings.extend(_require_string(payload, "workspace_id", "$"))
    findings.extend(_require_string(payload, "snapshot_id", "$"))
    if not isinstance(payload.get("allowed"), bool):
        findings.append(_finding("allowed", "$.allowed", "allowed must be a boolean"))
    for key in ("required_evidence", "available_evidence", "missing_evidence"):
        if not isinstance(payload.get(key), list):
            findings.append(_finding(key, f"$.{key}", f"{key} must be a list"))
    if payload.get("allowed") is True and payload.get("missing_evidence"):
        findings.append(
            _finding(
                "allowed_with_missing_evidence",
                "$.missing_evidence",
                "retention decision cannot allow prune while evidence is missing",
            )
        )
    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_command_plan(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != COMMAND_PLAN_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {COMMAND_PLAN_SCHEMA}"))
    findings.extend(_require_string(payload, "operation", "$"))
    if not isinstance(payload.get("ok"), bool):
        findings.append(_finding("ok", "$.ok", "ok must be a boolean"))
    if payload.get("argv_style") != "argv":
        findings.append(_finding("argv_style", "$.argv_style", "command plans must use argv style"))
    if payload.get("shell_required") is not False:
        findings.append(_finding("shell_required", "$.shell_required", "command plans must not require a shell"))
    if not isinstance(payload.get("execute_requested"), bool):
        findings.append(_finding("execute_requested", "$.execute_requested", "execute_requested must be a boolean"))
    if not isinstance(payload.get("force_requested"), bool):
        findings.append(_finding("force_requested", "$.force_requested", "force_requested must be a boolean"))
    if not isinstance(payload.get("skip_preflight_requested"), bool):
        findings.append(
            _finding(
                "skip_preflight_requested",
                "$.skip_preflight_requested",
                "skip_preflight_requested must be a boolean",
            )
        )
    if not isinstance(payload.get("requires_preflight"), bool):
        findings.append(_finding("requires_preflight", "$.requires_preflight", "requires_preflight must be a boolean"))
    if payload.get("preflight_ready") is not None and not isinstance(payload.get("preflight_ready"), bool):
        findings.append(
            _finding("preflight_ready", "$.preflight_ready", "preflight_ready must be a boolean or null")
        )
    for key in ("preflight_failed_codes", "missing_inputs", "refused_reasons", "warnings"):
        if not isinstance(payload.get(key), list):
            findings.append(_finding(key, f"$.{key}", f"{key} must be a list"))
    argv = payload.get("argv")
    if argv is not None and not _is_string_list(argv):
        findings.append(_finding("argv", "$.argv", "argv must be null or a non-empty list of non-empty strings"))
    if payload.get("ok") is True:
        if not _is_string_list(argv):
            findings.append(_finding("ok_without_argv", "$.argv", "ok command plans must include argv"))
        if payload.get("missing_inputs"):
            findings.append(_finding("ok_with_missing_inputs", "$.missing_inputs", "ok command plans cannot miss inputs"))
        if payload.get("refused_reasons"):
            findings.append(
                _finding("ok_with_refused_reasons", "$.refused_reasons", "ok command plans cannot be refused")
            )
    side_effects = payload.get("side_effects")
    if not isinstance(side_effects, dict):
        findings.append(_finding("side_effects", "$.side_effects", "side_effects must be an object"))
    else:
        for key in ("writes_run_summary", "mutates_backup_repository", "delegates_platform_scheduler_mutation"):
            if not isinstance(side_effects.get(key), bool):
                findings.append(_finding(key, f"$.side_effects.{key}", f"{key} must be a boolean"))
    guidance = payload.get("agent_guidance")
    if not isinstance(guidance, dict):
        findings.append(_finding("agent_guidance", "$.agent_guidance", "agent_guidance must be an object"))
    else:
        for key in (
            "execute_requires_explicit_flag",
            "bind_placeholders_before_invocation",
            "do_not_shell_join_argv",
            "do_not_read_or_log_secret_values",
        ):
            if guidance.get(key) is not True:
                findings.append(_finding(key, f"$.agent_guidance.{key}", f"{key} must be true"))
    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def build_run_summary(
    *,
    run_id: str,
    workspace_id: str,
    status: str,
    snapshot_result: dict[str, Any],
    verification_result: dict[str, Any],
    retention_decision: dict[str, Any] | None = None,
    tool_invocations: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": RUN_SUMMARY_SCHEMA,
        "run_id": run_id,
        "workspace_id": workspace_id,
        "status": status,
        "snapshot_result": snapshot_result,
        "verification_result": verification_result,
        "retention_decision": retention_decision,
        "tool_invocations": list(tool_invocations or []),
    }


def validate_document(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    schema = payload.get("schema_version")
    if schema == INVENTORY_SCHEMA:
        return validate_workspace_inventory(payload, public_safe=public_safe)
    if schema == POLICY_SCHEMA:
        return validate_custody_policy(payload, public_safe=public_safe)
    if schema == SNAPSHOT_PLAN_SCHEMA:
        return validate_snapshot_plan(payload, public_safe=public_safe)
    if schema == VERIFICATION_RESULT_SCHEMA:
        return validate_verification_result(payload, public_safe=public_safe)
    if schema == RETENTION_DECISION_SCHEMA:
        return validate_retention_decision(payload, public_safe=public_safe)
    if schema == RUN_SUMMARY_SCHEMA:
        return validate_run_summary(payload, public_safe=public_safe)
    if schema == SECRET_BINDINGS_SCHEMA:
        return validate_secret_bindings(payload, public_safe=public_safe)
    if schema == REPOSITORY_BINDINGS_SCHEMA:
        return validate_repository_bindings(payload, public_safe=public_safe)
    if schema == COMMAND_PLAN_SCHEMA:
        return validate_command_plan(payload, public_safe=public_safe)
    return [_finding("unknown_schema", "$.schema_version", f"unsupported custody schema: {schema}")]
