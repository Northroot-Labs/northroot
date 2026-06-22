"""CLI for Northroot custody contracts and steward profile helpers."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from . import model, steward


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

    capabilities = steward_sub.add_parser("capabilities", help="Print agent-safe steward capability manifest.")
    capabilities.add_argument("--state", required=True)

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
    command_plan.add_argument("--force", action="store_true")
    command_plan.add_argument("--use-recorded-evidence", action="store_true")
    command_plan.add_argument("--skip-preflight", action="store_true")

    report = steward_sub.add_parser("report", help="Render a consolidated read-only custody report.")
    report.add_argument("--state", required=True)
    report.add_argument("--snapshot-id", help="Snapshot id used to include snapshot-scoped evidence and retention state.")

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
    run.add_argument("--execute", action="store_true", help="Execute the delegated resticprofile command.")

    verify = steward_sub.add_parser("verify", help="Render or execute the delegated verification command.")
    verify.add_argument("--state", required=True)
    verify.add_argument("--snapshot-id", help="Snapshot id this verification evidence should be bound to.")
    verify.add_argument("--execute", action="store_true", help="Execute the delegated resticprofile command.")

    restore = steward_sub.add_parser("restore", help="Render or execute a delegated recovery restore.")
    restore.add_argument("--state", required=True)
    restore.add_argument("--snapshot-id", required=True, help="Snapshot id to restore.")
    restore.add_argument("--target", required=True, help="Recovery restore target directory.")
    restore.add_argument("--execute", action="store_true", help="Execute the delegated restore command.")

    restore_drill = steward_sub.add_parser("restore-drill", help="Render or execute a delegated restore drill.")
    restore_drill.add_argument("--state", required=True)
    restore_drill.add_argument("--snapshot-id", help="Snapshot id this restore evidence should be bound to.")
    restore_drill.add_argument("--target", help="Restore drill target directory.")
    restore_drill.add_argument("--execute", action="store_true", help="Execute the delegated restore command.")

    schedule = steward_sub.add_parser("schedule", help="Render scheduler templates.")
    schedule_sub = schedule.add_subparsers(dest="schedule_command", required=True)
    create = schedule_sub.add_parser("create")
    create.add_argument("--state", required=True)
    create.add_argument("--scheduler", choices=("launchd", "systemd"), required=True)
    create.add_argument("--operation", choices=tuple(sorted(steward.SCHEDULE_OPERATIONS)), default="run")
    create.add_argument("--every-minutes", type=int, required=True)
    create.add_argument("--runner-command", default=steward.DEFAULT_RUNNER_COMMAND)
    status = schedule_sub.add_parser("status")
    status.add_argument("--state", required=True)
    install = schedule_sub.add_parser("install")
    install.add_argument("--state", required=True)
    install.add_argument("--execute", action="store_true")
    install.add_argument("--skip-preflight", action="store_true")
    uninstall = schedule_sub.add_parser("uninstall")
    uninstall.add_argument("--state", required=True)
    uninstall.add_argument("--execute", action="store_true")
    delete = schedule_sub.add_parser("delete")
    delete.add_argument("--state", required=True)
    delete.add_argument("--force", action="store_true")

    retention_steward = steward_sub.add_parser("retention", help="Evaluate steward retention gates.")
    retention_sub = retention_steward.add_subparsers(dest="retention_command", required=True)
    retention_evaluate = retention_sub.add_parser("evaluate")
    retention_evaluate.add_argument("--state", required=True)
    retention_evaluate.add_argument("--snapshot-id", required=True)
    retention_evaluate.add_argument("--evidence", action="append", default=[])
    retention_evaluate.add_argument("--use-recorded-evidence", action="store_true")

    return parser.parse_args(argv)


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
        verification = steward.render_state_verification(Path(args.state), snapshot_id=args.snapshot_id)
        write_json(verification)
        return 0 if verification["ready"] else 1
    if args.command == "steward" and args.steward_command == "capabilities":
        write_json(steward.render_capabilities(Path(args.state)))
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
            force=args.force,
            use_recorded_evidence=args.use_recorded_evidence,
            skip_preflight=args.skip_preflight,
        )
        write_json(plan)
        return 0 if plan["ok"] else 1
    if args.command == "steward" and args.steward_command == "report":
        write_json(steward.render_report(Path(args.state), snapshot_id=args.snapshot_id))
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
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "verify":
        operation = steward.render_operation(
            Path(args.state),
            "verify",
            execute=args.execute,
            snapshot_id=args.snapshot_id,
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
        )
        write_json(operation)
        return 1 if operation.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "create":
        schedule = steward.create_schedule(
            output_dir=Path(args.state),
            scheduler=args.scheduler,
            every_minutes=args.every_minutes,
            runner_command=args.runner_command,
            operation=args.operation,
        )
        write_json(schedule)
        return 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "status":
        write_json(steward.schedule_status(Path(args.state)))
        return 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "install":
        result = steward.install_schedule(
            Path(args.state),
            execute=args.execute,
            require_preflight=not args.skip_preflight,
        )
        write_json(result)
        return 1 if result.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "uninstall":
        result = steward.uninstall_schedule(Path(args.state), execute=args.execute)
        write_json(result)
        return 1 if result.get("return_code") not in (None, 0) else 0
    if args.command == "steward" and args.steward_command == "schedule" and args.schedule_command == "delete":
        result = steward.delete_schedule(Path(args.state), force=args.force)
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
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
