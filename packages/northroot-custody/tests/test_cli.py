import contextlib
import io
import json
import os
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from northroot.custody import cli


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"


def write_fake_executable(directory: Path, name: str, body: str = "#!/bin/sh\nexit 0\n") -> None:
    path = directory / name
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)


class CliTests(unittest.TestCase):
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

            self.assertTrue((output_dir / "snapshot-plan.json").exists())
            self.assertTrue((output_dir / "resticprofile.yaml").exists())
            self.assertIn(
                "password-command",
                (output_dir / "resticprofile.yaml").read_text(encoding="utf-8"),
            )
            latest_summary = sorted((output_dir / "run-summaries").glob("*.json"))[-1]
            self.assertIn(
                '"status": "delegated-rendered"',
                latest_summary.read_text(encoding="utf-8"),
            )
            self.assertIn(
                '"operation": "verify"',
                stdout.getvalue(),
            )


if __name__ == "__main__":
    unittest.main()
