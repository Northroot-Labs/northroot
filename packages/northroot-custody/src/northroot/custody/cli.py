"""CLI for Northroot custody contracts and steward profile helpers."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from . import legacy_machine, model, registry, steward


def write_json(payload: object) -> None:
    sys.stdout.write(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    validate = sub.add_parser("validate", help="Validate a custody JSON document.")
    validate.add_argument("path")
    validate.add_argument("--public-safe", action="store_true")

    plan = sub.add_parser("render-plan", help="Render a delegated snapshot plan.")
    plan.add_argument("--inventory", required=True)
    plan.add_argument("--policy", required=True)

    retention = sub.add_parser("evaluate-retention", help="Evaluate prune safety for a snapshot.")
    retention.add_argument("--policy", required=True)
    retention.add_argument("--snapshot-id", required=True)
    retention.add_argument("--evidence", action="append", default=[])

    steward_parser = sub.add_parser("steward", help="Run steward profile commands.")
    steward_sub = steward_parser.add_subparsers(dest="steward_command", required=True)

    init = steward_sub.add_parser("init", help="Initialize steward profile files.")
    init.add_argument("--inventory", required=True)
    init.add_argument("--policy", required=True)
    init.add_argument("--output", required=True)
    init.add_argument("--profile-name", default=steward.DEFAULT_PROFILE_NAME)
    init.add_argument("--secret-bindings", help="Private secret binding document for unattended runs.")
    init.add_argument("--repository-bindings", help="Private repository binding document for runnable targets.")

    status = steward_sub.add_parser("status", help="Read steward profile status.")
    status.add_argument("--state", required=True)

    preflight = steward_sub.add_parser("preflight", help="Check delegated tool and unattended secret readiness.")
    preflight.add_argument("--state", required=True)

    verify_state = steward_sub.add_parser("verify-state", help="Verify steward state readiness without side effects.")
    verify_state.add_argument("--state", required=True)
    verify_state.add_argument("--snapshot-id", help="Snapshot id used to evaluate recorded retention evidence.")
    verify_state.add_argument("--registry-state")
    verify_state.add_argument("--project-id")
    verify_state.add_argument("--object-id")

    capabilities = steward_sub.add_parser("capabilities", help="Print agent-safe steward capability manifest.")
    capabilities.add_argument("--state", required=True)

    recover_operation = steward_sub.add_parser(
        "recover-operation",
        help="Record and clear an interrupted delegated operation lock.",
    )
    recover_operation.add_argument("--state", required=True)

    import_legacy_runs = steward_sub.add_parser(
        "import-legacy-runs",
        help="Import sanitized legacy run summaries into steward state.",
    )
    import_legacy_runs.add_argument("--state", required=True)
    import_legacy_runs.add_argument("--json", required=True)
    import_legacy_runs.add_argument("--public-safe", action="store_true")

    draft_legacy_import = steward_sub.add_parser(
        "draft-legacy-import",
        help="Draft sanitized import bundles from legacy machine-durability state.",
    )
    draft_legacy_import.add_argument("--document", choices=("profile", "runs"), required=True)
    draft_legacy_import.add_argument("--launch-agent")
    draft_legacy_import.add_argument("--machine-node")
    draft_legacy_import.add_argument("--project-nodes")
    draft_legacy_import.add_argument("--runner-state")
    draft_legacy_import.add_argument("--run-state-dir", required=True)
    draft_legacy_import.add_argument("--import-id")
    draft_legacy_import.add_argument("--legacy-import-ref")
    draft_legacy_import.add_argument(
        "--public-safe",
        action="store_true",
        help="Accepted for command-plan consistency; legacy draft output is always public-safe.",
    )

    command_plan = steward_sub.add_parser("command-plan", help="Plan a constrained agent-safe steward argv.")
    command_plan.add_argument("--state", required=True)
    command_plan.add_argument("--operation", choices=tuple(sorted(steward.COMMAND_PLAN_OPERATIONS)), required=True)
    command_plan.add_argument("--snapshot-id")
    command_plan.add_argument("--target")
    command_plan.add_argument("--execute", action="store_true")
    command_plan.add_argument("--scheduler", choices=("launchd", "systemd"))
    command_plan.add_argument("--schedule-operation", choices=tuple(sorted(steward.SCHEDULE_OPERATIONS)))
    command_plan.add_argument("--every-minutes", type=int)
    command_plan.add_argument("--runner-command")
    command_plan.add_argument("--evidence", action="append", choices=tuple(sorted(model.EXTERNAL_RETENTION_EVIDENCE)), default=[])
    command_plan.add_argument("--source")
    command_plan.add_argument("--detail")
    command_plan.add_argument("--artifact-ref")
    command_plan.add_argument("--json")
    command_plan.add_argument("--document", choices=("profile", "runs"))
    command_plan.add_argument("--launch-agent")
    command_plan.add_argument("--machine-node")
    command_plan.add_argument("--project-nodes")
    command_plan.add_argument("--runner-state")
    command_plan.add_argument("--run-state-dir")
    command_plan.add_argument("--import-id")
    command_plan.add_argument("--legacy-import-ref")
    command_plan.add_argument("--force", action="store_true")
    command_plan.add_argument("--use-recorded-evidence", action="store_true")
    command_plan.add_argument("--skip-preflight", action="store_true")
    command_plan.add_argument("--registry-state")
    command_plan.add_argument("--project-id")
    command_plan.add_argument("--object-id")

    report = steward_sub.add_parser("report", help="Render a consolidated read-only custody report.")
    report.add_argument("--state", required=True)
    report.add_argument("--snapshot-id", help="Snapshot id used to include snapshot-scoped evidence and retention state.")
    report.add_argument("--registry-state")
    report.add_argument("--project-id")
    report.add_argument("--object-id")

    evidence = steward_sub.add_parser("evidence", help="Report evidence derived from steward run summaries.")
    evidence_sub = evidence.add_subparsers(dest="evidence_command", required=True)
    evidence_report = evidence_sub.add_parser("report")
    evidence_report.add_argument("--state", required=True)
    evidence_report.add_argument("--snapshot-id")
    evidence_record = evidence_sub.add_parser("record")
    evidence_record.add_argument("--state", required=True)
    evidence_record.add_argument("--snapshot-id", required=True)
    evidence_record.add_argument("--evidence", action="append", choices=tuple(sorted(model.EXTERNAL_RETENTION_EVIDENCE)), required=True)
    evidence_record.add_argument("--source", required=True)
    evidence_record.add_argument("--detail")
    evidence_record.add_argument("--artifact-ref")

    offsite = steward_sub.add_parser("offsite", help="Report externally delegated offsite copy requirements.")
    offsite_sub = offsite.add_subparsers(dest="offsite_command", required=True)
    offsite_report = offsite_sub.add_parser("report")
    offsite_report.add_argument("--state", required=True)
    offsite_report.add_argument("--snapshot-id", required=True)

    run = steward_sub.add_parser("run", help="Render or execute the delegated backup run command.")
    run.add_argument("--state", required=True)
    run.add_argument("--snapshot-id", help="Snapshot id this run evidence should be bound to.")
    run.add_argument("--registry-state")
    run.add_argument("--project-id")
    run.add_argument("--object-id")
    run.add_argument("--execute", action="store_true", help="Execute the delegated resticprofile command.")

    verify = steward_sub.add_parser("verify", help="Render or execute the delegated verification command.")
    verify.add_argument("--state", required=True)
    verify.add_argument("--snapshot-id", help="Snapshot id this verification evidence should be bound to.")
    verify.add_argument("--registry-state")
    verify.add_argument("--project-id")
    verify.add_argument("--object-id")
    verify.add_argument("--execute", action="store_true", help="Execute the delegated resticprofile command.")

    restore = steward_sub.add_parser("restore", help="Render or execute a delegated recovery restore.")
    restore.add_argument("--state", required=True)
    restore.add_argument("--snapshot-id", required=True, help="Snapshot id to restore.")
    restore.add_argument("--target", required=True, help="Recovery restore target directory.")
    restore.add_argument("--registry-state")
    restore.add_argument("--project-id")
    restore.add_argument("--object-id")
    restore.add_argument("--execute", action="store_true", help="Execute the delegated restore command.")

    restore_drill = steward_sub.add_parser("restore-drill", help="Render or execute a delegated restore drill.")
    restore_drill.add_argument("--state", required=True)
    restore_drill.add_argument("--snapshot-id", help="Snapshot id this restore evidence should be bound to.")
    restore_drill.add_argument("--target", help="Restore drill target directory.")
    restore_drill.add_argument("--registry-state")
    restore_drill.add_argument("--project-id")
    restore_drill.add_argument("--object-id")
    restore_drill.add_argument("--execute", action="store_true", help="Execute the delegated restore command.")

    schedule = steward_sub.add_parser("schedule", help="Render scheduler templates.")
    schedule_sub = schedule.add_subparsers(dest="schedule_command", required=True)
    create = schedule_sub.add_parser("create")
    create.add_argument("--state", required=True)
    create.add_argument("--scheduler", choices=("launchd", "systemd"), required=True)
    create.add_argument("--operation", choices=tuple(sorted(steward.SCHEDULE_OPERATIONS)), default="run")
    create.add_argument("--every-minutes", type=int, required=True)
    create.add_argument("--runner-command", default=steward.DEFAULT_RUNNER_COMMAND)
    create.add_argument("--registry-state")
    create.add_argument("--project-id")
    create.add_argument("--object-id")
    status = schedule_sub.add_parser("status")
    status.add_argument("--state", required=True)
    status.add_argument("--registry-state")
    status.add_argument("--project-id")
    status.add_argument("--object-id")
    install = schedule_sub.add_parser("install")
    install.add_argument("--state", required=True)
    install.add_argument("--registry-state")
    install.add_argument("--project-id")
    install.add_argument("--object-id")
    install.add_argument("--execute", action="store_true")
    install.add_argument("--skip-preflight", action="store_true")
    uninstall = schedule_sub.add_parser("uninstall")
    uninstall.add_argument("--state", required=True)
    uninstall.add_argument("--registry-state")
    uninstall.add_argument("--project-id")
    uninstall.add_argument("--object-id")
    uninstall.add_argument("--execute", action="store_true")
    delete = schedule_sub.add_parser("delete")
    delete.add_argument("--state", required=True)
    delete.add_argument("--registry-state")
    delete.add_argument("--project-id")
    delete.add_argument("--object-id")
    delete.add_argument("--force", action="store_true")

    retention_steward = steward_sub.add_parser("retention", help="Evaluate steward retention gates.")
    retention_sub = retention_steward.add_subparsers(dest="retention_command", required=True)
    retention_evaluate = retention_sub.add_parser("evaluate")
    retention_evaluate.add_argument("--state", required=True)
    retention_evaluate.add_argument("--snapshot-id", required=True)
    retention_evaluate.add_argument("--evidence", action="append", default=[])
    retention_evaluate.add_argument("--use-recorded-evidence", action="store_true")

    registry_parser = steward_sub.add_parser("registry", help="Manage durable steward service registry state.")
    registry_sub = registry_parser.add_subparsers(dest="registry_command", required=True)
    registry_init = registry_sub.add_parser("init")
    registry_init.add_argument("--state", required=True)
    registry_init.add_argument("--registry", required=True)
    registry_init.add_argument("--public-safe", action="store_true")
    registry_init.add_argument("--overwrite", action="store_true")
    registry_status = registry_sub.add_parser("status")
    registry_status.add_argument("--state", required=True)
    registry_status.add_argument("--public-safe", action="store_true")
    registry_topology = registry_sub.add_parser("topology")
    registry_topology.add_argument("--state", required=True)
    registry_topology.add_argument("--project-id")
    registry_topology.add_argument("--public-safe", action="store_true")
    registry_verify = registry_sub.add_parser("verify")
    registry_verify.add_argument("--state", required=True)
    registry_verify.add_argument("--public-safe", action="store_true")
    registry_authorize = registry_sub.add_parser("authorize")
    registry_authorize.add_argument("--state", required=True)
    registry_authorize.add_argument("--operation", choices=tuple(sorted(model.SERVICE_PERMISSION_OPERATIONS)), required=True)
    registry_authorize.add_argument("--project-id", required=True)
    registry_authorize.add_argument("--object-id")
    registry_authorize.add_argument("--public-safe", action="store_true")
    registry_recover = registry_sub.add_parser("recover")
    registry_recover.add_argument("--state", required=True)
    registry_recover.add_argument("--public-safe", action="store_true")
    registry_recover.add_argument(
        "--adopt-landed-write",
        action="store_true",
        help="Clear a changed-after-lock registry only after the landed state has been reviewed.",
    )
    for name in (
        "add-object",
        "set-object",
        "add-permission",
        "set-permission",
        "add-project",
        "set-project",
        "add-destination",
        "set-destination",
        "bind-source",
        "set-source",
        "add-replica",
        "set-replica",
        "record-legacy-import",
        "set-legacy-import",
        "import-legacy-profile",
    ):
        mutation = registry_sub.add_parser(name)
        mutation.add_argument("--state", required=True)
        mutation.add_argument("--json", required=True, help="Path to the JSON object to apply.")
        mutation.add_argument("--public-safe", action="store_true")
    register_project = registry_sub.add_parser("register-project")
    register_project.add_argument("--state", required=True)
    register_project.add_argument("--project-json", required=True)
    register_project.add_argument("--permission-json", required=True)
    register_project.add_argument("--public-safe", action="store_true")

    return parser.parse_args(argv)


def _write_registry_result(result: object) -> int:
    write_json(result)
    if isinstance(result, dict) and result.get("locked"):
        return 1
    if isinstance(result, dict) and result.get("blocked"):
        return 1
    if isinstance(result, dict) and result.get("recovered") is False and result.get("resume_required"):
        return 1
    return 0


def _registry_mutation_result(fn, *, state: str, json_path: str, public_safe: bool) -> dict[str, object]:
    try:
        return fn(Path(state), model.load_json(Path(json_path)), public_safe=public_safe)
    except registry.RegistryLockedError as exc:
        return {
            "mutated": False,
            "locked": True,
            "resume_required": True,
            "lock": exc.lock,
            "detail": "service registry has an unresolved operation lock; run steward registry recover first",
        }
    except registry.RegistryIntegrityError as exc:
        return {
            "mutated": False,
            "blocked": True,
            "protected_state_ok": False,
            "resume_required": bool(exc.integrity.get("resume_required")),
            "operation_summary_path": str(exc.operation_summary_path) if exc.operation_summary_path else None,
            "integrity": exc.integrity,
            "detail": "service registry protected state is not ready; run steward registry verify and repair before mutating",
        }


def _legacy_draft_error(*, detail: str, missing_inputs: Sequence[str] | None = None) -> dict[str, object]:
    return {
        "ok": False,
        "operation": "draft-legacy-import",
        "missing_inputs": sorted(set(missing_inputs or [])),
        "detail": detail,
    }


def _schedule_registry_context(
    args: argparse.Namespace,
    *,
    status: dict[str, object] | None = None,
) -> tuple[Path | None, str | None, str | None]:
    registry_state = getattr(args, "registry_state", None)
    project_id = getattr(args, "project_id", None)
    object_id = getattr(args, "object_id", None)
    trusted_status = status if isinstance(status, dict) and status.get("schedule_integrity", {}).get("ok") else None
    if registry_state is None and trusted_status is not None:
        registry_state = trusted_status.get("registry_state")
    if project_id is None and trusted_status is not None:
        project_id = trusted_status.get("project_id")
    if object_id is None and trusted_status is not None:
        object_id = trusted_status.get("object_id")
    return (
        Path(str(registry_state)) if registry_state else None,
        str(project_id) if project_id else None,
        str(object_id) if object_id else None,
    )


def _authorize_schedule_context(
    args: argparse.Namespace,
    *,
    operation: str,
    status: dict[str, object] | None = None,
) -> dict[str, object] | None:
    registry_state, project_id, object_id = _schedule_registry_context(args, status=status)
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
    gate = steward.schedule_registry_gate(
        registry_state,
        operation=operation,
        project_id=project_id,
        object_id=object_id,
    )
    if gate is not None:
        return gate
    return registry.authorize_operation(
        registry_state,
        operation=operation,
        project_id=project_id,
        object_id=object_id,
        public_safe=True,
    )


def _schedule_authorization_denied_result(operation: str, authorization: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": "northroot.steward.schedule-authorization.v0",
        "operation": operation,
        "ok": False,
        "authorization": authorization,
        "error": f"registry authorization denied: {authorization.get('decision')}",
    }


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(list(argv or sys.argv[1:]))
    if args.command == "validate":
        findings = model.validate_document(model.load_json(Path(args.path)), public_safe=args.public_safe)
        write_json([finding.as_dict() for finding in findings])
        return 1 if any(finding.severity == "error" for finding in findings) else 0
    if args.command == "render-plan":
        plan = model.render_snapshot_plan(model.load_json(Path(args.inventory)), model.load_json(Path(args.policy)))
        write_json(plan)
        return 0
    if args.command == "evaluate-retention":
        decision = model.evaluate_retention(
            model.load_json(Path(args.policy)),
            snapshot_id=args.snapshot_id,
            available_evidence=list(args.evidence),
        )
        write_json(decision)
        return 0 if decision["allowed"] else 1
    if args.command == "steward" and args.steward_command == "init":
        installation = steward.init_steward(
            inventory_path=Path(args.inventory),
            policy_path=Path(args.policy),
            output_dir=Path(args.output),
            profile_name=args.profile_name,
            secret_bindings_path=Path(args.secret_bindings) if args.secret_bindings else None,
            repository_bindings_path=Path(args.repository_bindings) if args.repository_bindings else None,
        )
        write_json(installation.as_dict())
        return 0
    if args.command == "steward" and args.steward_command == "status":
        write_json(steward.render_status(Path(args.state)))
        return 0
    if args.command == "steward" and args.steward_command == "preflight":
        preflight = steward.render_preflight(Path(args.state))
        write_json(preflight)
        return 0 if preflight["ready"] else 1
    if args.command == "steward" and args.steward_command == "verify-state":
        verification = steward.render_state_verification(
            Path(args.state),
            snapshot_id=args.snapshot_id,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(verification)
        return 0 if verification["ready"] else 1
    if args.command == "steward" and args.steward_command == "capabilities":
        write_json(steward.render_capabilities(Path(args.state)))
        return 0
    if args.command == "steward" and args.steward_command == "recover-operation":
        write_json(steward.recover_operation(Path(args.state)))
        return 0
    if args.command == "steward" and args.steward_command == "import-legacy-runs":
        result = steward.import_legacy_run_summaries(
            Path(args.state),
            model.load_json(Path(args.json)),
            public_safe=args.public_safe,
        )
        write_json(result)
        return 1 if result.get("locked") else 0
    if args.command == "steward" and args.steward_command == "draft-legacy-import":
        if args.document == "profile":
            missing = [
                name
                for name, value in (
                    ("launch_agent", args.launch_agent),
                    ("machine_node", args.machine_node),
                    ("project_nodes", args.project_nodes),
                    ("runner_state", args.runner_state),
                )
                if not value
            ]
            if missing:
                write_json(
                    _legacy_draft_error(
                        detail="profile drafts require launch_agent, machine_node, project_nodes, and runner_state",
                        missing_inputs=missing,
                    )
                )
                return 1
            try:
                write_json(
                    legacy_machine.draft_legacy_profile_import(
                        launch_agent_path=Path(args.launch_agent),
                        machine_node_path=Path(args.machine_node),
                        project_nodes_path=Path(args.project_nodes),
                        runner_state_path=Path(args.runner_state),
                        run_state_dir=Path(args.run_state_dir),
                        import_id=args.import_id,
                        public_safe=True,
                    )
                )
            except (OSError, ValueError) as exc:
                write_json(_legacy_draft_error(detail=str(exc)))
                return 1
            return 0
        if not args.import_id:
            write_json(
                _legacy_draft_error(
                    detail="run drafts require import_id",
                    missing_inputs=["import_id"],
                )
            )
            return 1
        try:
            write_json(
                legacy_machine.draft_legacy_run_import(
                    run_state_dir=Path(args.run_state_dir),
                    import_id=args.import_id,
                    legacy_import_ref=args.legacy_import_ref,
                    public_safe=True,
                )
            )
        except (OSError, ValueError) as exc:
            write_json(_legacy_draft_error(detail=str(exc)))
            return 1
        return 0
    if args.command == "steward" and args.steward_command == "command-plan":
        plan = steward.render_command_plan(
            Path(args.state),
            operation=args.operation,
            snapshot_id=args.snapshot_id,
            target=args.target,
            execute=args.execute,
            scheduler=args.scheduler,
            schedule_operation=args.schedule_operation,
            every_minutes=args.every_minutes,
            runner_command=args.runner_command,
            evidence=list(args.evidence),
            source=args.source,
            detail=args.detail,
            artifact_ref=args.artifact_ref,
            json_path=Path(args.json) if args.json else None,
            document=args.document,
            launch_agent_path=Path(args.launch_agent) if args.launch_agent else None,
            machine_node_path=Path(args.machine_node) if args.machine_node else None,
            project_nodes_path=Path(args.project_nodes) if args.project_nodes else None,
            runner_state_path=Path(args.runner_state) if args.runner_state else None,
            run_state_dir=Path(args.run_state_dir) if args.run_state_dir else None,
            import_id=args.import_id,
            legacy_import_ref=args.legacy_import_ref,
            force=args.force,
            use_recorded_evidence=args.use_recorded_evidence,
            skip_preflight=args.skip_preflight,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(plan)
        return 0 if plan["ok"] else 1
    if args.command == "steward" and args.steward_command == "report":
        write_json(
            steward.render_report(
                Path(args.state),
                snapshot_id=args.snapshot_id,
                registry_state=Path(args.registry_state) if args.registry_state else None,
                project_id=args.project_id,
                object_id=args.object_id,
            )
        )
        return 0
    if args.command == "steward" and args.steward_command == "evidence" and args.evidence_command == "report":
        write_json(steward.render_evidence_report(Path(args.state), snapshot_id=args.snapshot_id))
        return 0
    if args.command == "steward" and args.steward_command == "evidence" and args.evidence_command == "record":
        write_json(
            steward.record_external_evidence(
                Path(args.state),
                snapshot_id=args.snapshot_id,
                evidence=list(args.evidence),
                source=args.source,
                detail=args.detail,
                artifact_ref=args.artifact_ref,
            )
        )
        return 0
    if args.command == "steward" and args.steward_command == "offsite" and args.offsite_command == "report":
        report = steward.render_offsite_report(Path(args.state), snapshot_id=args.snapshot_id)
        write_json(report)
        return 0 if report["complete"] else 1
    if args.command == "steward" and args.steward_command == "run":
        operation = steward.render_operation(
            Path(args.state),
            "run",
            execute=args.execute,
            snapshot_id=args.snapshot_id,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "verify":
        operation = steward.render_operation(
            Path(args.state),
            "verify",
            execute=args.execute,
            snapshot_id=args.snapshot_id,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "restore":
        operation = steward.render_operation(
            Path(args.state),
            "restore",
            execute=args.execute,
            restore_target=Path(args.target),
            snapshot_id=args.snapshot_id,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "restore-drill":
        operation = steward.render_operation(
            Path(args.state),
            "restore-drill",
            execute=args.execute,
            restore_target=Path(args.target) if args.target else None,
            snapshot_id=args.snapshot_id,
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "create":
        authorization = _authorize_schedule_context(args, operation="schedule.create")
        if authorization is not None and not authorization["allowed"]:
            write_json(_schedule_authorization_denied_result("schedule.create", authorization))
            return 1
        registry_state, project_id, object_id = _schedule_registry_context(args)
        schedule = steward.create_schedule(
            output_dir=Path(args.state),
            scheduler=args.scheduler,
            every_minutes=args.every_minutes,
            runner_command=args.runner_command,
            operation=args.operation,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if authorization is not None:
            schedule["authorization"] = authorization
        write_json(schedule)
        return 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "status":
        status = steward.schedule_status(
            Path(args.state),
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        authorization = _authorize_schedule_context(args, operation="schedule.status", status=status)
        if authorization is not None and not authorization["allowed"]:
            write_json(_schedule_authorization_denied_result("schedule.status", authorization))
            return 1
        if authorization is not None:
            status["authorization"] = authorization
        write_json(status)
        return 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "install":
        status = steward.schedule_status(
            Path(args.state),
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        authorization = _authorize_schedule_context(args, operation="schedule.install", status=status)
        if authorization is not None and not authorization["allowed"]:
            write_json(_schedule_authorization_denied_result("schedule.install", authorization))
            return 1
        registry_state, project_id, object_id = _schedule_registry_context(args, status=status)
        result = steward.install_schedule(
            Path(args.state),
            execute=args.execute,
            require_preflight=not args.skip_preflight,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if authorization is not None:
            result["authorization"] = authorization
        write_json(result)
        return 1 if result.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "uninstall":
        status = steward.schedule_status(
            Path(args.state),
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        authorization = _authorize_schedule_context(args, operation="schedule.uninstall", status=status)
        if authorization is not None and not authorization["allowed"]:
            write_json(_schedule_authorization_denied_result("schedule.uninstall", authorization))
            return 1
        registry_state, project_id, object_id = _schedule_registry_context(args, status=status)
        result = steward.uninstall_schedule(
            Path(args.state),
            execute=args.execute,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if authorization is not None:
            result["authorization"] = authorization
        write_json(result)
        return 1 if result.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "delete":
        status = steward.schedule_status(
            Path(args.state),
            registry_state=Path(args.registry_state) if args.registry_state else None,
            project_id=args.project_id,
            object_id=args.object_id,
        )
        authorization = _authorize_schedule_context(args, operation="schedule.delete", status=status)
        if authorization is not None and not authorization["allowed"]:
            write_json(_schedule_authorization_denied_result("schedule.delete", authorization))
            return 1
        registry_state, project_id, object_id = _schedule_registry_context(args, status=status)
        result = steward.delete_schedule(
            Path(args.state),
            force=args.force,
            registry_state=registry_state,
            project_id=project_id,
            object_id=object_id,
        )
        if authorization is not None:
            result["authorization"] = authorization
        write_json(result)
        return 0 if result.get("deleted") is True else 1
    if args.command == "steward" and args.steward_command == "retention" and args.retention_command == "evaluate":
        decision = steward.evaluate_retention(
            Path(args.state),
            snapshot_id=args.snapshot_id,
            available_evidence=list(args.evidence),
            use_recorded_evidence=args.use_recorded_evidence,
        )
        write_json(decision)
        return 0 if decision["allowed"] else 1
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "init":
        write_json(
            registry.initialize_registry(
                Path(args.state),
                model.load_json(Path(args.registry)),
                public_safe=args.public_safe,
                overwrite=args.overwrite,
            )
        )
        return 0
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "status":
        status = registry.registry_status(Path(args.state), public_safe=args.public_safe)
        write_json(status)
        return 0 if status["ready"] else 1
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "topology":
        topology = registry.registry_topology_report(
            Path(args.state),
            project_id=args.project_id,
            public_safe=args.public_safe,
        )
        write_json(topology)
        return 0 if topology["ready"] else 1
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "verify":
        integrity = registry.registry_integrity_report(Path(args.state), public_safe=args.public_safe)
        write_json(integrity)
        return 0 if integrity["ready"] else 1
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "authorize":
        authorization = registry.authorize_operation(
            Path(args.state),
            operation=args.operation,
            project_id=args.project_id,
            object_id=args.object_id,
            public_safe=args.public_safe,
        )
        write_json(authorization)
        return 0 if authorization["allowed"] else 1
    if args.command == "steward" and args.steward_command == "registry" and args.registry_command == "recover":
        return _write_registry_result(
            registry.recover_registry(
                Path(args.state),
                public_safe=args.public_safe,
                adopt_landed_write=args.adopt_landed_write,
            )
        )
    if args.command == "steward" and args.steward_command == "registry":
        mutation_map = {
            "add-object": registry.add_object,
            "set-object": registry.set_object,
            "add-permission": registry.add_permission,
            "set-permission": registry.set_permission,
            "add-project": registry.add_project,
            "set-project": registry.set_project,
            "add-destination": registry.add_destination,
            "set-destination": registry.set_destination,
            "bind-source": registry.bind_source_destination,
            "set-source": registry.set_source_destination,
            "add-replica": registry.add_replica,
            "set-replica": registry.set_replica,
            "record-legacy-import": registry.record_legacy_import,
            "set-legacy-import": registry.set_legacy_import,
            "import-legacy-profile": registry.import_legacy_profile,
        }
        if args.registry_command in mutation_map:
            return _write_registry_result(
                _registry_mutation_result(
                    mutation_map[args.registry_command],
                    state=args.state,
                    json_path=args.json,
                    public_safe=args.public_safe,
                )
            )
        if args.registry_command == "register-project":
            try:
                return _write_registry_result(
                    registry.register_project(
                        Path(args.state),
                        project=model.load_json(Path(args.project_json)),
                        permission=model.load_json(Path(args.permission_json)),
                        public_safe=args.public_safe,
                    )
                )
            except registry.RegistryLockedError as exc:
                return _write_registry_result(
                    {
                        "mutated": False,
                        "locked": True,
                        "resume_required": True,
                        "lock": exc.lock,
                        "detail": "service registry has an unresolved operation lock; run steward registry recover first",
                    }
                )
            except registry.RegistryIntegrityError as exc:
                return _write_registry_result(
                    {
                        "mutated": False,
                        "blocked": True,
                        "protected_state_ok": False,
                        "resume_required": bool(exc.integrity.get("resume_required")),
                        "operation_summary_path": str(exc.operation_summary_path)
                        if exc.operation_summary_path
                        else None,
                        "integrity": exc.integrity,
                        "detail": "service registry protected state is not ready; run steward registry verify and repair before mutating",
                    }
                )
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
