# Northroot v0.1 Adoption Reports

This directory stores imported downstream adoption reports for the `v0.1.0` stable-kernel gate.

Northroot readiness reads these reports only. It does not import downstream source records, product state, or runtime data.

Each report must include:

```json
{
  "schema": "northroot.v0_1.adoption_report.v0",
  "repo": "northroot-agent",
  "check_name": "northroot-agent-work-ledger-dogfood",
  "journal_path": ".northroot/adoption/work-ledger.nrj",
  "event_count": 4,
  "kernel_valid_event_count": 4,
  "profile_valid_event_count": 4,
  "invalid_event_count": 0,
  "projection_rebuilt": true,
  "passed": true
}
```

The v0.1 release gate requires at least two passing downstream repository reports.
