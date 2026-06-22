import json
import tomllib
import unittest
from pathlib import Path

from northroot.custody import model


ROOT = Path(__file__).resolve().parents[1]


class PackageMetadataTests(unittest.TestCase):
    def test_package_declares_installed_cli_entrypoint(self) -> None:
        pyproject = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
        self.assertEqual(pyproject["project"]["name"], "northroot-custody")
        self.assertEqual(
            pyproject["project"]["scripts"]["nr-custody"],
            "northroot.custody.cli:main",
        )
        self.assertEqual(pyproject["tool"]["setuptools"]["packages"]["find"]["where"], ["src"])

    def test_public_examples_cover_the_reusable_vocabulary(self) -> None:
        expected_schemas = {
            model.AGENT_DELEGATION_POLICY_SCHEMA,
            model.INVENTORY_SCHEMA,
            model.POLICY_SCHEMA,
            model.SNAPSHOT_PLAN_SCHEMA,
            model.VERIFICATION_RESULT_SCHEMA,
            model.RETENTION_DECISION_SCHEMA,
            model.RUN_SUMMARY_SCHEMA,
            model.SERVICE_REGISTRY_SCHEMA,
            model.LEGACY_PROFILE_IMPORT_SCHEMA,
            model.LEGACY_RUN_IMPORT_SCHEMA,
        }
        examples = [
            ROOT / "examples" / "workspace-inventory.example.json",
            ROOT / "examples" / "custody-policy.example.json",
            ROOT / "examples" / "snapshot-plan.example.json",
            ROOT / "examples" / "verification-result.example.json",
            ROOT / "examples" / "retention-decision.example.json",
            ROOT / "examples" / "run-summary.example.json",
            ROOT / "examples" / "service-registry.example.json",
            ROOT / "examples" / "legacy-profile-import.redacted.example.json",
            ROOT / "examples" / "legacy-run-import.redacted.example.json",
            ROOT / "examples" / "agent-delegation-policy.dogfood.example.json",
        ]

        observed_schemas = set()
        for path in examples:
            payload = json.loads(path.read_text(encoding="utf-8"))
            observed_schemas.add(payload["schema_version"])
            self.assertEqual(model.validate_document(payload, public_safe=True), [], path.name)

        self.assertEqual(observed_schemas, expected_schemas)

    def test_public_examples_cover_agent_contracts(self) -> None:
        command_plan = json.loads((ROOT / "examples" / "command-plan.example.json").read_text(encoding="utf-8"))

        self.assertEqual(command_plan["schema_version"], model.COMMAND_PLAN_SCHEMA)
        self.assertEqual(model.validate_document(command_plan, public_safe=True), [])


if __name__ == "__main__":
    unittest.main()
