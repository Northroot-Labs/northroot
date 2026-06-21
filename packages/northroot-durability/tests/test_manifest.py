import json
import tempfile
import unittest
from pathlib import Path

from northroot.durability.manifest import build_tree_manifest, load_tree_manifest, verify_tree_manifest


class ManifestTests(unittest.TestCase):
    def test_manifest_verifies_matching_copy(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            copied = Path(tmp) / "copy"
            source.mkdir()
            copied.mkdir()
            (source / "a.txt").write_text("alpha", encoding="utf-8")
            (copied / "a.txt").write_text("alpha", encoding="utf-8")

            manifest = build_tree_manifest(source)
            self.assertEqual(verify_tree_manifest(manifest, copied), [])

    def test_manifest_detects_digest_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            copied = Path(tmp) / "copy"
            source.mkdir()
            copied.mkdir()
            (source / "a.txt").write_text("alpha", encoding="utf-8")
            (copied / "a.txt").write_text("beta", encoding="utf-8")

            manifest = build_tree_manifest(source)
            errors = verify_tree_manifest(manifest, copied)

        self.assertTrue(any(error.startswith("size-mismatch") for error in errors))

    def test_manifest_load_rejects_wrong_schema(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.json"
            path.write_text(json.dumps({"schema_version": "other"}), encoding="utf-8")
            with self.assertRaises(ValueError):
                load_tree_manifest(path)


if __name__ == "__main__":
    unittest.main()
