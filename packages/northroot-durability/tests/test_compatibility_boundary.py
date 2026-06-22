from pathlib import Path
import tomllib
import unittest


PACKAGE_ROOT = Path(__file__).resolve().parents[1]


class CompatibilityBoundaryTests(unittest.TestCase):
    def test_durability_package_points_new_custody_work_to_custody(self) -> None:
        readme = (PACKAGE_ROOT / "README.md").read_text(encoding="utf-8")
        pyproject = tomllib.loads((PACKAGE_ROOT / "pyproject.toml").read_text(encoding="utf-8"))

        self.assertIn("legacy compatibility package", readme)
        compact_readme = " ".join(readme.split())

        self.assertIn("not the Northroot backup, restore, scheduling, or disaster-recovery surface", compact_readme)
        self.assertIn("northroot-custody", readme)
        self.assertIn("nr steward", readme)
        self.assertIn("Legacy Northroot public/private boundary helpers", pyproject["project"]["description"])


if __name__ == "__main__":
    unittest.main()
