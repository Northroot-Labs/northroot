import json
import unittest
from pathlib import Path

from northroot.custody import model


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def load_example(name: str) -> dict[str, object]:
    return json.loads((EXAMPLES / name).read_text(encoding="utf-8"))


class CustodyModelTests(unittest.TestCase):
    def test_public_examples_validate(self) -> None:
        self.assertEqual(
            model.validate_workspace_inventory(
                load_example("workspace-inventory.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_custody_policy(
                load_example("custody-policy.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_secret_bindings(
                load_example("secret-bindings.redacted.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_repository_bindings(
                load_example("repository-bindings.redacted.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_secret_bindings(
                load_example("secret-bindings.macos-keychain.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_command_plan(
                load_example("command-plan.example.json"),
                public_safe=True,
            ),
            [],
        )
        self.assertEqual(
            model.validate_service_registry(
                load_example("service-registry.example.json"),
                public_safe=True,
            ),
            [],
        )

    def test_render_snapshot_plan_delegates_to_resticprofile(self) -> None:
        plan = model.render_snapshot_plan(
            load_example("workspace-inventory.example.json"),
            load_example("custody-policy.example.json"),
        )

        self.assertEqual(plan["schema_version"], model.SNAPSHOT_PLAN_SCHEMA)
        self.assertEqual(plan["adapter"], "resticprofile")
        self.assertFalse(plan["execution"]["custom_backup_engine"])
        self.assertIn("sample-restore", plan["verification_required"])
        self.assertIn("restore_drill", plan["retention_prune_requires"])
        self.assertEqual(plan["destinations"][0]["secret_ref"], "secret://restic/local-password")
        self.assertEqual(
            plan["object_restore_classes"]["full-restore"],
            ["state/sqlite", "journals/main"],
        )
        self.assertEqual(
            plan["object_restore_classes"]["never-export"],
            ["cache/runtime"],
        )
        self.assertEqual(
            plan["object_custody"][0]["storage_binding"],
            "workspace://.",
        )

    def test_object_custody_rejects_invalid_restore_and_raw_binding(self) -> None:
        inventory = load_example("workspace-inventory.example.json")
        inventory["objects"][0]["restore_class"] = "best-effort"
        inventory["objects"][1]["storage_binding"] = "/Users/example/private-state/app.sqlite"

        findings = model.validate_workspace_inventory(inventory, public_safe=True)

        self.assertIn("invalid_restore_class", {finding.code for finding in findings})
        self.assertIn("invalid_storage_binding", {finding.code for finding in findings})
        self.assertIn("public_private_binding", {finding.code for finding in findings})

    def test_object_custody_rejects_ephemeral_full_restore(self) -> None:
        inventory = load_example("workspace-inventory.example.json")
        inventory["objects"][3]["restore_class"] = "full-restore"

        findings = model.validate_workspace_inventory(inventory)

        self.assertIn("ephemeral_full_restore", {finding.code for finding in findings})

    def test_snapshot_plan_checks_object_restore_class_summary(self) -> None:
        plan = model.render_snapshot_plan(
            load_example("workspace-inventory.example.json"),
            load_example("custody-policy.example.json"),
        )
        plan["object_restore_classes"]["full-restore"] = []

        findings = model.validate_snapshot_plan(plan, public_safe=True)

        self.assertIn("object_restore_classes", {finding.code for finding in findings})

    def test_service_registry_rejects_unknown_project_permission(self) -> None:
        registry = load_example("service-registry.example.json")
        registry["projects"][0]["permission_set_ref"] = "perm/missing"

        findings = model.validate_service_registry(registry)

        self.assertIn("unknown_project_permission_set", {finding.code for finding in findings})

    def test_service_registry_rejects_mismatched_project_permission(self) -> None:
        registry = load_example("service-registry.example.json")
        registry["permissions"][0]["project_id"] = "project/other"

        findings = model.validate_service_registry(registry)

        self.assertIn("mismatched_project_permission", {finding.code for finding in findings})

    def test_service_registry_rejects_unknown_replica_source(self) -> None:
        registry = load_example("service-registry.example.json")
        registry["replicas"][0]["source_destination_id"] = "source/missing"

        findings = model.validate_service_registry(registry)

        self.assertIn("unknown_replica_source_destination", {finding.code for finding in findings})

    def test_service_registry_rejects_unsafe_resume_policy(self) -> None:
        registry = load_example("service-registry.example.json")
        registry["resume_policy"]["on_disconnected_storage"] = "continue"
        registry["resume_policy"]["partial_run_handling"] = "prune-anyway"

        findings = model.validate_service_registry(registry)

        self.assertIn("invalid_resume_failure_policy", {finding.code for finding in findings})
        self.assertIn("invalid_partial_run_handling", {finding.code for finding in findings})

    def test_public_service_registry_rejects_private_legacy_paths(self) -> None:
        registry = load_example("service-registry.example.json")
        registry["legacy_imports"][0]["runner_state_ref"] = "/Users/example/.northroot/state/runner-state.json"

        findings = model.validate_service_registry(registry, public_safe=True)

        self.assertIn("invalid_symbolic_ref", {finding.code for finding in findings})
        self.assertIn("public_private_binding", {finding.code for finding in findings})

    def test_public_policy_rejects_real_secret_reference(self) -> None:
        policy = load_example("custody-policy.example.json")
        policy["destinations"][0]["secret_ref"] = "op://Northroot/restic/password"

        findings = model.validate_custody_policy(policy, public_safe=True)

        self.assertIn("invalid_secret_ref", {finding.code for finding in findings})
        self.assertIn("public_private_binding", {finding.code for finding in findings})

    def test_public_secret_bindings_reject_real_onepassword_reference(self) -> None:
        bindings = load_example("secret-bindings.redacted.example.json")
        bindings["bindings"][0]["command"] = ["op", "read", "op://Northroot/restic/password"]

        findings = model.validate_secret_bindings(bindings, public_safe=True)

        self.assertIn("public_private_binding", {finding.code for finding in findings})

    def test_blocked_private_bindings_fixture_is_valid_but_not_public_safe(self) -> None:
        bindings = load_example("private-bindings.blocked.example.json")

        normal_findings = model.validate_secret_bindings(bindings)
        public_findings = model.validate_secret_bindings(bindings, public_safe=True)

        self.assertEqual(normal_findings, [])
        self.assertIn("public_private_binding", {finding.code for finding in public_findings})

    def test_secret_bindings_require_unattended_command(self) -> None:
        bindings = load_example("secret-bindings.redacted.example.json")
        bindings["bindings"][0]["interactive"] = True

        findings = model.validate_secret_bindings(bindings)

        self.assertIn("interactive_secret_binding", {finding.code for finding in findings})

    def test_runtime_env_bindings_require_unattended_command(self) -> None:
        bindings = load_example("secret-bindings.redacted.example.json")
        bindings["runtime_env"][0]["interactive"] = True

        findings = model.validate_secret_bindings(bindings)

        self.assertIn("interactive_runtime_env_binding", {finding.code for finding in findings})

    def test_public_safe_validation_rejects_private_bindings(self) -> None:
        inventory = load_example("workspace-inventory.example.json")
        inventory["state_roots"][0]["path"] = "/Volumes/X9 Pro/private-state"

        findings = model.validate_workspace_inventory(inventory, public_safe=True)

        self.assertIn("public_private_binding", {finding.code for finding in findings})

    def test_run_summary_warns_without_restore_verification(self) -> None:
        findings = model.validate_run_summary(
            {
                "schema_version": model.RUN_SUMMARY_SCHEMA,
                "run_id": "run-001",
                "workspace_id": "example-workspace",
                "status": "snapshot-complete",
                "snapshot_result": {"snapshot_ids": ["snap-001"]},
                "verification_result": {
                    "schema_version": model.VERIFICATION_RESULT_SCHEMA,
                    "repository_check": "ok",
                    "restore_verified": False,
                },
            }
        )

        self.assertIn("restore_not_verified", {finding.code for finding in findings})

    def test_command_plan_rejects_shell_and_inconsistent_success(self) -> None:
        command_plan = load_example("command-plan.example.json")
        command_plan["shell_required"] = True
        command_plan["argv_style"] = "shell"
        command_plan["missing_inputs"] = ["snapshot_id"]

        findings = model.validate_command_plan(command_plan)

        self.assertIn("shell_required", {finding.code for finding in findings})
        self.assertIn("argv_style", {finding.code for finding in findings})
        self.assertIn("ok_with_missing_inputs", {finding.code for finding in findings})

    def test_verification_result_is_first_class_contract(self) -> None:
        result = {
            "schema_version": model.VERIFICATION_RESULT_SCHEMA,
            "repository_check": "ok",
            "restore_verified": True,
            "restore_observation": {
                "target": "restore-target://example",
                "exists": True,
                "file_count": 1,
                "byte_count": 7,
                "manifest_sha256": "0" * 64,
                "verified": True,
            },
            "external_evidence": ["verified_offsite_copy"],
        }

        self.assertEqual(model.validate_verification_result(result, public_safe=True), [])
        self.assertEqual(model.validate_document(result, public_safe=True), [])

    def test_retention_decision_blocks_prune_until_required_evidence_exists(self) -> None:
        policy = load_example("custody-policy.example.json")

        blocked = model.evaluate_retention(
            policy,
            snapshot_id="snap-001",
            available_evidence=["verified_snapshot"],
        )
        allowed = model.evaluate_retention(
            policy,
            snapshot_id="snap-001",
            available_evidence=["verified_snapshot", "verified_offsite_copy", "restore_drill"],
        )

        self.assertFalse(blocked["allowed"])
        self.assertEqual(blocked["missing_evidence"], ["verified_offsite_copy", "restore_drill"])
        self.assertTrue(allowed["allowed"])
        self.assertEqual(model.validate_retention_decision(allowed, public_safe=True), [])

    def test_build_run_summary_with_verified_restore_and_retention_decision(self) -> None:
        policy = load_example("custody-policy.example.json")
        retention_decision = model.evaluate_retention(
            policy,
            snapshot_id="snap-001",
            available_evidence=["verified_snapshot", "verified_offsite_copy", "restore_drill"],
        )
        summary = model.build_run_summary(
            run_id="run-001",
            workspace_id="example-workspace",
            status="verified",
            snapshot_result={"snapshot_ids": ["snap-001"]},
            verification_result={
                "schema_version": model.VERIFICATION_RESULT_SCHEMA,
                "repository_check": "ok",
                "restore_verified": True,
                "restore_observation": {
                    "target": "restore-target://example",
                    "exists": True,
                    "file_count": 1,
                    "byte_count": 7,
                    "manifest_sha256": "0" * 64,
                    "verified": True,
                },
            },
            retention_decision=retention_decision,
            tool_invocations=[
                {
                    "tool": "resticprofile",
                    "operation": "backup",
                    "executed": False
                }
            ],
        )

        self.assertEqual(summary["schema_version"], model.RUN_SUMMARY_SCHEMA)
        self.assertEqual(model.validate_run_summary(summary, public_safe=True), [])


if __name__ == "__main__":
    unittest.main()
