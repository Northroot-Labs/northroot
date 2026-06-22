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
SERVICE_REGISTRY_SCHEMA = "northroot.steward.service-registry.v0"
LEGACY_PROFILE_IMPORT_SCHEMA = "northroot.steward.legacy-profile-import.v0"
LEGACY_RUN_IMPORT_SCHEMA = "northroot.steward.legacy-run-import.v0"
AGENT_DELEGATION_POLICY_SCHEMA = "northroot.steward.agent-delegation-policy.v0"

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
RUN_ID_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.:-]*$")
REPOSITORY_CHECK_STATUSES = {"ok", "failed", "not-run"}
SERVICE_REF_PREFIXES = STORAGE_BINDING_PREFIXES + (
    "logs://",
    "node://",
    "project://",
    "receipt://",
    "run-state://",
    "scheduler://",
    "state://",
)
SERVICE_DESTINATION_ROLES = {"primary", "replica", "source", "receipt-log"}
SERVICE_ADAPTERS = ADAPTERS | {"external-delegated", "filesystem", "provider-native"}
SERVICE_PERMISSION_SCOPES = {"project", "object"}
SERVICE_PERMISSION_OPERATIONS = {
    "evidence.record",
    "evidence.report",
    "legacy.import",
    "offsite.report",
    "preflight",
    "replica.sync",
    "report",
    "restore",
    "restore-drill",
    "retention.evaluate",
    "run",
    "schedule.create",
    "schedule.delete",
    "schedule.install",
    "schedule.status",
    "schedule.uninstall",
    "source.bind",
    "status",
    "verify",
    "verify-state",
}
RESUME_LOCK_STRATEGIES = {"operation-lock-file", "scheduler-singleflight", "external-lock"}
RESUME_FAILURE_POLICIES = {
    "fail-closed-record-summary",
    "resume-by-run-id",
    "require-human-review",
}
PARTIAL_RUN_HANDLING = {
    "never-prune-without-retention-decision",
    "rerun-idempotent-operation",
    "record-and-hold",
}
AGENT_DELEGATION_OPERATIONS = {
    "branch.checkout",
    "branch.create",
    "commit.create",
    "commit.verify",
    "pr.check.verify",
    "pr.comment.follow-up",
    "pr.draft.open",
    "pr.draft.update",
    "push.branch",
}
AGENT_COAUTHORSHIP_POLICIES = {
    "agent-authored",
    "human-authored-agent-assisted",
    "mixed-authorship-explicit",
}

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


def _validate_symbolic_ref(value: Any, *, path: str, name: str) -> list[Finding]:
    if not _is_string(value):
        return [_finding("missing_string", path, f"{name} is required")]
    if not str(value).startswith(SERVICE_REF_PREFIXES):
        return [
            _finding(
                "invalid_symbolic_ref",
                path,
                f"{name} must use one of {SERVICE_REF_PREFIXES}",
            )
        ]
    return []


def _validate_permission_operations(value: Any, *, path: str) -> list[Finding]:
    findings: list[Finding] = []
    if not isinstance(value, list) or not value:
        return [_finding("permission_operations", path, "permission operations must be a non-empty list")]
    for index, operation in enumerate(value):
        if operation not in SERVICE_PERMISSION_OPERATIONS:
            findings.append(
                _finding(
                    "invalid_permission_operation",
                    f"{path}[{index}]",
                    f"operation must be one of {sorted(SERVICE_PERMISSION_OPERATIONS)}",
                )
            )
    return findings


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
        if payload.get("status") not in {"external-evidence-recorded", "legacy-run-imported"} and verification.get(
            "restore_verified"
        ) is not True:
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


