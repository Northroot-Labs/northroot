import json
import tempfile
import unittest
from pathlib import Path

from northroot.custody import model, registry


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


class RegistryTests(unittest.TestCase):
    def test_authorize_operation_evaluates_project_and_object_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )

            allowed_project = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                public_safe=True,
            )
            self.assertTrue(allowed_project["allowed"])
            self.assertEqual(allowed_project["decision"], "allowed")
            self.assertEqual(allowed_project["matched_permission_sets"], ["perm/project-example"])

            blocked_secret_object = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                object_id="secrets/restic-env",
                public_safe=True,
            )
            self.assertFalse(blocked_secret_object["allowed"])
            self.assertEqual(blocked_secret_object["decision"], "blocked")
            self.assertIn("perm/object-secrets", blocked_secret_object["matched_permission_sets"])

            allowed_secret_preflight = registry.authorize_operation(
                state_dir,
                operation="preflight",
                project_id="project/example",
                object_id="secrets/restic-env",
                public_safe=True,
            )
            self.assertTrue(allowed_secret_preflight["allowed"])
            self.assertEqual(allowed_secret_preflight["decision"], "allowed")

            human_restore = registry.authorize_operation(
                state_dir,
                operation="restore",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(human_restore["allowed"])
            self.assertTrue(human_restore["requires_human_clearance"])
            self.assertEqual(human_restore["decision"], "human-clearance-required")

            not_allowed_source_bind = registry.authorize_operation(
                state_dir,
                operation="source.bind",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(not_allowed_source_bind["allowed"])
            self.assertEqual(not_allowed_source_bind["decision"], "not-allowed")

            unknown_object = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                object_id="artifact/missing",
                public_safe=True,
            )
            self.assertFalse(unknown_object["allowed"])
            self.assertEqual(unknown_object["decision"], "unknown-project-object")

            write_json(
                state_dir / "service-registry.lock.json",
                {
                    "schema_version": "northroot.steward.registry-operation-lock.v0",
                    "operation_id": "interrupted-test",
                    "operation": "registry.project.register",
                    "started_at": "2026-06-22T00:00:00Z",
                    "pid": 123,
                    "failure_policy": "fail-closed-record-summary",
                },
            )
            locked = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(locked["allowed"])
            self.assertEqual(locked["decision"], "resume-required")

    def test_registry_state_mutations_are_validated_and_recoverable(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            source_registry = model.load_json(EXAMPLES / "service-registry.example.json")
            initialized = registry.initialize_registry(state_dir, source_registry, public_safe=True)
            self.assertTrue(initialized["initialized"])
            self.assertTrue((state_dir / "service-registry.json").exists())
            self.assertTrue(Path(str(initialized["operation_summary_path"])).exists())

            status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(status["ready"])
            self.assertEqual(status["node_id"], "node-example")
            self.assertEqual(status["project_count"], 1)
            self.assertEqual(status["object_count"], 5)

            new_object = {
                "object_id": "artifact/release-bundle",
                "object_type": "artifact-dir",
                "visibility": "private",
                "storage_binding": "artifact://release-bundle",
                "custody_policy": {
                    "backup": "delegated",
                    "verification": "sample-restore",
                },
                "redaction_policy": {
                    "public_summary": "object-id-and-status",
                },
                "restore_class": "full-restore",
            }
            added_object = registry.add_object(state_dir, new_object, public_safe=True)
            self.assertTrue(added_object["mutated"])

            project_permission = {
                "permission_set_id": "perm/project-release",
                "scope": "project",
                "project_id": "project/release",
                "allowed_operations": [
                    "status",
                    "preflight",
                    "verify-state",
                    "report",
                    "run",
                    "verify",
                    "restore-drill",
                    "retention.evaluate",
                    "evidence.report",
                    "offsite.report",
                    "source.bind",
                    "replica.sync",
                ],
                "requires_human_clearance": [
                    "restore",
                    "schedule.install",
                    "schedule.uninstall",
                    "schedule.delete",
                ],
            }
            project = {
                "project_id": "project/release",
                "workspace_id": "release-workspace",
                "node_ref": "node://example",
                "permission_set_ref": "perm/project-release",
                "object_ids": [
                    "repo/main",
                    "artifact/release-bundle",
                ],
                "source_destination_ids": [
                    "source/release-primary",
                ],
                "schedule_ref": "scheduler://steward/release-hourly",
            }
            registered = registry.register_project(
                state_dir,
                project=project,
                permission=project_permission,
                public_safe=True,
            )
            self.assertTrue(registered["mutated"])

            destination = {
                "destination_id": "dest/release-replica",
                "role": "replica",
                "adapter": "external-delegated",
                "storage_binding": "repository://release-replica",
                "visibility": "private",
            }
            source_destination = {
                "source_destination_id": "source/release-primary",
                "project_id": "project/release",
                "destination_id": "dest/local-primary",
                "permission_set_ref": "perm/project-release",
                "object_ids": [
                    "repo/main",
                    "artifact/release-bundle",
                ],
                "consistency_boundary_ids": [
                    "journal-seal",
                ],
            }
            replica = {
                "replica_id": "replica/release-offsite",
                "source_destination_id": "source/release-primary",
                "destination_id": "dest/release-replica",
                "execution_model": "external-delegated",
                "required_evidence": [
                    "verified_offsite_copy",
                ],
                "resume_policy_ref": "resume/fail-closed-v0",
            }
            self.assertTrue(registry.add_destination(state_dir, destination, public_safe=True)["mutated"])
            self.assertTrue(registry.bind_source_destination(state_dir, source_destination, public_safe=True)["mutated"])
            self.assertTrue(registry.add_replica(state_dir, replica, public_safe=True)["mutated"])

            legacy_import = {
                "import_id": "legacy/release-machine-durability",
                "source": "legacy-machine-durability",
                "scheduler_ref": "scheduler://legacy-release-machine-durability",
                "machine_node_ref": "node://legacy-release-machine",
                "project_nodes_ref": "project://legacy-release-project-nodes",
                "runner_state_ref": "run-state://legacy-release-machine-durability",
                "per_run_state_ref": "state://legacy-release-machine-durability/runs",
                "import_mode": "metadata-only",
                "status": "pending",
            }
            self.assertTrue(registry.record_legacy_import(state_dir, legacy_import, public_safe=True)["mutated"])

            final_registry = registry.load_registry(state_dir)
            self.assertEqual(model.validate_service_registry(final_registry, public_safe=True), [])
            final_status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(final_status["ready"])
            self.assertEqual(final_status["project_count"], 2)
            self.assertEqual(final_status["replica_count"], 2)
            self.assertEqual(final_status["legacy_import_count"], 2)
            operation_summaries = sorted((state_dir / "registry-operations").glob("*.json"))
            self.assertGreaterEqual(len(operation_summaries), 7)

            lock = {
                "schema_version": "northroot.steward.registry-operation-lock.v0",
                "operation_id": "interrupted-test",
                "operation": "registry.project.register",
                "started_at": "2026-06-22T00:00:00Z",
                "pid": 123,
                "failure_policy": "fail-closed-record-summary",
            }
            write_json(state_dir / "service-registry.lock.json", lock)
            locked_status = registry.registry_status(state_dir, public_safe=True)
            self.assertFalse(locked_status["ready"])
            self.assertTrue(locked_status["resume_required"])
            with self.assertRaises(registry.RegistryLockedError):
                registry.add_object(
                    state_dir,
                    {
                        **new_object,
                        "object_id": "artifact/blocked-while-locked",
                        "storage_binding": "artifact://blocked-while-locked",
                    },
                    public_safe=True,
                )
            recovered = registry.recover_registry(state_dir, public_safe=True)
            self.assertTrue(recovered["recovered"])
            self.assertFalse((state_dir / "service-registry.lock.json").exists())
            self.assertTrue(registry.registry_status(state_dir, public_safe=True)["ready"])

    def test_public_safe_mutation_rejects_raw_private_paths_without_corrupting_registry(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            source_registry = model.load_json(EXAMPLES / "service-registry.example.json")
            registry.initialize_registry(state_dir, source_registry, public_safe=True)
            before = registry.registry_status(state_dir, public_safe=True)["registry_sha256"]
            unsafe_object = {
                "object_id": "artifact/private-path",
                "object_type": "artifact-dir",
                "visibility": "private",
                "storage_binding": "artifact://safe-ref",
                "custody_policy": {
                    "backup": "delegated",
                    "note": "/Users/example/private-state",
                },
                "redaction_policy": {
                    "public_summary": "object-id-and-status",
                },
                "restore_class": "full-restore",
            }
            with self.assertRaises(ValueError):
                registry.add_object(state_dir, unsafe_object, public_safe=True)
            self.assertFalse((state_dir / "service-registry.lock.json").exists())
            status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(status["ready"])
            self.assertEqual(status["registry_sha256"], before)


if __name__ == "__main__":
    unittest.main()
