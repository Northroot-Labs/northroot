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
            self.assertFalse((state_dir / "service-registry.lock.json").exists())

            status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(status["ready"])
            self.assertTrue(status["protected_state_ok"])
            self.assertEqual(status["registry_sha256"], status["expected_registry_sha256"])
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
            self.assertTrue(final_status["integrity"]["protected_state_ok"])
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
                "before_sha256": final_status["registry_sha256"],
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
            self.assertEqual(recovered["resume_state"], "registry-unchanged-after-lock")
            self.assertFalse(recovered["registry_changed_since_lock"])
            self.assertFalse((state_dir / "service-registry.lock.json").exists())
            self.assertTrue(registry.registry_status(state_dir, public_safe=True)["ready"])

    def test_registry_recover_handles_interrupted_initialization(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            state_dir.mkdir()
            service_registry = model.load_json(EXAMPLES / "service-registry.example.json")
            lock = {
                "schema_version": "northroot.steward.registry-operation-lock.v0",
                "operation_id": "interrupted-init",
                "operation": "registry.init",
                "started_at": "2026-06-22T00:00:00Z",
                "pid": 123,
                "registry_path": str(registry.registry_path(state_dir)),
                "before_sha256": None,
                "failure_policy": "fail-closed-record-summary",
            }
            write_json(registry.lock_path(state_dir), lock)
            write_json(registry.registry_path(state_dir), service_registry)

            locked_status = registry.registry_status(state_dir, public_safe=True)
            self.assertFalse(locked_status["ready"])
            self.assertTrue(locked_status["resume_required"])

            recovered = registry.recover_registry(state_dir, public_safe=True)

            self.assertTrue(recovered["recovered"])
            self.assertFalse(recovered["resume_required"])
            self.assertEqual(recovered["resume_state"], "registry-change-unknown")
            self.assertFalse(registry.lock_path(state_dir).exists())
            recovered_summary = model.load_json(Path(str(recovered["operation_summary_path"])))
            self.assertEqual(recovered_summary["status"], "recovered-after-interruption")
            self.assertEqual(recovered_summary["resume_state"], "registry-change-unknown")
            recovered_status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(recovered_status["ready"])
            self.assertTrue(recovered_status["protected_state_ok"])

    def test_registry_recover_clears_interrupted_initialization_before_registry_write(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            state_dir.mkdir()
            lock = {
                "schema_version": "northroot.steward.registry-operation-lock.v0",
                "operation_id": "interrupted-init-before-write",
                "operation": "registry.init",
                "started_at": "2026-06-22T00:00:00Z",
                "pid": 123,
                "registry_path": str(registry.registry_path(state_dir)),
                "before_sha256": None,
                "failure_policy": "fail-closed-record-summary",
            }
            write_json(registry.lock_path(state_dir), lock)

            recovered = registry.recover_registry(state_dir, public_safe=True)

            self.assertTrue(recovered["recovered"])
            self.assertEqual(recovered["resume_state"], "registry-missing-after-lock")
            self.assertFalse(registry.lock_path(state_dir).exists())
            missing_status = registry.registry_status(state_dir, public_safe=True)
            self.assertFalse(missing_status["configured"])
            initialized = registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            self.assertTrue(initialized["initialized"])
            self.assertTrue(registry.registry_status(state_dir, public_safe=True)["ready"])

    def test_registry_recovery_records_landed_interrupted_write(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            before = registry.registry_status(state_dir, public_safe=True)["registry_sha256"]
            lock = {
                "schema_version": "northroot.steward.registry-operation-lock.v0",
                "operation_id": "interrupted-after-registry-write",
                "operation": "registry.object.add",
                "started_at": "2026-06-22T00:00:00Z",
                "pid": 123,
                "registry_path": str(registry.registry_path(state_dir)),
                "before_sha256": before,
                "failure_policy": "fail-closed-record-summary",
            }
            write_json(registry.lock_path(state_dir), lock)
            landed = registry.load_registry(state_dir)
            landed["objects"].append(
                {
                    "object_id": "artifact/interrupted-write",
                    "object_type": "artifact-dir",
                    "visibility": "private",
                    "storage_binding": "artifact://interrupted-write",
                    "custody_policy": {
                        "backup": "delegated",
                        "verification": "sample-restore",
                    },
                    "redaction_policy": {
                        "public_summary": "object-id-and-status",
                    },
                    "restore_class": "full-restore",
                }
            )
            write_json(registry.registry_path(state_dir), landed)

            locked_status = registry.registry_status(state_dir, public_safe=True)
            self.assertFalse(locked_status["ready"])
            self.assertTrue(locked_status["resume_required"])

            recovered = registry.recover_registry(state_dir, public_safe=True)

            self.assertTrue(recovered["recovered"])
            self.assertEqual(recovered["resume_state"], "registry-changed-after-lock")
            self.assertTrue(recovered["registry_changed_since_lock"])
            summary = model.load_json(Path(str(recovered["operation_summary_path"])))
            self.assertEqual(summary["status"], "recovered-after-interruption")
            self.assertEqual(summary["resume_state"], "registry-changed-after-lock")
            self.assertEqual(summary["interrupted_before_sha256"], before)
            self.assertNotEqual(summary["registry_sha256"], before)
            self.assertFalse(registry.lock_path(state_dir).exists())
            status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(status["ready"])
            self.assertEqual(status["object_count"], 6)

    def test_registry_integrity_detects_valid_but_unproven_state_change(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            before = registry.registry_integrity_report(state_dir, public_safe=True)
            self.assertTrue(before["ready"])
            self.assertTrue(before["protected_state_ok"])

            tampered = registry.load_registry(state_dir)
            tampered["service_id"] = "steward/tampered-but-structurally-valid"
            write_json(state_dir / "service-registry.json", tampered)

            integrity = registry.registry_integrity_report(state_dir, public_safe=True)
            self.assertFalse(integrity["ready"])
            self.assertFalse(integrity["protected_state_ok"])
            self.assertNotEqual(integrity["registry_sha256"], integrity["expected_registry_sha256"])
            self.assertIn(
                "registry_digest_mismatch",
                {check["code"] for check in integrity["checks"] if check["code"]},
            )
            status = registry.registry_status(state_dir, public_safe=True)
            self.assertFalse(status["ready"])
            self.assertFalse(status["protected_state_ok"])
            authorization = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(authorization["allowed"])
            self.assertEqual(authorization["decision"], "invalid-registry")

    def test_registry_integrity_detects_tampered_operation_summary(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            before = registry.registry_integrity_report(state_dir, public_safe=True)
            self.assertTrue(before["ready"])
            self.assertTrue(before["operation_log_integrity"]["ok"])
            self.assertTrue((state_dir / "registry-operations" / "index.json").exists())

            operation_summary = sorted(
                path for path in (state_dir / "registry-operations").glob("*.json") if path.name != "index.json"
            )[0]
            tampered = model.load_json(operation_summary)
            tampered["completed_at"] = "20260622T000000000000Z"
            write_json(operation_summary, tampered)

            operation_log_integrity = registry.registry_operation_log_integrity(state_dir)
            self.assertFalse(operation_log_integrity["ok"])
            self.assertEqual(operation_log_integrity["observations"][0]["status"], "digest-mismatch")

            integrity = registry.registry_integrity_report(state_dir, public_safe=True)
            self.assertFalse(integrity["ready"])
            self.assertFalse(integrity["protected_state_ok"])
            self.assertFalse(integrity["operation_log_integrity"]["ok"])
            self.assertIn(
                "invalid_registry_operation_log",
                {check["code"] for check in integrity["checks"] if check["code"]},
            )
            authorization = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(authorization["allowed"])
            self.assertEqual(authorization["decision"], "invalid-registry")

    def test_registry_status_and_authorization_fail_closed_for_unreadable_registry(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            (state_dir / "service-registry.json").write_text("{not-json\n", encoding="utf-8")

            status = registry.registry_status(state_dir, public_safe=True)

            self.assertFalse(status["ready"])
            self.assertFalse(status["protected_state_ok"])
            self.assertEqual(status["error_count"], 1)
            self.assertEqual(status["findings"][0]["code"], "unreadable_service_registry")
            self.assertFalse(status["integrity"]["ready"])
            self.assertIn(
                "unreadable_service_registry",
                {check["code"] for check in status["integrity"]["checks"] if check["code"]},
            )
            authorization = registry.authorize_operation(
                state_dir,
                operation="run",
                project_id="project/example",
                public_safe=True,
            )
            self.assertFalse(authorization["allowed"])
            self.assertEqual(authorization["decision"], "invalid-registry")
            self.assertEqual(authorization["findings"][0]["code"], "unreadable_service_registry")

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

    def test_import_legacy_profile_applies_sanitized_batch_idempotently(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            state_dir = Path(temp_dir) / "registry-state"
            registry.initialize_registry(
                state_dir,
                model.load_json(EXAMPLES / "service-registry.example.json"),
                public_safe=True,
            )
            legacy_import = model.load_json(EXAMPLES / "legacy-profile-import.redacted.example.json")

            imported = registry.import_legacy_profile(state_dir, legacy_import, public_safe=True)

            self.assertTrue(imported["mutated"])
            self.assertEqual(imported["operation"], "registry.legacy-profile.import")
            self.assertEqual(imported["schema_version"], "northroot.steward.legacy-profile-import-result.v0")
            self.assertEqual(imported["imported_counts"]["objects"]["inserted"], 3)
            self.assertEqual(imported["imported_counts"]["legacy_imports"]["inserted"], 1)
            imported_status = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(imported_status["ready"])
            self.assertEqual(imported_status["project_count"], 2)
            self.assertEqual(imported_status["object_count"], 8)
            self.assertEqual(imported_status["destination_count"], 6)
            self.assertEqual(imported_status["source_destination_count"], 2)
            self.assertEqual(imported_status["replica_count"], 2)
            self.assertEqual(imported_status["legacy_import_count"], 2)

            replayed = registry.import_legacy_profile(state_dir, legacy_import, public_safe=True)

            self.assertTrue(replayed["mutated"])
            self.assertEqual(replayed["imported_counts"]["objects"]["inserted"], 0)
            self.assertEqual(replayed["imported_counts"]["objects"]["skipped_existing"], 3)
            self.assertEqual(replayed["imported_counts"]["legacy_imports"]["skipped_existing"], 1)
            replayed_status = registry.registry_status(state_dir, public_safe=True)
            self.assertEqual(replayed_status["registry_sha256"], imported_status["registry_sha256"])

            conflicting_import = model.load_json(EXAMPLES / "legacy-profile-import.redacted.example.json")
            conflicting_import["objects"][0]["storage_binding"] = "workspace://other-legacy-project"
            before_conflict = registry.registry_status(state_dir, public_safe=True)["registry_sha256"]
            with self.assertRaises(ValueError):
                registry.import_legacy_profile(state_dir, conflicting_import, public_safe=True)
            self.assertFalse((state_dir / "service-registry.lock.json").exists())
            after_conflict = registry.registry_status(state_dir, public_safe=True)
            self.assertTrue(after_conflict["ready"])
            self.assertEqual(after_conflict["registry_sha256"], before_conflict)


if __name__ == "__main__":
    unittest.main()
