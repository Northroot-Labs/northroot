# northroot-durability

`northroot-durability` is the importable Northroot package for public-safe durability policy, naming, manifest, and public/private commit checks.

It does not contain machine custody state, backup receipts, real local paths, raw Cursor/Codex state, or operational data. Those belong in the governed Northroot-Labs refinery or private custody storage.

```python
from northroot.durability.policy import public_commit_decision
from northroot.durability.manifest import build_tree_manifest
```

```bash
northroot-durability roots
northroot-durability modes
northroot-durability public-check artifacts.json
```
