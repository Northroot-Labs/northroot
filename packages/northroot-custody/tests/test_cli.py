import contextlib
import io
import json
import os
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from northroot.custody import cli, steward


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def write_fake_executable(directory: Path, name: str, body: str = "#!/bin/sh\nexit 0\n") -> None:
    path = directory / name
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)


class CliTests(unittest.TestCase):
    def test_steward_registry_cli_manages_service_registry_state(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            state_dir = temp_path / "registry-state"
            object_path = temp_path / "object.json"
            permission_path = temp_path / "permission.json"
            project_path = temp_path / "project.json"

            object_path.write_text(
                json.dumps(
                    {
                        "object_id": "artifact/cli-release-bundle",
                        "object_type": "artifact-dir",
                        "visibility": "private",
                        "storage_binding": "artifact://cli-release-bundle",
                        "custody_policy": {
                            "backup": "delegated",
                            "verification": "sample-restore",
                        },
                        "redaction_policy": {
                            "public_summary": "object-id-and-status",
                        },
                        "restore_class": "full-restore",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            permission_path.write_text(
                json.dumps(
                    {
                        "permission_set_id": "perm/cli-project",
                        "scope": "project",
                        "project_id": "project/cli",
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
                        ],
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            project_path.write_text(
                json.dumps(
                    {
                        "project_id": "project/cli",
                        "workspace_id": "cli-workspace",
                        "node_ref": "node://example",
                        "permission_set_ref": "perm/cli-project",
                        "object_ids": [
                            "repo/main",
                            "artifact/cli-release-bundle",
                        ],
                        "source_destination_ids": [
                            "source/cli-primary",
                        ],
                        "schedule_ref": "scheduler://steward/cli-hourly",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "init",
                            "--state",
                            str(state_dir),
                            "--registry",
                            str(EXAMPLES / "service-registry.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "topology",
                            "--state",
                            str(state_dir),
                            "--project-id",
                            "project/example",
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "verify",
                            "--state",
                            str(state_dir),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "import-legacy-profile",
                            "--state",
                            str(state_dir),
                            "--json",
                            str(EXAMPLES / "legacy-profile-import.redacted.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "add-object",
                            "--state",
                            str(state_dir),
                            "--json",
                            str(object_path),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "authorize",
                            "--state",
                            str(state_dir),
                            "--operation",
                            "legacy.import",
                            "--project-id",
                            "project/legacy-import",
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "register-project",
                            "--state",
                            str(state_dir),
                            "--project-json",
                            str(project_path),
                            "--permission-json",
                            str(permission_path),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "status",
                            "--state",
                            str(state_dir),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "authorize",
                            "--state",
                            str(state_dir),
                            "--operation",
                            "run",
                            "--project-id",
                            "project/cli",
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "authorize",
                            "--state",
                            str(state_dir),
                            "--operation",
                            "restore",
                            "--project-id",
                            "project/cli",
                            "--public-safe",
                        ]
                    ),
                    1,
                )
            self.assertIn('"project_count": 3', stdout.getvalue())
            self.assertIn('"schema_version": "northroot.steward.legacy-profile-import-result.v0"', stdout.getvalue())
            self.assertIn('"schema_version": "northroot.steward.registry-topology.v0"', stdout.getvalue())
            self.assertIn('"fail_closed_on_disconnected_storage": true', stdout.getvalue())
            self.assertIn('"decision": "allowed"', stdout.getvalue())
            self.assertIn('"decision": "not-allowed"', stdout.getvalue())
            self.assertTrue((state_dir / "registry-operations").exists())

    def test_validate_render_plan_and_steward_flow(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir) / "steward"
            stdout = io.StringIO()

            with contextlib.redirect_stdout(stdout):
                self.assertEqual(
                    cli.main(
                        [
                            "validate",
                            str(EXAMPLES / "workspace-inventory.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "render-plan",
                            "--inventory",
                            str(EXAMPLES / "workspace-inventory.example.json"),
                            "--policy",
                            str(EXAMPLES / "custody-policy.example.json"),
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "validate",
                            str(EXAMPLES / "secret-bindings.redacted.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "validate",
                            str(EXAMPLES / "repository-bindings.redacted.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "validate",
                            str(EXAMPLES / "legacy-run-import.redacted.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "evaluate-retention",
                            "--policy",
                            str(EXAMPLES / "custody-policy.example.json"),
                            "--snapshot-id",
                            "snap-001",
                            "--evidence",
                            "verified_snapshot",
                            "--evidence",
                            "verified_offsite_copy",
                            "--evidence",
                            "restore_drill",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "init",
                            "--inventory",
                            str(EXAMPLES / "workspace-inventory.example.json"),
                            "--policy",
                            str(EXAMPLES / "custody-policy.example.json"),
                            "--output",
                            str(output_dir),
                            "--secret-bindings",
                            str(EXAMPLES / "secret-bindings.redacted.example.json"),
                            "--repository-bindings",
                            str(EXAMPLES / "repository-bindings.redacted.example.json"),
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "import-legacy-runs",
                            "--state",
                            str(output_dir),
                            "--json",
                            str(EXAMPLES / "legacy-run-import.redacted.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(cli.main(["steward", "status", "--state", str(output_dir)]), 0)
                self.assertEqual(cli.main(["steward", "capabilities", "--state", str(output_dir)]), 0)
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "command-plan",
                            "--state",
                            str(output_dir),
                            "--operation",
                            "report",
                            "--snapshot-id",
                            "snap-001",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "command-plan",
                            "--state",
                            str(output_dir),
                            "--operation",
                            "import-legacy-runs",
                            "--json",
                            str(EXAMPLES / "legacy-run-import.redacted.example.json"),
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "command-plan",
                            "--state",
                            str(output_dir),
                            "--operation",
                            "branch.create",
                            "--branch",
                            "codex/cli-dogfood-policy",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "command-plan",
                            "--state",
                            str(output_dir),
                            "--operation",
                            "restore",
                            "--snapshot-id",
                            "snap-001",
                            "--target",
                            str(Path(temp_dir) / "recovery"),
                            "--execute",
                        ]
                    ),
                    1,
                )
                registry_dir = Path(temp_dir) / "registry-state"
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "registry",
                            "init",
                            "--state",
                            str(registry_dir),
                            "--registry",
                            str(EXAMPLES / "service-registry.example.json"),
                            "--public-safe",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "command-plan",
                            "--state",
                            str(output_dir),
                            "--operation",
                            "run",
                            "--registry-state",
                            str(registry_dir),
                            "--project-id",
                            "project/example",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "run",
                            "--state",
                            str(output_dir),
                            "--registry-state",
                            str(registry_dir),
                            "--project-id",
                            "project/example",
                            "--object-id",
                            "secrets/restic-env",
                            "--execute",
                        ]
                    ),
                    1,
                )
                self.assertEqual(
                    cli.main(["steward", "report", "--state", str(output_dir), "--snapshot-id", "snap-001"]),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "retention",
                            "evaluate",
                            "--state",
                            str(output_dir),
                            "--snapshot-id",
                            "snap-001",
                            "--evidence",
                            "verified_snapshot",
                        ]
                    ),
                    1,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "retention",
                            "evaluate",
                            "--state",
                            str(output_dir),
                            "--snapshot-id",
                            "snap-001",
                            "--evidence",
                            "verified_snapshot",
                            "--evidence",
                            "verified_offsite_copy",
                            "--evidence",
                            "restore_drill",
                        ]
                    ),
                    0,
                )
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
                with mock.patch.dict(
                    os.environ,
                    {"PATH": str(fake_bin), "OP_SERVICE_ACCOUNT_TOKEN": "dummy"},
                    clear=True,
                ):
                    self.assertEqual(cli.main(["steward", "preflight", "--state", str(output_dir)]), 0)
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "verify-state",
                                "--state",
                                str(output_dir),
                                "--registry-state",
                                str(registry_dir),
                                "--project-id",
                                "project/example",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "verify",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                                "--execute",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "restore-drill",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                                "--execute",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "restore",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-recovery",
                                "--target",
                                str(Path(temp_dir) / "recovery-restore"),
                                "--execute",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "verify-state",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                            ]
                        ),
                        1,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "offsite",
                                "report",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                            ]
                        ),
                        1,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "evidence",
                                "record",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                                "--evidence",
                                "verified_offsite_copy",
                                "--source",
                                "external-monitor://offsite-copy-check",
                                "--artifact-ref",
                                "artifact://private/offsite-check/run-001",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "offsite",
                                "report",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                            ]
                        ),
                        0,
                    )
                    self.assertEqual(cli.main(["steward", "evidence", "report", "--state", str(output_dir)]), 0)
                    self.assertEqual(
                        cli.main(
                            [
                                "steward",
                                "verify-state",
                                "--state",
                                str(output_dir),
                                "--snapshot-id",
                                "snap-001",
                            ]
                        ),
                        0,
                    )
                with mock.patch.dict(os.environ, {"PATH": str(fake_bin)}, clear=True):
                    self.assertEqual(cli.main(["steward", "preflight", "--state", str(output_dir)]), 1)
                    self.assertEqual(cli.main(["steward", "verify-state", "--state", str(output_dir)]), 1)
                    self.assertEqual(cli.main(["steward", "run", "--state", str(output_dir), "--execute"]), 1)
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "retention",
                            "evaluate",
                            "--state",
                            str(output_dir),
                            "--snapshot-id",
                            "snap-001",
                            "--use-recorded-evidence",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(["steward", "run", "--state", str(output_dir), "--snapshot-id", "snap-001"]),
                    0,
                )
                self.assertEqual(
                    cli.main(["steward", "restore-drill", "--state", str(output_dir), "--snapshot-id", "snap-001"]),
                    0,
                )
                (output_dir / steward.OPERATION_LOCK_FILENAME).write_text(
                    json.dumps(
                        {
                            "schema_version": "northroot.steward.operation-lock.v0",
                            "operation_id": "cli-stale-operation",
                            "operation": "verify",
                            "state": str(output_dir),
                            "command": "resticprofile --name steward check",
                            "command_args": ["resticprofile", "--name", "steward", "check"],
                            "snapshot_id": "snap-001",
                            "restore_target": None,
                            "registry_state": None,
                            "project_id": None,
                            "object_id": None,
                            "pid": 999999,
                            "started_at": "20260622T000000000000Z",
                            "failure_policy": "fail-closed-record-summary-before-retry",
                            "resume_hint": "run steward recover-operation before retrying delegated execution",
                        },
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n",
                    encoding="utf-8",
                )
                self.assertEqual(cli.main(["steward", "recover-operation", "--state", str(output_dir)]), 0)
                self.assertFalse((output_dir / steward.OPERATION_LOCK_FILENAME).exists())
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "schedule",
                            "create",
                            "--state",
                            str(output_dir),
                            "--scheduler",
                            "launchd",
                            "--operation",
                            "verify",
                            "--every-minutes",
                            "60",
                        ]
                    ),
                    0,
                )
                self.assertEqual(cli.main(["steward", "schedule", "status", "--state", str(output_dir)]), 0)
                self.assertEqual(cli.main(["steward", "schedule", "install", "--state", str(output_dir)]), 0)
                self.assertEqual(cli.main(["steward", "schedule", "uninstall", "--state", str(output_dir)]), 0)
                schedule_file = output_dir / "schedules" / "schedule.json"
                schedule_payload = json.loads(schedule_file.read_text(encoding="utf-8"))
                schedule_payload["installed"] = True
                schedule_file.write_text(json.dumps(schedule_payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
                self.assertEqual(cli.main(["steward", "schedule", "delete", "--state", str(output_dir)]), 1)
                self.assertEqual(
                    cli.main(["steward", "schedule", "delete", "--state", str(output_dir), "--force"]),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "schedule",
                            "create",
                            "--state",
                            str(output_dir),
                            "--scheduler",
                            "launchd",
                            "--operation",
                            "verify",
                            "--every-minutes",
                            "60",
                        ]
                    ),
                    0,
                )
                self.assertEqual(cli.main(["steward", "schedule", "delete", "--state", str(output_dir)]), 0)

                registry_bound_output_dir = Path(temp_dir) / "steward-registry-bound-schedule"
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "init",
                            "--inventory",
                            str(EXAMPLES / "workspace-inventory.example.json"),
                            "--policy",
                            str(EXAMPLES / "custody-policy.example.json"),
                            "--secret-bindings",
                            str(EXAMPLES / "secret-bindings.redacted.example.json"),
                            "--repository-bindings",
                            str(EXAMPLES / "repository-bindings.redacted.example.json"),
                            "--output",
                            str(registry_bound_output_dir),
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(
                        [
                            "steward",
                            "schedule",
                            "create",
                            "--state",
                            str(registry_bound_output_dir),
                            "--scheduler",
                            "launchd",
                            "--operation",
                            "run",
                            "--every-minutes",
                            "60",
                            "--registry-state",
                            str(registry_dir),
                            "--project-id",
                            "project/example",
                        ]
                    ),
                    0,
                )
                self.assertEqual(
                    cli.main(["steward", "schedule", "status", "--state", str(registry_bound_output_dir)]),
                    0,
                )
                self.assertEqual(
                    cli.main(["steward", "schedule", "install", "--state", str(registry_bound_output_dir)]),
                    1,
                )

            self.assertTrue((output_dir / "snapshot-plan.json").exists())
            self.assertTrue((output_dir / "resticprofile.yaml").exists())
            self.assertIn(
                "password-command",
                (output_dir / "resticprofile.yaml").read_text(encoding="utf-8"),
            )
            run_summaries = [
                path.read_text(encoding="utf-8")
                for path in sorted((output_dir / "run-summaries").glob("*.json"))
            ]
            self.assertTrue(
                any('"status": "delegated-rendered"' in summary for summary in run_summaries),
            )
            self.assertIn(
                '"operation": "verify"',
                stdout.getvalue(),
            )
            self.assertIn('"failure_stage": "authorization"', stdout.getvalue())
            self.assertIn('"schema_version": "northroot.steward.operation-recovery.v0"', stdout.getvalue())
            self.assertIn('"schema_version": "northroot.steward.legacy-run-import-result.v0"', stdout.getvalue())
            self.assertIn('"operation": "schedule.install"', stdout.getvalue())
            self.assertIn('"decision": "human-clearance-required"', stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
