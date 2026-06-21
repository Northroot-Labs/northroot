import unittest

from northroot.durability.naming import canonical_roots, normalize_root_name, root_purpose


class NamingTests(unittest.TestCase):
    def test_legacy_names_normalize_to_canonical_roots(self) -> None:
        self.assertEqual(normalize_root_name("northroot-restic"), "northroot-dr-restic")
        self.assertEqual(normalize_root_name("northroot-machine-node"), "northroot-machine-custody")
        self.assertEqual(normalize_root_name("offloads"), "northroot-offload-vault")

    def test_canonical_roots_explain_visibility(self) -> None:
        roots = {item["name"]: item for item in canonical_roots()}
        self.assertEqual(roots["northroot-durability"]["visibility"], "public-safe")
        self.assertTrue(roots["northroot-offload-vault"]["contains_payloads"])
        purpose = root_purpose("backup-receipts")
        self.assertIsNotNone(purpose)
        self.assertEqual(purpose.name, "northroot-receipts")


if __name__ == "__main__":
    unittest.main()
