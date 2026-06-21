import unittest

from northroot.durability.policy import BackupMode, classify_artifact, mode_policy, public_commit_decision


class PolicyTests(unittest.TestCase):
    def test_source_policy_can_be_public_safe(self) -> None:
        decision = public_commit_decision("packages/northroot-durability/src/northroot/durability/policy.py", artifact_kind="source")
        self.assertTrue(decision.allowed)
        self.assertEqual(decision.visibility, "public")

    def test_machine_state_is_private_even_when_json(self) -> None:
        decision = public_commit_decision(
            ".northroot/state/machine-durability/20260101T000000Z/run-result.json",
            artifact_kind="offload",
            contains_real_paths=True,
            contains_backup_receipts=True,
        )
        self.assertFalse(decision.allowed)
        self.assertEqual(decision.visibility, "private-or-internal")
        self.assertIn("contains_backup_receipts", decision.reasons)

    def test_redacted_example_can_clear_path_identity_flags_only(self) -> None:
        decision = public_commit_decision(
            "examples/machine-custody.redacted.json",
            artifact_kind="redacted-example",
            is_redacted_example=True,
            contains_real_paths=True,
            contains_host_identity=True,
        )
        self.assertTrue(decision.allowed)

    def test_live_operational_state_never_public(self) -> None:
        decision = public_commit_decision(
            "state/live_ops.sqlite",
            artifact_kind="payload",
            contains_live_operational_state=True,
        )
        self.assertFalse(decision.allowed)
        self.assertIn("contains_live_operational_state", decision.reasons)

    def test_tiered_modes_require_restore_evidence_before_prune(self) -> None:
        policy = mode_policy(BackupMode.PRUNE_AFTER_RESTORE_PROOF)
        self.assertTrue(policy.local_prune_allowed)
        self.assertIn("restore-script", policy.required_evidence)
        self.assertIn("human-review-ack", policy.required_evidence)

    def test_classification_routes_cursor_extracts_to_knowledge_mode(self) -> None:
        classified = classify_artifact("knowledge/cursor-summary.json", "cursor-extract")
        self.assertEqual(classified.backup_mode, BackupMode.KNOWLEDGE_EXTRACT)
        self.assertFalse(classified.public_safe_candidate)


if __name__ == "__main__":
    unittest.main()