def validate_service_registry(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != SERVICE_REGISTRY_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {SERVICE_REGISTRY_SCHEMA}"))
    findings.extend(_require_string(payload, "service_id", "$"))
    findings.extend(_require_string(payload, "node_id", "$"))
    findings.extend(_validate_symbolic_ref(payload.get("node_ref"), path="$.node_ref", name="node_ref"))

    objects = payload.get("objects")
    if not _is_object_list(objects):
        findings.append(_finding("objects", "$.objects", "objects must be a non-empty list of custody objects"))
    elif not objects:
        findings.append(_finding("objects", "$.objects", "objects must be a non-empty list of custody objects"))
    findings.extend(_validate_object_custody(objects, path="$.objects"))
    object_ids = {
        str(obj.get("object_id"))
        for obj in objects or []
        if isinstance(obj, dict) and _is_string(obj.get("object_id"))
    }

    destinations = payload.get("destinations")
    destination_ids: set[str] = set()
    if not _is_object_list(destinations):
        findings.append(_finding("destinations", "$.destinations", "destinations must be a list of objects"))
    else:
        for index, destination in enumerate(destinations):
            dest_path = f"$.destinations[{index}]"
            destination_id = destination.get("destination_id")
            if not _is_string(destination_id):
                findings.append(_finding("missing_string", f"{dest_path}.destination_id", "destination_id is required"))
            elif str(destination_id) in destination_ids:
                findings.append(
                    _finding(
                        "duplicate_destination_id",
                        f"{dest_path}.destination_id",
                        f"duplicate destination_id: {destination_id}",
                    )
                )
            else:
                destination_ids.add(str(destination_id))
            if destination.get("role") not in SERVICE_DESTINATION_ROLES:
                findings.append(
                    _finding(
                        "invalid_destination_role",
                        f"{dest_path}.role",
                        f"role must be one of {sorted(SERVICE_DESTINATION_ROLES)}",
                    )
                )
            if destination.get("adapter") not in SERVICE_ADAPTERS:
                findings.append(
                    _finding(
                        "invalid_destination_adapter",
                        f"{dest_path}.adapter",
                        f"adapter must be one of {sorted(SERVICE_ADAPTERS)}",
                    )
                )
            findings.extend(
                _validate_symbolic_ref(
                    destination.get("storage_binding"),
                    path=f"{dest_path}.storage_binding",
                    name="storage_binding",
                )
            )
            if destination.get("visibility") not in VISIBILITY_CLASSES:
                findings.append(
                    _finding(
                        "invalid_visibility",
                        f"{dest_path}.visibility",
                        f"visibility must be one of {sorted(VISIBILITY_CLASSES)}",
                    )
                )

    permissions = payload.get("permissions")
    permission_set_ids: set[str] = set()
    permission_scopes: dict[str, str] = {}
    permission_project_ids: dict[str, str] = {}
    project_permission_refs: list[tuple[str, str]] = []
    if not _is_object_list(permissions):
        findings.append(_finding("permissions", "$.permissions", "permissions must be a list of objects"))
    else:
        for index, permission in enumerate(permissions):
            perm_path = f"$.permissions[{index}]"
            permission_set_id = permission.get("permission_set_id")
            scope = permission.get("scope")
            if not _is_string(permission_set_id):
                findings.append(
                    _finding("missing_string", f"{perm_path}.permission_set_id", "permission_set_id is required")
                )
            elif str(permission_set_id) in permission_set_ids:
                findings.append(
                    _finding(
                        "duplicate_permission_set_id",
                        f"{perm_path}.permission_set_id",
                        f"duplicate permission_set_id: {permission_set_id}",
                    )
                )
            else:
                permission_set_ids.add(str(permission_set_id))
                permission_scopes[str(permission_set_id)] = str(scope)
            if scope not in SERVICE_PERMISSION_SCOPES:
                findings.append(
                    _finding(
                        "invalid_permission_scope",
                        f"{perm_path}.scope",
                        f"scope must be one of {sorted(SERVICE_PERMISSION_SCOPES)}",
                    )
                )
            if scope == "object":
                object_id = permission.get("object_id")
                if object_id not in object_ids:
                    findings.append(
                        _finding(
                            "unknown_permission_object",
                            f"{perm_path}.object_id",
                            f"unknown object_id: {object_id}",
                        )
                    )
            elif scope == "project" and not _is_string(permission.get("project_id")):
                findings.append(_finding("missing_string", f"{perm_path}.project_id", "project_id is required"))
            elif scope == "project":
                project_permission_refs.append((perm_path, str(permission.get("project_id"))))
                if _is_string(permission_set_id):
                    permission_project_ids[str(permission_set_id)] = str(permission.get("project_id"))
            findings.extend(
                _validate_permission_operations(
                    permission.get("allowed_operations"),
                    path=f"{perm_path}.allowed_operations",
                )
            )
            blocked = permission.get("blocked_operations", [])
            if blocked is not None:
                findings.extend(
                    _validate_permission_operations(
                        blocked,
                        path=f"{perm_path}.blocked_operations",
                    )
                    if blocked
                    else []
                )
            clearance = permission.get("requires_human_clearance", [])
            if clearance is not None:
                findings.extend(
                    _validate_permission_operations(
                        clearance,
                        path=f"{perm_path}.requires_human_clearance",
                    )
                    if clearance
                    else []
                )

    projects = payload.get("projects")
    project_ids: set[str] = set()
    if not _is_object_list(projects):
        findings.append(_finding("projects", "$.projects", "projects must be a list of objects"))
    else:
        for index, project in enumerate(projects):
            project_path = f"$.projects[{index}]"
            project_id = project.get("project_id")
            if not _is_string(project_id):
                findings.append(_finding("missing_string", f"{project_path}.project_id", "project_id is required"))
            elif str(project_id) in project_ids:
                findings.append(
                    _finding("duplicate_project_id", f"{project_path}.project_id", f"duplicate project_id: {project_id}")
                )
            else:
                project_ids.add(str(project_id))
            findings.extend(_require_string(project, "workspace_id", project_path))
            project_node_ref = project.get("node_ref")
            if project_node_ref is not None:
                findings.extend(
                    _validate_symbolic_ref(project_node_ref, path=f"{project_path}.node_ref", name="node_ref")
                )
            permission_set_ref = project.get("permission_set_ref")
            if permission_set_ref not in permission_set_ids:
                findings.append(
                    _finding(
                        "unknown_project_permission_set",
                        f"{project_path}.permission_set_ref",
                        f"unknown permission_set_ref: {permission_set_ref}",
                    )
                )
            elif permission_scopes.get(str(permission_set_ref)) != "project":
                findings.append(
                    _finding(
                        "invalid_project_permission_scope",
                        f"{project_path}.permission_set_ref",
                        "project permission_set_ref must reference a project-scoped permission set",
                    )
                )
            elif permission_project_ids.get(str(permission_set_ref)) != project_id:
                findings.append(
                    _finding(
                        "mismatched_project_permission",
                        f"{project_path}.permission_set_ref",
                        "project permission_set_ref must reference a permission set for the same project_id",
                    )
                )
            if not isinstance(project.get("object_ids"), list) or not project.get("object_ids"):
                findings.append(
                    _finding("project_object_ids", f"{project_path}.object_ids", "project must name at least one object")
                )
            else:
                for obj_index, object_id in enumerate(project["object_ids"]):
                    if object_id not in object_ids:
                        findings.append(
                            _finding(
                                "unknown_project_object",
                                f"{project_path}.object_ids[{obj_index}]",
                                f"unknown object_id: {object_id}",
                            )
                        )

    for perm_path, project_id in project_permission_refs:
        if project_id not in project_ids:
            findings.append(
                _finding(
                    "unknown_permission_project",
                    f"{perm_path}.project_id",
                    f"unknown project_id: {project_id}",
                )
            )

    source_destinations = payload.get("source_destinations")
    source_destination_ids: set[str] = set()
    if not _is_object_list(source_destinations):
        findings.append(
            _finding(
                "source_destinations",
                "$.source_destinations",
                "source_destinations must be a list of objects",
            )
        )
    else:
        for index, binding in enumerate(source_destinations):
            binding_path = f"$.source_destinations[{index}]"
            binding_id = binding.get("source_destination_id")
            if not _is_string(binding_id):
                findings.append(
                    _finding(
                        "missing_string",
                        f"{binding_path}.source_destination_id",
                        "source_destination_id is required",
                    )
                )
            elif str(binding_id) in source_destination_ids:
                findings.append(
                    _finding(
                        "duplicate_source_destination_id",
                        f"{binding_path}.source_destination_id",
                        f"duplicate source_destination_id: {binding_id}",
                    )
                )
            else:
                source_destination_ids.add(str(binding_id))
            if binding.get("project_id") not in project_ids:
                findings.append(
                    _finding(
                        "unknown_source_project",
                        f"{binding_path}.project_id",
                        f"unknown project_id: {binding.get('project_id')}",
                    )
                )
            if binding.get("destination_id") not in destination_ids:
                findings.append(
                    _finding(
                        "unknown_source_destination",
                        f"{binding_path}.destination_id",
                        f"unknown destination_id: {binding.get('destination_id')}",
                    )
                )
            if binding.get("permission_set_ref") not in permission_set_ids:
                findings.append(
                    _finding(
                        "unknown_source_permission_set",
                        f"{binding_path}.permission_set_ref",
                        f"unknown permission_set_ref: {binding.get('permission_set_ref')}",
                    )
                )
            if not isinstance(binding.get("object_ids"), list) or not binding.get("object_ids"):
                findings.append(
                    _finding(
                        "source_object_ids",
                        f"{binding_path}.object_ids",
                        "source destination must name at least one object",
                    )
                )
            else:
                for obj_index, object_id in enumerate(binding["object_ids"]):
                    if object_id not in object_ids:
                        findings.append(
                            _finding(
                                "unknown_source_object",
                                f"{binding_path}.object_ids[{obj_index}]",
                                f"unknown object_id: {object_id}",
                            )
                        )

    resume_policy = payload.get("resume_policy")
    resume_policy_id = None
    if not isinstance(resume_policy, dict):
        findings.append(_finding("resume_policy", "$.resume_policy", "resume_policy must be an object"))
    else:
        resume_policy_id = resume_policy.get("policy_id")
        findings.extend(_require_string(resume_policy, "policy_id", "$.resume_policy"))
        if resume_policy.get("lock_strategy") not in RESUME_LOCK_STRATEGIES:
            findings.append(
                _finding(
                    "invalid_lock_strategy",
                    "$.resume_policy.lock_strategy",
                    f"lock_strategy must be one of {sorted(RESUME_LOCK_STRATEGIES)}",
                )
            )
        for key in ("on_disconnected_storage", "on_power_loss", "on_interrupted_run"):
            if resume_policy.get(key) not in RESUME_FAILURE_POLICIES:
                findings.append(
                    _finding(
                        "invalid_resume_failure_policy",
                        f"$.resume_policy.{key}",
                        f"{key} must be one of {sorted(RESUME_FAILURE_POLICIES)}",
                    )
                )
        if resume_policy.get("partial_run_handling") not in PARTIAL_RUN_HANDLING:
            findings.append(
                _finding(
                    "invalid_partial_run_handling",
                    "$.resume_policy.partial_run_handling",
                    f"partial_run_handling must be one of {sorted(PARTIAL_RUN_HANDLING)}",
                )
            )

    replicas = payload.get("replicas")
    if not _is_object_list(replicas):
        findings.append(_finding("replicas", "$.replicas", "replicas must be a list of objects"))
    else:
        seen_replicas: set[str] = set()
        for index, replica in enumerate(replicas):
            replica_path = f"$.replicas[{index}]"
            replica_id = replica.get("replica_id")
            if not _is_string(replica_id):
                findings.append(_finding("missing_string", f"{replica_path}.replica_id", "replica_id is required"))
            elif str(replica_id) in seen_replicas:
                findings.append(
                    _finding("duplicate_replica_id", f"{replica_path}.replica_id", f"duplicate replica_id: {replica_id}")
                )
            else:
                seen_replicas.add(str(replica_id))
            if replica.get("source_destination_id") not in source_destination_ids:
                findings.append(
                    _finding(
                        "unknown_replica_source_destination",
                        f"{replica_path}.source_destination_id",
                        f"unknown source_destination_id: {replica.get('source_destination_id')}",
                    )
                )
            if replica.get("destination_id") not in destination_ids:
                findings.append(
                    _finding(
                        "unknown_replica_destination",
                        f"{replica_path}.destination_id",
                        f"unknown destination_id: {replica.get('destination_id')}",
                    )
                )
            if replica.get("execution_model") != "external-delegated":
                findings.append(
                    _finding(
                        "invalid_replica_execution_model",
                        f"{replica_path}.execution_model",
                        "replica execution_model must be external-delegated",
                    )
                )
            if replica.get("resume_policy_ref") != resume_policy_id:
                findings.append(
                    _finding(
                        "unknown_replica_resume_policy",
                        f"{replica_path}.resume_policy_ref",
                        f"unknown resume_policy_ref: {replica.get('resume_policy_ref')}",
                    )
                )
            evidence = replica.get("required_evidence")
            if not isinstance(evidence, list) or not evidence:
                findings.append(
                    _finding(
                        "replica_required_evidence",
                        f"{replica_path}.required_evidence",
                        "replicas must name required evidence",
                    )
                )
            else:
                for evidence_index, item in enumerate(evidence):
                    if item not in RETENTION_EVIDENCE:
                        findings.append(
                            _finding(
                                "invalid_replica_evidence",
                                f"{replica_path}.required_evidence[{evidence_index}]",
                                f"evidence must be one of {sorted(RETENTION_EVIDENCE)}",
                            )
                        )

    legacy_imports = payload.get("legacy_imports", [])
    if not isinstance(legacy_imports, list):
        findings.append(_finding("legacy_imports", "$.legacy_imports", "legacy_imports must be a list when present"))
    else:
        for index, legacy_import in enumerate(legacy_imports):
            legacy_path = f"$.legacy_imports[{index}]"
            if not isinstance(legacy_import, dict):
                findings.append(_finding("legacy_imports", legacy_path, "legacy imports must be objects"))
                continue
            findings.extend(_require_string(legacy_import, "import_id", legacy_path))
            if legacy_import.get("source") != "legacy-machine-durability":
                findings.append(
                    _finding(
                        "invalid_legacy_import_source",
                        f"{legacy_path}.source",
                        "legacy import source must be legacy-machine-durability",
                    )
                )
            for key in (
                "scheduler_ref",
                "machine_node_ref",
                "project_nodes_ref",
                "runner_state_ref",
                "per_run_state_ref",
            ):
                findings.extend(
                    _validate_symbolic_ref(legacy_import.get(key), path=f"{legacy_path}.{key}", name=key)
                )
            if legacy_import.get("import_mode") not in {"metadata-only", "sanitized-run-summaries"}:
                findings.append(
                    _finding(
                        "invalid_legacy_import_mode",
                        f"{legacy_path}.import_mode",
                        "import_mode must be metadata-only or sanitized-run-summaries",
                    )
                )
            if legacy_import.get("status") not in {"pending", "imported", "blocked"}:
                findings.append(
                    _finding(
                        "invalid_legacy_import_status",
                        f"{legacy_path}.status",
                        "status must be pending, imported, or blocked",
                    )
                )

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def _validate_legacy_import_record(legacy_import: dict[str, Any], *, path: str) -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(_require_string(legacy_import, "import_id", path))
    if legacy_import.get("source") != "legacy-machine-durability":
        findings.append(
            _finding(
                "invalid_legacy_import_source",
                f"{path}.source",
                "legacy import source must be legacy-machine-durability",
            )
        )
    for key in (
        "scheduler_ref",
        "machine_node_ref",
        "project_nodes_ref",
        "runner_state_ref",
        "per_run_state_ref",
    ):
        findings.extend(_validate_symbolic_ref(legacy_import.get(key), path=f"{path}.{key}", name=key))
    if legacy_import.get("import_mode") not in {"metadata-only", "sanitized-run-summaries"}:
        findings.append(
            _finding(
                "invalid_legacy_import_mode",
                f"{path}.import_mode",
                "import_mode must be metadata-only or sanitized-run-summaries",
            )
        )
    if legacy_import.get("status") not in {"pending", "imported", "blocked"}:
        findings.append(
            _finding(
                "invalid_legacy_import_status",
                f"{path}.status",
                "status must be pending, imported, or blocked",
            )
        )
    return findings


def _validate_legacy_batch_list(
    payload: dict[str, Any],
    key: str,
    identity_key: str,
    *,
    required: bool = True,
) -> tuple[list[Finding], list[dict[str, Any]]]:
    findings: list[Finding] = []
    values = payload.get(key)
    if not isinstance(values, list):
        findings.append(_finding(key, f"$.{key}", f"{key} must be a list"))
        return findings, []
    if required and not values:
        findings.append(_finding(key, f"$.{key}", f"{key} must not be empty"))
    seen: set[str] = set()
    objects: list[dict[str, Any]] = []
    for index, value in enumerate(values):
        item_path = f"$.{key}[{index}]"
        if not isinstance(value, dict):
            findings.append(_finding(key, item_path, f"{key} entries must be objects"))
            continue
        objects.append(value)
        identity = value.get(identity_key)
        if not _is_string(identity):
            findings.append(_finding("missing_string", f"{item_path}.{identity_key}", f"{identity_key} is required"))
        elif str(identity) in seen:
            findings.append(
                _finding(
                    f"duplicate_{identity_key}",
                    f"{item_path}.{identity_key}",
                    f"duplicate {identity_key}: {identity}",
                )
            )
        else:
            seen.add(str(identity))
    return findings, objects


def validate_legacy_profile_import(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != LEGACY_PROFILE_IMPORT_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {LEGACY_PROFILE_IMPORT_SCHEMA}"))
    findings.extend(_require_string(payload, "import_id", "$"))
    if payload.get("source") != "legacy-machine-durability":
        findings.append(
            _finding(
                "invalid_legacy_import_source",
                "$.source",
                "legacy profile import source must be legacy-machine-durability",
            )
        )
    if payload.get("import_mode") not in {"metadata-only", "sanitized-run-summaries"}:
        findings.append(
            _finding(
                "invalid_legacy_import_mode",
                "$.import_mode",
                "import_mode must be metadata-only or sanitized-run-summaries",
            )
        )

    list_specs = (
        ("objects", "object_id", True),
        ("permissions", "permission_set_id", True),
        ("projects", "project_id", True),
        ("destinations", "destination_id", True),
        ("source_destinations", "source_destination_id", True),
        ("replicas", "replica_id", False),
        ("legacy_imports", "import_id", True),
    )
    parsed: dict[str, list[dict[str, Any]]] = {}
    for key, identity_key, required in list_specs:
        list_findings, objects = _validate_legacy_batch_list(payload, key, identity_key, required=required)
        findings.extend(list_findings)
        parsed[key] = objects

    findings.extend(_validate_object_custody(parsed.get("objects", []), path="$.objects"))

    for index, destination in enumerate(parsed.get("destinations", [])):
        dest_path = f"$.destinations[{index}]"
        if destination.get("role") not in SERVICE_DESTINATION_ROLES:
            findings.append(
                _finding(
                    "invalid_destination_role",
                    f"{dest_path}.role",
                    f"role must be one of {sorted(SERVICE_DESTINATION_ROLES)}",
                )
            )
        if destination.get("adapter") not in SERVICE_ADAPTERS:
            findings.append(
                _finding(
                    "invalid_destination_adapter",
                    f"{dest_path}.adapter",
                    f"adapter must be one of {sorted(SERVICE_ADAPTERS)}",
                )
            )
        findings.extend(
            _validate_symbolic_ref(
                destination.get("storage_binding"),
                path=f"{dest_path}.storage_binding",
                name="storage_binding",
            )
        )
        if destination.get("visibility") not in VISIBILITY_CLASSES:
            findings.append(
                _finding(
                    "invalid_visibility",
                    f"{dest_path}.visibility",
                    f"visibility must be one of {sorted(VISIBILITY_CLASSES)}",
                )
            )

    for index, permission in enumerate(parsed.get("permissions", [])):
        perm_path = f"$.permissions[{index}]"
        if permission.get("scope") not in SERVICE_PERMISSION_SCOPES:
            findings.append(
                _finding(
                    "invalid_permission_scope",
                    f"{perm_path}.scope",
                    f"scope must be one of {sorted(SERVICE_PERMISSION_SCOPES)}",
                )
            )
        findings.extend(
            _validate_permission_operations(
                permission.get("allowed_operations"),
                path=f"{perm_path}.allowed_operations",
            )
        )
        for key in ("blocked_operations", "requires_human_clearance"):
            values = permission.get(key, [])
            if values:
                findings.extend(_validate_permission_operations(values, path=f"{perm_path}.{key}"))

    for index, project in enumerate(parsed.get("projects", [])):
        project_path = f"$.projects[{index}]"
        findings.extend(_require_string(project, "workspace_id", project_path))
        if project.get("node_ref") is not None:
            findings.extend(
                _validate_symbolic_ref(project.get("node_ref"), path=f"{project_path}.node_ref", name="node_ref")
            )
        findings.extend(_require_string(project, "permission_set_ref", project_path))
        if not isinstance(project.get("object_ids"), list) or not project.get("object_ids"):
            findings.append(
                _finding("project_object_ids", f"{project_path}.object_ids", "project must name at least one object")
            )

    for index, binding in enumerate(parsed.get("source_destinations", [])):
        binding_path = f"$.source_destinations[{index}]"
        for key in ("project_id", "destination_id", "permission_set_ref"):
            findings.extend(_require_string(binding, key, binding_path))
        if not isinstance(binding.get("object_ids"), list) or not binding.get("object_ids"):
            findings.append(
                _finding(
                    "source_object_ids",
                    f"{binding_path}.object_ids",
                    "source destination must name at least one object",
                )
            )

    for index, replica in enumerate(parsed.get("replicas", [])):
        replica_path = f"$.replicas[{index}]"
        for key in ("source_destination_id", "destination_id", "resume_policy_ref"):
            findings.extend(_require_string(replica, key, replica_path))
        if replica.get("execution_model") != "external-delegated":
            findings.append(
                _finding(
                    "invalid_replica_execution_model",
                    f"{replica_path}.execution_model",
                    "replica execution_model must be external-delegated",
                )
            )
        evidence = replica.get("required_evidence")
        if not isinstance(evidence, list) or not evidence:
            findings.append(
                _finding("replica_required_evidence", f"{replica_path}.required_evidence", "replicas must name required evidence")
            )
        else:
            for evidence_index, item in enumerate(evidence):
                if item not in RETENTION_EVIDENCE:
                    findings.append(
                        _finding(
                            "invalid_replica_evidence",
                            f"{replica_path}.required_evidence[{evidence_index}]",
                            f"evidence must be one of {sorted(RETENTION_EVIDENCE)}",
                        )
                    )

    import_ids = {str(item.get("import_id")) for item in parsed.get("legacy_imports", []) if _is_string(item.get("import_id"))}
    if _is_string(payload.get("import_id")) and str(payload.get("import_id")) not in import_ids:
        findings.append(
            _finding(
                "missing_matching_legacy_import",
                "$.legacy_imports",
                "legacy_imports must include the top-level import_id",
            )
        )
    for index, legacy_import in enumerate(parsed.get("legacy_imports", [])):
        findings.extend(_validate_legacy_import_record(legacy_import, path=f"$.legacy_imports[{index}]"))

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


def validate_legacy_run_import(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != LEGACY_RUN_IMPORT_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {LEGACY_RUN_IMPORT_SCHEMA}"))
    findings.extend(_require_string(payload, "import_id", "$"))
    if payload.get("source") != "legacy-machine-durability":
        findings.append(
            _finding(
                "invalid_legacy_import_source",
                "$.source",
                "legacy run import source must be legacy-machine-durability",
            )
        )
    if payload.get("import_mode") != "sanitized-run-summaries":
        findings.append(
            _finding(
                "invalid_legacy_import_mode",
                "$.import_mode",
                "legacy run import mode must be sanitized-run-summaries",
            )
        )
    for key in ("legacy_import_ref", "runner_state_ref", "per_run_state_ref"):
        if payload.get(key) is not None:
            findings.extend(_validate_symbolic_ref(payload.get(key), path=f"$.{key}", name=key))

    run_summaries = payload.get("run_summaries")
    run_ids: set[str] = set()
    if not isinstance(run_summaries, list):
        findings.append(_finding("run_summaries", "$.run_summaries", "run_summaries must be a non-empty list"))
    elif not run_summaries:
        findings.append(_finding("run_summaries", "$.run_summaries", "run_summaries must be a non-empty list"))
    else:
        for index, summary in enumerate(run_summaries):
            summary_path = f"$.run_summaries[{index}]"
            if not isinstance(summary, dict):
                findings.append(_finding("run_summary", summary_path, "run summary entries must be objects"))
                continue
            run_id = summary.get("run_id")
            if not _is_string(run_id):
                findings.append(_finding("missing_string", f"{summary_path}.run_id", "run_id is required"))
            elif not RUN_ID_PATTERN.match(str(run_id)):
                findings.append(
                    _finding(
                        "invalid_run_id",
                        f"{summary_path}.run_id",
                        "run_id must be filesystem-safe and may not contain path separators",
                    )
                )
            elif str(run_id) in run_ids:
                findings.append(
                    _finding("duplicate_run_id", f"{summary_path}.run_id", f"duplicate run_id: {run_id}")
                )
            else:
                run_ids.add(str(run_id))
            findings.extend(validate_run_summary(summary, public_safe=public_safe))

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


def validate_agent_delegation_policy(payload: dict[str, Any], *, public_safe: bool = False) -> list[Finding]:
    findings: list[Finding] = []
    if payload.get("schema_version") != AGENT_DELEGATION_POLICY_SCHEMA:
        findings.append(_finding("schema_version", "$.schema_version", f"expected {AGENT_DELEGATION_POLICY_SCHEMA}"))
    findings.extend(_require_string(payload, "policy_id", "$"))
    if payload.get("default_for_dogfood") is not True:
        findings.append(
            _finding(
                "default_for_dogfood",
                "$.default_for_dogfood",
                "default dogfood policy must set default_for_dogfood to true",
            )
        )

    scope = payload.get("scope")
    if not isinstance(scope, dict):
        findings.append(_finding("scope", "$.scope", "scope must be an object"))
    else:
        findings.extend(_validate_symbolic_ref(scope.get("repository_ref"), path="$.scope.repository_ref", name="repository_ref"))
        branch_prefixes = scope.get("delegated_branch_prefixes")
        if not _is_string_list(branch_prefixes):
            findings.append(
                _finding(
                    "delegated_branch_prefixes",
                    "$.scope.delegated_branch_prefixes",
                    "delegated_branch_prefixes must be a non-empty string list",
                )
            )
        else:
            for index, prefix in enumerate(branch_prefixes):
                if prefix in {"main", "master", "release"} or prefix.startswith(("main/", "master/", "release/")):
                    findings.append(
                        _finding(
                            "protected_branch_prefix",
                            f"$.scope.delegated_branch_prefixes[{index}]",
                            "delegated branch prefixes must not target protected branch namespaces",
                        )
                    )
        if scope.get("protected_base_branch") not in {"main"}:
            findings.append(
                _finding(
                    "protected_base_branch",
                    "$.scope.protected_base_branch",
                    "default dogfood policy currently expects protected_base_branch main",
                )
            )

    agents = payload.get("registered_agents")
    if not _is_object_list(agents):
        findings.append(_finding("registered_agents", "$.registered_agents", "registered_agents must be a list of objects"))
    elif not agents:
        findings.append(_finding("registered_agents", "$.registered_agents", "registered_agents must not be empty"))
    else:
        seen_agents: set[str] = set()
        for index, agent in enumerate(agents):
            agent_path = f"$.registered_agents[{index}]"
            agent_id = agent.get("agent_id")
            if not _is_string(agent_id):
                findings.append(_finding("missing_string", f"{agent_path}.agent_id", "agent_id is required"))
            elif not str(agent_id).startswith("agent:"):
                findings.append(_finding("invalid_agent_id", f"{agent_path}.agent_id", "agent_id must use agent: prefix"))
            elif str(agent_id) in seen_agents:
                findings.append(_finding("duplicate_agent_id", f"{agent_path}.agent_id", f"duplicate agent_id: {agent_id}"))
            else:
                seen_agents.add(str(agent_id))
            findings.extend(_require_string(agent, "display_name", agent_path))
            if not _is_string_list(agent.get("branch_prefixes")):
                findings.append(
                    _finding("branch_prefixes", f"{agent_path}.branch_prefixes", "branch_prefixes must be a non-empty string list")
                )
            operations = agent.get("allowed_operations")
            if not _is_string_list(operations):
                findings.append(
                    _finding(
                        "allowed_operations",
                        f"{agent_path}.allowed_operations",
                        "allowed_operations must be a non-empty string list",
                    )
                )
            else:
                for op_index, operation in enumerate(operations):
                    if operation not in AGENT_DELEGATION_OPERATIONS:
                        findings.append(
                            _finding(
                                "invalid_agent_delegation_operation",
                                f"{agent_path}.allowed_operations[{op_index}]",
                                f"operation must be one of {sorted(AGENT_DELEGATION_OPERATIONS)}",
                            )
                        )
            metadata = agent.get("required_metadata")
            if not isinstance(metadata, dict):
                findings.append(
                    _finding("required_metadata", f"{agent_path}.required_metadata", "required_metadata must be an object")
                )
            else:
                for key in ("author_identity", "committer_identity", "provenance_headers"):
                    if key == "provenance_headers":
                        if not _is_string_list(metadata.get(key)):
                            findings.append(
                                _finding(
                                    "provenance_headers",
                                    f"{agent_path}.required_metadata.provenance_headers",
                                    "provenance_headers must be a non-empty string list",
                                )
                            )
                    elif not _is_string(metadata.get(key)):
                        findings.append(
                            _finding("missing_string", f"{agent_path}.required_metadata.{key}", f"{key} is required")
                        )
                if metadata.get("coauthorship_policy") not in AGENT_COAUTHORSHIP_POLICIES:
                    findings.append(
                        _finding(
                            "invalid_coauthorship_policy",
                            f"{agent_path}.required_metadata.coauthorship_policy",
                            f"coauthorship_policy must be one of {sorted(AGENT_COAUTHORSHIP_POLICIES)}",
                        )
                    )

    draft_pr_policy = payload.get("draft_pr_policy")
    if not isinstance(draft_pr_policy, dict):
        findings.append(_finding("draft_pr_policy", "$.draft_pr_policy", "draft_pr_policy must be an object"))
    else:
        for key in ("allow_open_draft_prs", "allow_update_draft_prs", "final_human_review_required"):
            if draft_pr_policy.get(key) is not True:
                findings.append(_finding(key, f"$.draft_pr_policy.{key}", f"{key} must be true"))
        if draft_pr_policy.get("allow_ready_for_review_without_clearance") is not False:
            findings.append(
                _finding(
                    "allow_ready_for_review_without_clearance",
                    "$.draft_pr_policy.allow_ready_for_review_without_clearance",
                    "agents must not mark PRs ready for review without clearance",
                )
            )

    verification_policy = payload.get("verification_policy")
    if not isinstance(verification_policy, dict):
        findings.append(_finding("verification_policy", "$.verification_policy", "verification_policy must be an object"))
    else:
        if not _is_string_list(verification_policy.get("required_before_push")):
            findings.append(
                _finding(
                    "required_before_push",
                    "$.verification_policy.required_before_push",
                    "required_before_push must be a non-empty string list",
                )
            )
        if verification_policy.get("record_failed_checks_before_follow_up") is not True:
            findings.append(
                _finding(
                    "record_failed_checks_before_follow_up",
                    "$.verification_policy.record_failed_checks_before_follow_up",
                    "failed checks must be recorded before follow-up",
                )
            )

    prohibited = payload.get("prohibited_operations")
    if not _is_string_list(prohibited):
        findings.append(
            _finding(
                "prohibited_operations",
                "$.prohibited_operations",
                "prohibited_operations must be a non-empty string list",
            )
        )
    else:
        required_prohibitions = {
            "merge.protected-branch",
            "push.protected-branch",
            "workflow.permission-escalation",
            "impersonate-human-author",
        }
        missing = sorted(required_prohibitions - set(str(item) for item in prohibited))
        if missing:
            findings.append(
                _finding(
                    "missing_required_prohibitions",
                    "$.prohibited_operations",
                    f"missing required prohibitions: {', '.join(missing)}",
                )
            )

    if public_safe:
        findings.extend(find_public_private_bindings(payload))
    return findings


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
    if schema == SERVICE_REGISTRY_SCHEMA:
        return validate_service_registry(payload, public_safe=public_safe)
    if schema == LEGACY_PROFILE_IMPORT_SCHEMA:
        return validate_legacy_profile_import(payload, public_safe=public_safe)
    if schema == LEGACY_RUN_IMPORT_SCHEMA:
        return validate_legacy_run_import(payload, public_safe=public_safe)
    if schema == AGENT_DELEGATION_POLICY_SCHEMA:
        return validate_agent_delegation_policy(payload, public_safe=public_safe)
    return [_finding("unknown_schema", "$.schema_version", f"unsupported custody schema: {schema}")]
