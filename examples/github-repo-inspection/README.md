# GitHub Repo Inspection Example

This example shows the first Northroot proof shape using common formats:

- JSON for actor, obligation, policy, receipt, and manifest objects
- JSONL for append-only execution events
- `sha256:<hex>` hashes over canonical JSON/JSONL evidence

`.nrj` is intentionally not required for this demo. It remains an optional
export container only when binary framing, streaming verification, truncation
handling, and cross-language offline audit are worth the extra format.

## Run

```bash
cargo run -p northroot -- validate examples/github-repo-inspection/actor.json
cargo run -p northroot -- validate examples/github-repo-inspection/obligation.json
cargo run -p northroot -- validate examples/github-repo-inspection/policy.json
cargo run -p northroot -- hash examples/github-repo-inspection/events.jsonl
cargo run -p northroot -- verify examples/github-repo-inspection/receipt.json
```
