import json
import hashlib
import os
import shlex
import tempfile
import unittest
import xml.etree.ElementTree as ET
from unittest import mock
from pathlib import Path

from northroot.custody import model, registry, steward


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def write_fake_executable(directory: Path, name: str, body: str = "#!/bin/sh\nexit 0\n") -> None:
    path = directory / name
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class StewardTests(unittest.TestCase):
    def test_import_legacy_run_summaries_feeds_evidence_report_safely(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir) / "steward"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=output_dir,
            )
            legacy_import = model.load_json(EXAMPLES / "legacy-run-import.redacted.example.json")

            imported = steward.import_legacy_run_summaries(output_dir, legacy_import, public_safe=True)

            self.assertTrue(imported["imported"])
            self.assertEqual(imported["inserted_run_ids"], ["legacy-run-001", "legacy-offsite-evidence-001"])
            self.assertEqual(imported["skipped_existing_run_ids"], [])
            self.assertTrue((output_dir / "run-summaries" / "legacy-run-001.json").exists())
            self.assertFalse(steward.operation_lock_path(output_dir).exists())
            evidence = steward.render_evidence_report(output_dir, snapshot_id="legacy-snap-001")
            self.assertTrue(evidence["run_summary_integrity"]["ok"])
            summary_index = model.load_json(output_dir / "run-summaries" / "index.json")
            self.assertEqual(summary_index["schema_version"], steward.RUN_SUMMARY_INDEX_SCHEMA)
            self.assertEqual(
                evidence["available_evidence"],
                ["restore_drill", "verified_offsite_copy", "verified_snapshot"],
            )
            retention = steward.evaluate_retention(
                output_dir,
                snapshot_id="legacy-snap-001",
                available_evidence=[],
                use_recorded_evidence=True,
            )
            self.assertTrue(retention["allowed"])

            replayed = steward.import_legacy_run_summaries(output_dir, legacy_import, public_safe=True)
            self.assertEqual(replayed["inserted_run_ids"], [])
            self.assertEqual(replayed["skipped_existing_run_ids"], ["legacy-run-001", "legacy-offsite-evidence-001"])
            self.assertIsNone(replayed["operation_summary_path"])

            conflicting_import = model.load_json(EXAMPLES / "legacy-run-import.redacted.example.json")
            conflicting_import["run_summaries"][0]["status"] = "delegated-failed"
            with self.assertRaises(ValueError):
                steward.import_legacy_run_summaries(output_dir, conflicting_import, public_safe=True)
            self.assertFalse(steward.operation_lock_path(output_dir).exists())
            retained = model.load_json(output_dir / "run-summaries" / "legacy-run-001.json")
            self.assertEqual(retained["status"], "delegated-ok")

            steward.operation_lock_path(output_dir).write_text(
                json.dumps(
                    {
                        "schema_version": "northroot.steward.operation-lock.v0",
                        "operation_id": "legacy-import-lock",
                        "operation": "legacy-run-import",
                        "state": str(output_dir),
                        "command": "steward import-legacy-runs",
                        "import_id": "legacy/example-machine-durability-runs",
                        "pid": 999999,
                        "started_at": "20260622T000000000000Z",
                        "failure_policy": "fail-closed-record-summary-before-retry",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            locked = steward.import_legacy_run_summaries(output_dir, legacy_import, public_safe=True)
            self.assertFalse(locked["imported"])
            self.assertTrue(locked["locked"])
            recovered = steward.recover_operation(output_dir)
            self.assertTrue(recovered["recovered"])
            recovered_summary = model.load_json(Path(str(recovered["run_summary_path"])))
            self.assertEqual(recovered_summary["snapshot_result"]["operation"], "legacy-run-import")

    def test_unreadable_operation_lock_fails_closed_and_recovers_with_summary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir) / "steward"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=output_dir,
            )
            legacy_import = model.load_json(EXAMPLES / "legacy-run-import.redacted.example.json")
            lock_path = steward.operation_lock_path(output_dir)
            lock_path.write_text("{not-json\n", encoding="utf-8")

            locked_import = steward.import_legacy_run_summaries(output_dir, legacy_import, public_safe=True)
            blocked_operation = steward.render_operation(output_dir, "verify", execute=True)

            self.assertFalse(locked_import["imported"])
            self.assertTrue(locked_import["locked"])
            self.assertIsNone(locked_import["lock"])
            self.assertIsInstance(locked_import["lock_error"], str)
            self.assertEqual(blocked_operation["return_code"], 75)
            self.assertEqual(blocked_operation["failure_stage"], "operation-lock")
            self.assertTrue(blocked_operation["operation_lock"]["unreadable"])
            blocked_summary = model.load_json(Path(str(blocked_operation["run_summary_path"])))
            self.assertEqual(blocked_summary["status"], "delegated-operation-locked")
            self.assertTrue(blocked_summary["snapshot_result"]["operation_lock"]["unreadable"])
            self.assertTrue(lock_path.exists())

            recovered = steward.recover_operation(output_dir)

            self.assertTrue(recovered["recovered"])
            self.assertTrue(recovered["cleared_lock"])
            self.assertFalse(lock_path.exists())
            recovery_summary = model.load_json(Path(str(recovered["run_summary_path"])))
            self.assertEqual(recovery_summary["status"], "delegated-invalid-lock-recovered")
            self.assertEqual(recovery_summary["snapshot_result"]["failure_stage"], "invalid-operation-lock")
            self.assertTrue(recovery_summary["snapshot_result"]["operation_lock"]["unreadable"])
            self.assertTrue(steward.render_run_summary_integrity(output_dir)["ok"])
            self.assertFalse(steward.recover_operation(output_dir)["recovered"])

    def test_state_verification_and_report_include_registry_policy_context(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir) / "steward"
            registry_dir = Path(temp_dir) / "registry"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
            )
            registry.initialize_registry(
                registry_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            fake_bin = Path(temp_dir) / "bin"
            fake_bin.mkdir()
            write_fake_executable(fake_bin, "resticprofile")
            write_fake_executable(fake_bin, "op")

            with mock.patch.dict(
                os.environ,
                {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"},
                clear=True,
            ):
                verified = steward.render_state_verification(
                    output_dir,
                    registry_state=registry_dir,
                    project_id="project/example",
                    object_id="secrets/restic-env",
                )
                report = steward.render_report(
                    output_dir,
                    registry_state=registry_dir,
                    project_id="project/example",
                )

            self.assertTrue(verified["ready"])
            self.assertTrue(verified["safe_to_execute"])
            self.assertTrue(verified["registry_context"]["ready"])
            self.assertEqual(verified["registry_context"]["authorization"]["decision"], "allowed")
            self.assertTrue(report["registry_ready"])
            self.assertEqual(report["registry_context"]["authorization"]["decision"], "allowed")

            registry_path = registry.registry_path(registry_dir)
            tampered_registry = model.load_json(registry_path)
            tampered_registry["service_id"] = "steward/tampered"
            registry_path.write_text(json.dumps(tampered_registry, indent=2, sort_keys=True) + "\n", encoding="utf-8")

            with mock.patch.dict(
                os.environ,
                {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"},
                clear=True,
            ):
                tampered_verified = steward.render_state_verification(
                    output_dir,
                    registry_state=registry_dir,
                    project_id="project/example",
                )
                tampered_report = steward.render_report(
                    output_dir,
                    registry_state=registry_dir,
                    project_id="project/example",
                )

            self.assertFalse(tampered_verified["ready"])
            self.assertFalse(tampered_verified["safe_to_execute"])
            self.assertFalse(tampered_verified["registry_context"]["registry_ready"])
            self.assertIn(
                "registry_not_ready",
                {check["code"] for check in tampered_verified["checks"] if check["code"]},
            )
            self.assertIn(
                "repair service registry integrity before relying on steward policy authorization",
                tampered_report["recommended_actions"],
            )

    def test_init_status_run_and_schedule_are_delegated(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir) / "steward"

            installation = steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
            )

            self.assertEqual(installation.execution_mode, "delegated")
            self.assertEqual(installation.delegated_tool, "resticprofile")
            self.assertFalse(installation.custom_backup_engine)
            self.assertEqual(
                sorted(installation.generated_artifacts),
                ["resticprofile_config", "snapshot_plan"],
            )
            self.assertTrue((output_dir / "snapshot-plan.json").exists())
            self.assertTrue((output_dir / "resticprofile.yaml").exists())
            resticprofile_config = (output_dir / "resticprofile.yaml").read_text(encoding="utf-8")
            self.assertIn(
                'password-command: "op read secret-provider://restic/local-password"',
                resticprofile_config,
            )
            self.assertIn("Primary destination: local-restic-profile (repository://local-primary)", resticprofile_config)
            self.assertIn("Additional destination: offsite-restic-copy (repository://offsite-vault)", resticprofile_config)
            self.assertIn('repository: "repository-target://local-primary"', resticprofile_config)
            self.assertNotIn('repository: "repository://local-primary"', resticprofile_config)
            self.assertNotIn('repository: "repository-target://offsite-vault"', resticprofile_config)

            status = steward.render_status(output_dir)
            self.assertEqual(status["source_count"], 2)
            self.assertEqual(status["destination_count"], 2)
            self.assertEqual(status["primary_destination_id"], "local-restic-profile")
            self.assertEqual(status["primary_repository_ref"], "repository://local-primary")
            self.assertEqual(status["external_destination_count"], 1)
            self.assertEqual(status["external_destination_ids"], ["offsite-restic-copy"])
            self.assertEqual(status["external_destination_evidence_required"], ["verified_offsite_copy"])
            self.assertIn("resticprofile_config", status["generated_artifacts"])
            self.assertIn("snapshot_plan", status["generated_artifacts"])
            self.assertTrue(status["installation_integrity"]["ok"])
            self.assertTrue((output_dir / "steward-installation-index.json").exists())
            self.assertIn("nr steward status --state", status["commands"]["status"])
            self.assertIn("nr steward preflight --state", status["commands"]["preflight"])
            self.assertIn("nr steward capabilities --state", status["commands"]["capabilities"])

            capabilities = steward.render_capabilities(output_dir)
            self.assertEqual(capabilities["schema_version"], "northroot.steward.capabilities.v0")
            self.assertTrue(capabilities["execution_model"]["preflight_required_before_execute"])
            self.assertTrue(capabilities["execution_model"]["installation_manifest_integrity_checked_by_preflight"])
            self.assertTrue(capabilities["execution_model"]["generated_artifact_integrity_checked_by_preflight"])
            self.assertIn("macos-keychain", capabilities["secret_binding_providers"])
            self.assertIn("onepassword-cli", capabilities["secret_binding_providers"])
            self.assertIn("macos-keychain", capabilities["runtime_environment_providers"])
            self.assertTrue(capabilities["restore_verification"]["records_manifest_sha256"])
            self.assertTrue(capabilities["evidence_scope"]["retention_recorded_evidence_is_snapshot_bound"])
            self.assertTrue(capabilities["evidence_scope"]["snapshot_filtered_reports_ignore_unscoped_operation_evidence"])
            self.assertEqual(
                capabilities["contracts"]["verification_result_schema"],
                model.VERIFICATION_RESULT_SCHEMA,
            )
            self.assertEqual(
                capabilities["agent_contract"]["schema_version"],
                "northroot.steward.agent-contract.v0",
            )
            self.assertFalse(capabilities["agent_contract"]["invocation"]["shell_required"])
            self.assertTrue(capabilities["agent_contract"]["invocation"]["template_placeholders_must_be_bound"])
            self.assertEqual(
                capabilities["agent_contract"]["invocation"]["command_plan_schema"],
                "northroot.steward.command-plan.v0",
            )
            self.assertFalse(capabilities["agent_contract"]["secret_handling"]["secret_values_returned"])
            self.assertEqual(
                model.validate_agent_delegation_policy(
                    capabilities["agent_contract"]["default_dogfood_policy"],
                    public_safe=True,
                ),
                [],
            )
            self.assertEqual(
                capabilities["agent_contract"]["default_dogfood_policy"]["registered_agents"][0]["agent_id"],
                "agent:codex",
            )
            operations = {operation["name"]: operation for operation in capabilities["allowed_operations"]}
            operation_contracts = {
                operation["name"]: operation for operation in capabilities["operation_contracts"]
            }
            self.assertLessEqual(steward.COMMAND_PLAN_OPERATIONS, set(operations))
            self.assertLessEqual(steward.COMMAND_PLAN_OPERATIONS, set(operation_contracts))
            self.assertIn("report", operations)
            self.assertEqual(
                operation_contracts["command-plan"]["success_schema"],
                "northroot.steward.command-plan.v0",
            )
            self.assertEqual(
                operation_contracts["command-plan"]["allowed_operations"],
                sorted(steward.COMMAND_PLAN_OPERATIONS),
            )
            self.assertEqual(operation_contracts["schedule.status"]["success_schema"], "northroot.steward.schedule-status.v0")
            self.assertEqual(operation_contracts["report"]["argv_template"][0:3], ["nr", "steward", "report"])
            self.assertEqual(operation_contracts["report"]["success_schema"], "northroot.steward.report.v0")
            self.assertEqual(operation_contracts["run"]["argv_template"][0:3], ["nr", "steward", "run"])
            self.assertIn("snapshot_id", operation_contracts["run"]["recommended_inputs"])
            self.assertEqual(operation_contracts["restore"]["required_inputs"], ["snapshot_id", "target"])
            self.assertEqual(operation_contracts["restore"]["satisfies_retention_evidence"], [])
            self.assertIn("restore_drill", operation_contracts["restore-drill"]["satisfies_retention_evidence"])
            self.assertEqual(operation_contracts["retention.evaluate"]["success_schema"], model.RETENTION_DECISION_SCHEMA)
            self.assertEqual(operation_contracts["evidence.record"]["allowed_evidence"], ["verified_offsite_copy"])
            self.assertEqual(
                operation_contracts["import-legacy-runs"]["success_schema"],
                "northroot.steward.legacy-run-import-result.v0",
            )
            self.assertTrue(operations["branch.create"]["governed_by_default_dogfood_policy"])
            self.assertTrue(operations["import-legacy-runs"]["uses_operation_lock"])
            self.assertIn(
                "delegates_platform_scheduler_registration_when_execute_is_set",
                operation_contracts["schedule.install"]["side_effects"],
            )
            self.assertTrue(operations["run"]["requires_preflight"])
            self.assertIn("--execute", operations["run"]["execute_command"])
            self.assertIn("--snapshot-id", operations["verify"]["execute_command"])
            self.assertTrue(operations["verify"]["snapshot_id_required_for_retention_evidence"])
            self.assertTrue(operations["restore"]["snapshot_id_required"])
            self.assertTrue(operations["restore"]["target_required"])
            self.assertEqual(operations["restore"]["satisfies_retention_evidence"], [])
            self.assertTrue(operations["restore-drill"]["snapshot_id_required_for_retention_evidence"])
            self.assertTrue(capabilities["restore_verification"]["restore_operation_requires_explicit_snapshot_id"])
            self.assertTrue(capabilities["restore_verification"]["restore_operation_requires_explicit_target"])
            self.assertTrue(
                capabilities["restore_verification"]["actual_restore_does_not_satisfy_restore_drill_retention_evidence"]
            )
            self.assertIn("retention.evaluate", operations)
            self.assertTrue(operations["schedule.delete"]["blocked_when_installed"])
            self.assertIn("--force", operations["schedule.delete"]["force_command"])
            self.assertIn("direct restic/resticprofile", capabilities["prohibited_operations"][0])
            self.assertTrue(capabilities["schedule_lifecycle"]["scheduled_runner_command_checked_by_preflight"])
            self.assertTrue(capabilities["schedule_lifecycle"]["schedule_manifest_integrity_checked_by_preflight"])
            self.assertTrue(capabilities["schedule_lifecycle"]["generated_schedule_integrity_checked_by_preflight"])
            self.assertTrue(capabilities["schedule_lifecycle"]["uninstall_required_before_delete_when_installed"])
            self.assertTrue(capabilities["schedule_lifecycle"]["delete_force_is_operator_cleanup_only"])
            self.assertEqual(capabilities["offsite_execution_model"]["kind"], "external-delegated")
            self.assertIn(
                "assuming additional destinations were copied because the primary backup ran",
                capabilities["prohibited_operations"],
            )
            self.assertEqual(
                capabilities["destination_execution"]["primary_destination"]["repository_ref"],
                "repository://local-primary",
            )
            self.assertEqual(
                capabilities["destination_execution"]["additional_destinations"][0]["repository_ref"],
                "repository://offsite-vault",
            )
            self.assertEqual(
                capabilities["destination_execution"]["required_external_evidence"],
                ["verified_offsite_copy"],
            )

            report_plan = steward.render_command_plan(
                output_dir,
                operation="report",
                snapshot_id="snap-001",
            )
            self.assertTrue(report_plan["ok"])
            self.assertFalse(report_plan["shell_required"])
            self.assertEqual(
                report_plan["argv"],
                ["nr", "steward", "report", "--state", str(output_dir), "--snapshot-id", "snap-001"],
            )
            self.assertEqual(model.validate_command_plan(report_plan), [])

            legacy_import_plan = steward.render_command_plan(
                output_dir,
                operation="import-legacy-runs",
                json_path=EXAMPLES / "legacy-run-import.redacted.example.json",
            )
            self.assertTrue(legacy_import_plan["ok"])
            self.assertIn("--public-safe", legacy_import_plan["argv"])
            self.assertTrue(legacy_import_plan["side_effects"]["writes_run_summary"])
            self.assertEqual(model.validate_command_plan(legacy_import_plan), [])

            missing_legacy_import_plan = steward.render_command_plan(output_dir, operation="import-legacy-runs")
            self.assertFalse(missing_legacy_import_plan["ok"])
            self.assertIn("json", missing_legacy_import_plan["missing_inputs"])

            agent_branch_plan = steward.render_command_plan(
                output_dir,
                operation="branch.create",
                branch="codex/steward-dogfood-policy",
            )
            self.assertTrue(agent_branch_plan["ok"])
            self.assertEqual(
                agent_branch_plan["argv"],
                ["git", "switch", "-c", "codex/steward-dogfood-policy", "main"],
            )
            self.assertEqual(agent_branch_plan["agent_authorization"]["agent_id"], "agent:codex")
            self.assertEqual(agent_branch_plan["agent_provenance"]["policy_id"], "agent-delegation/dogfood-default")
            self.assertEqual(model.validate_command_plan(agent_branch_plan), [])

            agent_commit_plan = steward.render_command_plan(
                output_dir,
                operation="commit.create",
                branch="codex/steward-dogfood-policy",
                commit_message="Checkpoint steward dogfood policy",
                detail="focused tests passed",
            )
            self.assertTrue(agent_commit_plan["ok"])
            self.assertIn("--trailer", agent_commit_plan["argv"])
            self.assertIn("Agent-Id=agent:codex", agent_commit_plan["argv"])
            self.assertEqual(
                agent_commit_plan["agent_provenance"]["required_commit_trailers"]["Agent-Coauthorship"],
                "agent-authored",
            )

            protected_agent_plan = steward.render_command_plan(
                output_dir,
                operation="push.branch",
                branch="main",
            )
            self.assertFalse(protected_agent_plan["ok"])
            self.assertIn(
                "branch is outside delegated agent prefixes or protected: main",
                protected_agent_plan["refused_reasons"],
            )

            refused_restore_plan = steward.render_command_plan(
                output_dir,
                operation="restore",
                snapshot_id="snap-001",
                target=str(Path(temp_dir) / "recovery"),
                execute=True,
            )
            self.assertFalse(refused_restore_plan["ok"])
            self.assertEqual(refused_restore_plan["preflight_ready"], False)
            self.assertIn("preflight is not ready for execute", refused_restore_plan["refused_reasons"])
            self.assertIn("--execute", refused_restore_plan["argv"])
            self.assertTrue(refused_restore_plan["agent_guidance"]["do_not_shell_join_argv"])
            self.assertEqual(model.validate_command_plan(refused_restore_plan), [])

            registry_dir = Path(temp_dir) / "registry"
            registry.initialize_registry(
                registry_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            authorized_registry_plan = steward.render_command_plan(
                output_dir,
                operation="run",
                registry_state=registry_dir,
                project_id="project/example",
            )
            self.assertTrue(authorized_registry_plan["ok"])
            self.assertTrue(authorized_registry_plan["authorization"]["allowed"])
            self.assertIn("--registry-state", authorized_registry_plan["argv"])
            authorized_schedule_plan = steward.render_command_plan(
                output_dir,
                operation="schedule.create",
                scheduler="launchd",
                schedule_operation="run",
                every_minutes=60,
                registry_state=registry_dir,
                project_id="project/example",
            )
            self.assertTrue(authorized_schedule_plan["ok"])
            self.assertTrue(authorized_schedule_plan["authorization"]["allowed"])
            self.assertIn("--registry-state", authorized_schedule_plan["argv"])
            denied_registry_plan = steward.render_command_plan(
                output_dir,
                operation="run",
                registry_state=registry_dir,
                project_id="project/example",
                object_id="secrets/restic-env",
            )
            self.assertFalse(denied_registry_plan["ok"])
            self.assertEqual(denied_registry_plan["authorization"]["decision"], "blocked")
            self.assertIn("registry authorization denied: blocked", denied_registry_plan["refused_reasons"])
            missing_project_plan = steward.render_command_plan(
                output_dir,
                operation="run",
                registry_state=registry_dir,
            )
            self.assertFalse(missing_project_plan["ok"])
            self.assertIn("project_id", missing_project_plan["missing_inputs"])
            denied_registry_execute = steward.render_operation(
                output_dir,
                "run",
                execute=True,
                registry_state=registry_dir,
                project_id="project/example",
                object_id="secrets/restic-env",
            )
            self.assertFalse(denied_registry_execute["executed"])
            self.assertEqual(denied_registry_execute["return_code"], 77)
            self.assertEqual(denied_registry_execute["failure_stage"], "authorization")
            self.assertIsNone(denied_registry_execute["preflight"])
            denied_summary = model.load_json(Path(str(denied_registry_execute["run_summary_path"])))
            self.assertEqual(denied_summary["status"], "delegated-authorization-denied")
            self.assertEqual(denied_summary["snapshot_result"]["authorization"]["decision"], "blocked")
            self.assertFalse(steward.operation_lock_path(output_dir).exists())

            stale_lock = {
                "schema_version": "northroot.steward.operation-lock.v0",
                "operation_id": "test-stale-operation",
                "operation": "verify",
                "state": str(output_dir),
                "command": "resticprofile --name steward check",
                "command_args": ["resticprofile", "--name", "steward", "check"],
                "snapshot_id": "snap-locked",
                "restore_target": None,
                "registry_state": str(registry_dir),
                "project_id": "project/example",
                "object_id": "workspace/example",
                "pid": 999999,
                "started_at": "20260622T000000000000Z",
                "failure_policy": "fail-closed-record-summary-before-retry",
                "resume_hint": "run steward recover-operation before retrying delegated execution",
            }
            steward.operation_lock_path(output_dir).write_text(
                json.dumps(stale_lock, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            blocked_by_lock = steward.render_operation(output_dir, "verify", execute=True)
            self.assertFalse(blocked_by_lock["executed"])
            self.assertEqual(blocked_by_lock["return_code"], 75)
            self.assertEqual(blocked_by_lock["failure_stage"], "operation-lock")
            self.assertIsNone(blocked_by_lock["preflight"])
            self.assertTrue(steward.operation_lock_path(output_dir).exists())
            locked_state = steward.render_state_verification(output_dir)
            self.assertFalse(locked_state["ready"])
            self.assertFalse(locked_state["safe_to_execute"])
            self.assertTrue(locked_state["operation_resume_required"])
            self.assertIn(
                "steward_operation_lock_present",
                {check["code"] for check in locked_state["checks"] if check["code"]},
            )
            locked_report = steward.render_report(output_dir)
            self.assertIn(
                "run steward recover-operation before retrying steward execution",
                locked_report["recommended_actions"],
            )

            locked_execute_plan = steward.render_command_plan(output_dir, operation="verify", execute=True)
            self.assertFalse(locked_execute_plan["ok"])
            self.assertIn(
                "steward operation lock requires recover-operation before execute",
                locked_execute_plan["refused_reasons"],
            )
            locked_import_plan = steward.render_command_plan(
                output_dir,
                operation="import-legacy-runs",
                json_path=EXAMPLES / "legacy-run-import.redacted.example.json",
            )
            self.assertFalse(locked_import_plan["ok"])
            self.assertIn(
                "steward operation lock requires recover-operation before legacy run import",
                locked_import_plan["refused_reasons"],
            )
            locked_summary = model.load_json(Path(str(blocked_by_lock["run_summary_path"])))
            self.assertEqual(locked_summary["status"], "delegated-operation-locked")
            self.assertEqual(
                locked_summary["snapshot_result"]["operation_lock"]["operation_id"],
                "test-stale-operation",
            )
            recovered_lock = steward.recover_operation(output_dir)
            self.assertTrue(recovered_lock["recovered"])
            self.assertTrue(recovered_lock["cleared_lock"])
            self.assertFalse(steward.operation_lock_path(output_dir).exists())
            recovery_lock_summary = model.load_json(Path(str(recovered_lock["run_summary_path"])))
            self.assertEqual(recovery_lock_summary["status"], "delegated-interrupted-recovered")
            self.assertEqual(
                recovery_lock_summary["snapshot_result"]["operation_lock"]["operation_id"],
                "test-stale-operation",
            )
            self.assertFalse(steward.recover_operation(output_dir)["recovered"])

            missing_schedule_plan = steward.render_command_plan(output_dir, operation="schedule.create")
            self.assertFalse(missing_schedule_plan["ok"])
            self.assertEqual(
                missing_schedule_plan["missing_inputs"],
                ["every_minutes", "schedule_operation", "scheduler"],
            )
            self.assertEqual(model.validate_command_plan(missing_schedule_plan), [])

            blocked_retention = steward.evaluate_retention(
                output_dir,
                snapshot_id="snap-001",
                available_evidence=["verified_snapshot"],
            )
            allowed_retention = steward.evaluate_retention(
                output_dir,
                snapshot_id="snap-001",
                available_evidence=["verified_snapshot", "verified_offsite_copy", "restore_drill"],
            )
            self.assertFalse(blocked_retention["allowed"])
            self.assertEqual(blocked_retention["missing_evidence"], ["verified_offsite_copy", "restore_drill"])
            self.assertTrue(allowed_retention["allowed"])
            self.assertEqual(model.validate_retention_decision(allowed_retention), [])

            fake_bin = Path(temp_dir) / "bin"
            fake_bin.mkdir()
            write_fake_executable(
                fake_bin,
                "resticprofile",
                "#!/bin/sh\n"
                "while [ \"$#\" -gt 0 ]; do\n"
                "  if [ \"$1\" = \"--target\" ]; then\n"
                "    shift\n"
                "    /bin/mkdir -p \"$1\"\n"
                "    printf '%s\\n' restored > \"$1/restored.txt\"\n"
                "  fi\n"
                "  shift\n"
                "done\n"
                "exit 0\n",
            )
            write_fake_executable(fake_bin, "op")
            write_fake_executable(
                fake_bin,
                "storage-probe",
                "#!/bin/sh\n"
                "if [ \"$1\" = \"ok\" ]; then\n"
                "  exit 0\n"
                "fi\n"
                "exit 2\n",
            )
            available_repository_bindings_path = Path(temp_dir) / "repository-bindings-available.json"
            unavailable_repository_bindings_path = Path(temp_dir) / "repository-bindings-unavailable.json"
            repository_bindings = model.load_json(EXAMPLES / "repository-bindings.redacted.example.json")
            for binding in repository_bindings["bindings"]:
                binding["availability_check"] = {
                    "mode": "probe-command",
                    "command": ["storage-probe", "ok"],
                    "interactive": False,
                    "timeout_seconds": 5,
                }
            available_repository_bindings_path.write_text(
                json.dumps(repository_bindings, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            unavailable_repository_bindings = model.load_json(EXAMPLES / "repository-bindings.redacted.example.json")
            unavailable_repository_bindings["bindings"][0]["availability_check"] = {
                "mode": "probe-command",
                "command": ["storage-probe", "missing"],
                "interactive": False,
                "timeout_seconds": 5,
            }
            unavailable_repository_bindings_path.write_text(
                json.dumps(unavailable_repository_bindings, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            available_storage_output_dir = Path(temp_dir) / "steward-available-storage"
            unavailable_storage_output_dir = Path(temp_dir) / "steward-unavailable-storage"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=available_storage_output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=available_repository_bindings_path,
            )
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=unavailable_storage_output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=unavailable_repository_bindings_path,
            )
            with mock.patch.dict(
                os.environ,
                {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"},
                clear=True,
            ):
                preflight = steward.render_preflight(output_dir)
                available_storage_preflight = steward.render_preflight(available_storage_output_dir)
                unavailable_storage_preflight = steward.render_preflight(unavailable_storage_output_dir)
                executed_unscoped_verify = steward.render_operation(output_dir, "verify", execute=True)
                executed_verify = steward.render_operation(
                    output_dir,
                    "verify",
                    execute=True,
                    snapshot_id="snap-001",
                )
                executed_restore = steward.render_operation(
                    output_dir,
                    "restore-drill",
                    execute=True,
                    snapshot_id="snap-001",
                )
                executed_recovery_restore = steward.render_operation(
                    output_dir,
                    "restore",
                    execute=True,
                    restore_target=Path(temp_dir) / "recovery-restore",
                    snapshot_id="snap-recovery",
                )
                tampered_output_dir = Path(temp_dir) / "steward-tampered"
                steward.init_steward(
                    inventory_path=EXAMPLES / "workspace-inventory.example.json",
                    policy_path=EXAMPLES / "custody-policy.example.json",
                    output_dir=tampered_output_dir,
                    secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                    repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
                )
                tampered_config = tampered_output_dir / "resticprofile.yaml"
                tampered_config.write_text(
                    tampered_config.read_text(encoding="utf-8") + "# operator hand edit\n",
                    encoding="utf-8",
                )
                tampered_preflight = steward.render_preflight(tampered_output_dir)
                tampered_verification = steward.render_state_verification(tampered_output_dir)
            self.assertTrue(preflight["ready"])
            self.assertTrue(available_storage_preflight["ready"])
            self.assertFalse(unavailable_storage_preflight["ready"])
            self.assertIn("repository_availability:repository://local-primary", {
                check["name"] for check in available_storage_preflight["checks"]
            })
            self.assertIn("repository_storage_unavailable", {
                check["code"] for check in unavailable_storage_preflight["checks"] if check["code"]
            })
            self.assertEqual(preflight["schema_version"], "northroot.steward.preflight.v0")
            self.assertFalse(tampered_preflight["ready"])
            self.assertIn("generated_artifact_drift", {
                check["code"] for check in tampered_preflight["checks"] if check["code"]
            })
            self.assertFalse(tampered_verification["ready"])
            self.assertFalse(tampered_verification["safe_to_execute"])
            self.assertIn("generated_artifact_integrity_failed", {
                check["code"] for check in tampered_verification["checks"] if check["code"]
            })
            tampered_installation_dir = Path(temp_dir) / "steward-tampered-installation"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=tampered_installation_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
            )
            installation_manifest = tampered_installation_dir / "steward-installation.json"
            tampered_installation = model.load_json(installation_manifest)
            tampered_installation["profile_name"] = "operator-hand-edit"
            installation_manifest.write_text(
                json.dumps(tampered_installation, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            tampered_installation_preflight = steward.render_preflight(tampered_installation_dir)
            tampered_installation_verify = steward.render_state_verification(tampered_installation_dir)
            self.assertFalse(tampered_installation_preflight["ready"])
            self.assertEqual(tampered_installation_preflight["installation_integrity"]["status"], "digest-mismatch")
            self.assertIn("installation_manifest_integrity_failed", {
                check["code"] for check in tampered_installation_preflight["checks"] if check["code"]
            })
            self.assertFalse(tampered_installation_verify["ready"])
            self.assertFalse(tampered_installation_verify["safe_to_execute"])
            self.assertEqual(
                preflight["destination_execution"]["additional_destination_handling"],
                "external-evidence-required",
            )
            self.assertIn("secret_env:secret://restic/local-password:OP_SERVICE_ACCOUNT_TOKEN", {
                check["name"] for check in preflight["checks"]
            })
            self.assertIn("repository_binding:repository://local-primary", {
                check["name"] for check in preflight["checks"]
            })
            self.assertTrue(executed_unscoped_verify["executed"])
            self.assertTrue(executed_verify["executed"])
            self.assertTrue(executed_restore["executed"])
            self.assertTrue(executed_recovery_restore["executed"])
            self.assertFalse(steward.operation_lock_path(output_dir).exists())
            restore_summary = model.load_json(Path(str(executed_restore["run_summary_path"])))
            self.assertEqual(restore_summary["snapshot_result"]["snapshot_id"], "snap-001")
            restore_observation = restore_summary["verification_result"]["restore_observation"]
            self.assertEqual(
                restore_summary["verification_result"]["schema_version"],
                model.VERIFICATION_RESULT_SCHEMA,
            )
            self.assertTrue(restore_summary["verification_result"]["restore_verified"])
            self.assertTrue(restore_observation["verified"])
            self.assertEqual(restore_observation["file_count"], 1)
            self.assertEqual(restore_observation["byte_count"], len("restored\n"))
            self.assertIsNotNone(restore_observation["manifest_sha256"])
            recovery_summary = model.load_json(Path(str(executed_recovery_restore["run_summary_path"])))
            self.assertEqual(recovery_summary["snapshot_result"]["operation"], "restore")
            self.assertEqual(recovery_summary["snapshot_result"]["snapshot_id"], "snap-recovery")
            self.assertTrue(recovery_summary["verification_result"]["restore_verified"])
            self.assertEqual(model.validate_run_summary(recovery_summary), [])
            recovery_evidence_report = steward.render_evidence_report(output_dir, snapshot_id="snap-recovery")
            self.assertNotIn("restore_drill", recovery_evidence_report["available_evidence"])
            with self.assertRaises(ValueError):
                steward.render_operation(output_dir, "restore", restore_target=Path(temp_dir) / "missing-snapshot")
            with self.assertRaises(ValueError):
                steward.render_operation(output_dir, "restore", snapshot_id="snap-missing-target")
            evidence_report = steward.render_evidence_report(output_dir)
            self.assertIn("verified_snapshot", evidence_report["available_evidence"])
            self.assertIn("restore_drill", evidence_report["available_evidence"])
            self.assertIn("verified_offsite_copy", evidence_report["missing_evidence"])
            offsite_report = steward.render_offsite_report(output_dir, snapshot_id="snap-001")
            self.assertTrue(offsite_report["required"])
            self.assertFalse(offsite_report["complete"])
            self.assertEqual(offsite_report["missing_evidence"], ["verified_offsite_copy"])
            self.assertEqual(offsite_report["destinations"][0]["id"], "offsite-restic-copy")
            self.assertEqual(offsite_report["destinations"][0]["repository_ref"], "repository://offsite-vault")
            self.assertEqual(offsite_report["destinations"][0]["repository_target"], "repository-target://offsite-vault")
            self.assertIn("evidence record", offsite_report["destinations"][0]["record_command"])
            evidence_record = steward.record_external_evidence(
                output_dir,
                snapshot_id="snap-001",
                evidence=["verified_offsite_copy"],
                source="external-monitor://offsite-copy-check",
                detail="offsite repository check passed",
                artifact_ref="artifact://private/offsite-check/run-001",
            )
            self.assertTrue(evidence_record["recorded"])
            evidence_report = steward.render_evidence_report(output_dir, snapshot_id="snap-001")
            self.assertIn("verified_offsite_copy", evidence_report["available_evidence"])
            self.assertIn("verified_snapshot", evidence_report["available_evidence"])
            self.assertIn("restore_drill", evidence_report["available_evidence"])
            offsite_report = steward.render_offsite_report(output_dir, snapshot_id="snap-001")
            self.assertTrue(offsite_report["complete"])
            self.assertEqual(offsite_report["missing_evidence"], [])
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                state_verification = steward.render_state_verification(output_dir, snapshot_id="snap-001")
            self.assertEqual(state_verification["schema_version"], "northroot.steward.state-verification.v0")
            self.assertTrue(state_verification["ready"])
            self.assertTrue(state_verification["safe_to_execute"])
            self.assertTrue(state_verification["retention_evidence_ready"])
            self.assertEqual(state_verification["snapshot_id"], "snap-001")
            self.assertIn("verify-state", {
                operation["name"] for operation in capabilities["allowed_operations"]
            })
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                custody_report = steward.render_report(output_dir, snapshot_id="snap-001")
            self.assertEqual(custody_report["schema_version"], "northroot.steward.report.v0")
            self.assertTrue(custody_report["preflight_ready"])
            self.assertTrue(custody_report["retention_evidence_ready"])
            self.assertEqual(custody_report["offsite_report"]["missing_evidence"], [])
            self.assertEqual(custody_report["evidence_report"]["snapshot_id"], "snap-001")
            self.assertEqual(custody_report["recommended_actions"], ["no custody action required by this report"])
            recorded_retention = steward.evaluate_retention(
                output_dir,
                snapshot_id="snap-001",
                available_evidence=[],
                use_recorded_evidence=True,
            )
            self.assertTrue(recorded_retention["allowed"])
            self.assertEqual(recorded_retention["missing_evidence"], [])
            stale_retention = steward.evaluate_retention(
                output_dir,
                snapshot_id="snap-002",
                available_evidence=[],
                use_recorded_evidence=True,
            )
            self.assertFalse(stale_retention["allowed"])
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                stale_state_verification = steward.render_state_verification(output_dir, snapshot_id="snap-002")
            self.assertFalse(stale_state_verification["ready"])
            self.assertFalse(stale_state_verification["retention_evidence_ready"])
            self.assertIn("verified_snapshot", stale_retention["missing_evidence"])
            self.assertIn("verified_offsite_copy", stale_retention["missing_evidence"])
            self.assertIn("restore_drill", stale_retention["missing_evidence"])
            tampered_summary_path = Path(str(evidence_record["run_summary_path"]))
            tampered_summary = model.load_json(tampered_summary_path)
            tampered_summary["snapshot_result"]["detail"] = "operator hand edit after steward indexing"
            tampered_summary_path.write_text(
                json.dumps(tampered_summary, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            tampered_evidence = steward.render_evidence_report(output_dir, snapshot_id="snap-001")
            self.assertFalse(tampered_evidence["run_summary_integrity"]["ok"])
            self.assertNotIn("verified_offsite_copy", tampered_evidence["available_evidence"])
            self.assertIn("verified_snapshot", tampered_evidence["available_evidence"])
            self.assertIn("restore_drill", tampered_evidence["available_evidence"])
            tampered_observations = {
                observation["summary_path"]: observation for observation in tampered_evidence["observations"]
            }
            self.assertEqual(tampered_observations[str(tampered_summary_path)]["status"], "digest-mismatch")
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                tampered_state_verification = steward.render_state_verification(output_dir, snapshot_id="snap-001")
            self.assertFalse(tampered_state_verification["ready"])
            self.assertIn("run_summary_integrity_failed", {
                check["code"] for check in tampered_state_verification["checks"] if check["code"]
            })
            with self.assertRaises(ValueError):
                steward.record_external_evidence(
                    output_dir,
                    snapshot_id="snap-001",
                    evidence=["restore_drill"],
                    source="external-monitor://not-allowed",
                )

            missing_repo_output_dir = Path(temp_dir) / "steward-missing-repository"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=missing_repo_output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
            )
            with mock.patch.dict(
                os.environ,
                {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"},
                clear=True,
            ):
                missing_repo_preflight = steward.render_preflight(missing_repo_output_dir)
            self.assertFalse(missing_repo_preflight["ready"])
            self.assertIn("missing_repository_bindings", {
                check["code"] for check in missing_repo_preflight["checks"] if check["code"]
            })

            with mock.patch.dict(os.environ, {"PATH": str(fake_bin)}, clear=True):
                not_ready = steward.render_preflight(output_dir)
            self.assertFalse(not_ready["ready"])
            self.assertIn("missing_secret_env", {
                check["code"] for check in not_ready["checks"] if check["code"]
            })
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin)}, clear=True):
                blocked_execute = steward.render_operation(output_dir, "run", execute=True)
            self.assertEqual(blocked_execute["return_code"], 78)
            self.assertFalse(blocked_execute["executed"])
            self.assertTrue(blocked_execute["execute_requested"])
            self.assertIn("preflight failed", blocked_execute["error"])
            blocked_summary = model.load_json(Path(str(blocked_execute["run_summary_path"])))
            self.assertEqual(blocked_summary["status"], "delegated-preflight-failed")
            self.assertFalse(blocked_summary["snapshot_result"]["executed"])
            self.assertFalse(blocked_summary["snapshot_result"]["preflight_ready"])

            unverified_restore_target = Path(temp_dir) / "empty-restore"
            empty_resticprofile = Path(temp_dir) / "empty-bin"
            empty_resticprofile.mkdir()
            write_fake_executable(empty_resticprofile, "resticprofile")
            write_fake_executable(empty_resticprofile, "op")
            write_fake_executable(empty_resticprofile, "security", "#!/bin/sh\nprintf '%s\\n' runtime-token\n")
            with mock.patch.dict(os.environ, {"PATH": str(empty_resticprofile)}, clear=True):
                unverified_restore = steward.render_operation(
                    output_dir,
                    "restore-drill",
                    execute=True,
                    restore_target=unverified_restore_target,
                )
            self.assertTrue(unverified_restore["executed"])
            self.assertEqual(unverified_restore["failure_stage"], "restore-observation")
            unverified_summary = model.load_json(Path(str(unverified_restore["run_summary_path"])))
            self.assertEqual(unverified_summary["status"], "delegated-restore-unverified")
            self.assertFalse(unverified_summary["verification_result"]["restore_verified"])

            write_fake_executable(fake_bin, "security", "#!/bin/sh\nprintf '%s\\n' runtime-token\n")
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin)}, clear=True):
                runtime_env_preflight = steward.render_preflight(output_dir)
                runtime_env_verify = steward.render_operation(output_dir, "verify", execute=True)
            self.assertTrue(runtime_env_preflight["ready"])
            self.assertIn("runtime_env:OP_SERVICE_ACCOUNT_TOKEN", {
                check["name"] for check in runtime_env_preflight["checks"]
            })
            self.assertTrue(runtime_env_verify["executed"])
            self.assertEqual(runtime_env_verify["return_code"], 0)

            keychain_output_dir = Path(temp_dir) / "steward-keychain"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=keychain_output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.macos-keychain.example.json",
                repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
            )
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin)}, clear=True):
                keychain_preflight = steward.render_preflight(keychain_output_dir)
            self.assertTrue(keychain_preflight["ready"])
            self.assertIn("secret_command:secret://restic/local-password", {
                check["name"] for check in keychain_preflight["checks"]
            })

            run = steward.render_operation(output_dir, "run")
            verify = steward.render_operation(output_dir, "verify")
            restore_drill = steward.render_operation(output_dir, "restore-drill")
            self.assertFalse(run["executed"])
            self.assertIn("steward.backup", run["command"])
            self.assertIn("steward.check", verify["command"])
            self.assertIn("steward.restore", restore_drill["command"])
            self.assertIn("restore-drills/latest", restore_drill["command"])
            self.assertTrue(Path(str(run["run_summary_path"])).exists())
            self.assertTrue(Path(str(verify["run_summary_path"])).exists())
            self.assertTrue(Path(str(restore_drill["run_summary_path"])).exists())

            summary = model.load_json(Path(str(run["run_summary_path"])))
            self.assertEqual(summary["schema_version"], model.RUN_SUMMARY_SCHEMA)
            self.assertEqual(summary["status"], "delegated-rendered")
            findings = model.validate_run_summary(summary)
            self.assertFalse([finding for finding in findings if finding.severity == "error"])
            self.assertIn("restore_not_verified", {finding.code for finding in findings})

            status_after_run = steward.render_status(output_dir)
            self.assertEqual(status_after_run["latest_run_summary_path"], restore_drill["run_summary_path"])

            schedule = steward.create_schedule(
                output_dir=output_dir,
                scheduler="systemd",
                every_minutes=30,
            )
            self.assertFalse(schedule["installed"])
            self.assertEqual(schedule["scheduler"], "systemd")
            self.assertEqual(schedule["operation"], "run")
            self.assertEqual(
                sorted(schedule["generated_artifacts"]),
                ["systemd_service", "systemd_timer"],
            )
            for artifact in schedule["generated_artifacts"].values():
                artifact_path = Path(str(artifact["path"]))
                self.assertEqual(artifact["sha256"], file_sha256(artifact_path))
            self.assertFalse(list((output_dir / "schedules").glob(".*.tmp")))
            self.assertTrue(Path(str(schedule["schedule_path"])).exists())
            self.assertIn(
                "--execute",
                (output_dir / "schedules" / "northroot-steward.service").read_text(encoding="utf-8"),
            )
            schedule_status = steward.schedule_status(output_dir)
            self.assertTrue(schedule_status["configured"])
            self.assertEqual(schedule_status["runner_command"], "nr steward")
            self.assertTrue(schedule_status["schedule_integrity"]["ok"])
            install_plan = steward.install_schedule(output_dir)
            self.assertFalse(install_plan["executed"])
            self.assertTrue(install_plan["preflight_required"])
            self.assertIsNone(install_plan["preflight"])
            self.assertIn("systemctl --user link", install_plan["commands"][0])
            self.assertIn("systemctl --user enable --now", install_plan["commands"][1])
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                missing_runner_preflight = steward.render_preflight(output_dir)
            self.assertFalse(missing_runner_preflight["ready"])
            self.assertIn("missing_scheduled_runner_command", {
                check["code"] for check in missing_runner_preflight["checks"] if check["code"]
            })
            write_fake_executable(fake_bin, "nr")
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                scheduled_preflight = steward.render_preflight(output_dir)
            self.assertTrue(scheduled_preflight["ready"])
            self.assertIn("scheduled_runner_command", {
                check["name"] for check in scheduled_preflight["checks"]
            })
            self.assertIn("schedule_generated_artifact:systemd_service", {
                check["name"] for check in scheduled_preflight["checks"]
            })
            service_path = output_dir / "schedules" / "northroot-steward.service"
            service_path.write_text(
                service_path.read_text(encoding="utf-8") + "# operator hand edit\n",
                encoding="utf-8",
            )
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                drifted_schedule_preflight = steward.render_preflight(output_dir)
            self.assertFalse(drifted_schedule_preflight["ready"])
            self.assertIn("schedule_generated_artifact_drift", {
                check["code"] for check in drifted_schedule_preflight["checks"] if check["code"]
            })
            with mock.patch.dict(os.environ, {"PATH": str(Path(temp_dir) / "missing-bin")}, clear=True):
                blocked_install = steward.install_schedule(output_dir, execute=True)
            self.assertFalse(blocked_install["executed"])
            self.assertEqual(blocked_install["return_code"], 78)
            self.assertFalse(blocked_install["preflight"]["ready"])
            uninstall_plan = steward.uninstall_schedule(output_dir)
            self.assertFalse(uninstall_plan["executed"])
            self.assertIn("systemctl --user disable --now", uninstall_plan["commands"][0])
            schedule_file = output_dir / "schedules" / "schedule.json"
            installed_schedule = model.load_json(schedule_file)
            installed_schedule["installed"] = True
            schedule_file.write_text(json.dumps(installed_schedule, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                tampered_schedule_preflight = steward.render_preflight(output_dir)
            self.assertFalse(tampered_schedule_preflight["ready"])
            self.assertIn("schedule_manifest_integrity_failed", {
                check["code"] for check in tampered_schedule_preflight["checks"] if check["code"]
            })
            blocked_delete = steward.delete_schedule(output_dir)
            self.assertFalse(blocked_delete["deleted"])
            self.assertTrue(blocked_delete["installed"])
            self.assertEqual(blocked_delete["schedule_integrity"]["status"], "digest-mismatch")
            self.assertIn("schedule manifest integrity failed", blocked_delete["error"])
            self.assertTrue(Path(str(schedule["schedule_path"])).exists())
            forced_delete = steward.delete_schedule(output_dir, force=True)
            self.assertTrue(forced_delete["deleted"])
            self.assertTrue(forced_delete["force"])
            self.assertFalse(Path(str(schedule["schedule_path"])).exists())

            orphan_path = output_dir / "schedules" / "northroot-steward.timer"
            orphan_path.parent.mkdir(parents=True, exist_ok=True)
            orphan_path.write_text("# interrupted schedule create\n", encoding="utf-8")
            orphan_status = steward.schedule_status(output_dir)
            self.assertFalse(orphan_status["configured"])
            self.assertFalse(orphan_status["schedule_integrity"]["ok"])
            self.assertEqual(orphan_status["schedule_integrity"]["status"], "orphaned-artifacts")
            self.assertIn(str(orphan_path), orphan_status["schedule_integrity"]["orphaned_paths"])
            with mock.patch.dict(os.environ, {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"}, clear=True):
                orphan_preflight = steward.render_preflight(output_dir)
            self.assertFalse(orphan_preflight["ready"])
            self.assertIn("schedule_manifest_integrity_failed", {
                check["code"] for check in orphan_preflight["checks"] if check["code"]
            })
            blocked_orphan_delete = steward.delete_schedule(output_dir)
            self.assertFalse(blocked_orphan_delete["deleted"])
            self.assertEqual(blocked_orphan_delete["schedule_integrity"]["status"], "orphaned-artifacts")
            forced_orphan_delete = steward.delete_schedule(output_dir, force=True)
            self.assertTrue(forced_orphan_delete["deleted"])
            self.assertIn(str(orphan_path), forced_orphan_delete["removed_paths"])
            self.assertFalse(orphan_path.exists())

            schedule = steward.create_schedule(
                output_dir=output_dir,
                scheduler="systemd",
                every_minutes=30,
                registry_state=registry_dir,
                project_id="project/example",
            )
            service_text = Path(str(schedule["schedule_path"])).with_suffix(".service").read_text(encoding="utf-8")
            self.assertIn("--registry-state", service_text)
            self.assertIn(str(registry_dir), service_text)
            self.assertIn("--project-id project/example", service_text)
            self.assertEqual(schedule["registry_state"], str(registry_dir))
            self.assertEqual(schedule["project_id"], "project/example")
            deleted = steward.delete_schedule(output_dir)
            self.assertTrue(deleted["configured_before_delete"])
            self.assertTrue(deleted["deleted"])
            self.assertFalse(Path(str(schedule["schedule_path"])).exists())
            self.assertFalse(steward.schedule_status(output_dir)["configured"])

            first_project_schedule = steward.create_schedule(
                output_dir=output_dir,
                scheduler="systemd",
                every_minutes=30,
                registry_state=registry_dir,
                project_id="project/example",
            )
            second_project_schedule = steward.create_schedule(
                output_dir=output_dir,
                scheduler="launchd",
                every_minutes=60,
                registry_state=registry_dir,
                project_id="project/release",
            )
            self.assertNotEqual(
                first_project_schedule["schedule_scope_id"],
                second_project_schedule["schedule_scope_id"],
            )
            self.assertNotEqual(
                Path(str(first_project_schedule["schedule_path"])).parent,
                Path(str(second_project_schedule["schedule_path"])).parent,
            )
            ambiguous_schedule = steward.schedule_status(output_dir)
            self.assertTrue(ambiguous_schedule["requires_schedule_context"])
            self.assertEqual(ambiguous_schedule["schedule_count"], 2)
            first_status = steward.schedule_status(
                output_dir,
                registry_state=registry_dir,
                project_id="project/example",
            )
            self.assertTrue(first_status["configured"])
            self.assertEqual(first_status["project_id"], "project/example")
            first_delete = steward.delete_schedule(
                output_dir,
                registry_state=registry_dir,
                project_id="project/example",
            )
            self.assertTrue(first_delete["deleted"])
            self.assertFalse(Path(str(first_project_schedule["schedule_path"])).exists())
            self.assertTrue(Path(str(second_project_schedule["schedule_path"])).exists())
            second_delete = steward.delete_schedule(
                output_dir,
                registry_state=registry_dir,
                project_id="project/release",
            )
            self.assertTrue(second_delete["deleted"])

            verify_schedule = steward.create_schedule(
                output_dir=output_dir,
                scheduler="launchd",
                every_minutes=45,
                operation="verify",
            )
            self.assertEqual(verify_schedule["operation"], "verify")
            self.assertEqual(sorted(verify_schedule["generated_artifacts"]), ["launchd_plist"])
            verify_artifact = verify_schedule["generated_artifacts"]["launchd_plist"]
            self.assertEqual(verify_artifact["sha256"], file_sha256(Path(str(verify_artifact["path"]))))
            self.assertFalse(list((output_dir / "schedules").glob(".*.tmp")))
            launchd_template = (output_dir / "schedules" / "org.northroot.steward.plist").read_text(encoding="utf-8")
            self.assertIn("nr steward verify --state", launchd_template)
            self.assertIn("--execute", launchd_template)
            self.assertEqual(steward.schedule_status(output_dir)["operation"], "verify")

            special_output_dir = Path(temp_dir) / "steward state & xml"
            steward.init_steward(
                inventory_path=EXAMPLES / "workspace-inventory.example.json",
                policy_path=EXAMPLES / "custody-policy.example.json",
                output_dir=special_output_dir,
                secret_bindings_path=EXAMPLES / "secret-bindings.redacted.example.json",
                repository_bindings_path=EXAMPLES / "repository-bindings.redacted.example.json",
            )
            special_schedule = steward.create_schedule(
                output_dir=special_output_dir,
                scheduler="launchd",
                every_minutes=15,
                runner_command="'/opt/Northroot Tools/nr' steward",
                operation="verify",
            )
            special_template = Path(str(special_schedule["schedule_path"])).read_text(encoding="utf-8")
            ET.fromstring(special_template)
            self.assertIn("/opt/Northroot Tools/nr", special_template)
            self.assertIn("&apos;/opt/Northroot Tools/nr&apos;", special_template)
            self.assertIn("steward state &amp; xml", special_template)

            special_systemd = steward.create_schedule(
                output_dir=special_output_dir,
                scheduler="systemd",
                every_minutes=15,
                runner_command="'/opt/Northroot Tools/nr' steward",
                operation="run",
            )
            special_service = (
                Path(str(special_systemd["schedule_path"])).with_suffix(".service").read_text(encoding="utf-8")
            )
            self.assertIn("ExecStart='/opt/Northroot Tools/nr' steward run --state", special_service)
            self.assertIn(shlex.quote(str(special_output_dir)), special_service)


if __name__ == "__main__":
    unittest.main()
