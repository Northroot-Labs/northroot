# northroot-durability

`northroot-durability` is a legacy compatibility package for public/private
artifact boundary checks and simple copy manifests.

It is not the Northroot backup, restore, scheduling, or disaster-recovery
surface. New custody work belongs in `northroot-custody` and should be exposed
to operators and agents through `nr steward`.

It does not contain machine custody state, backup receipts, real local paths,
raw Cursor/Codex state, or operational data. Those belong in private deployment
state such as the governed Northroot-Labs refinery or private custody storage.

```python
from northroot.durability.policy import public_commit_decision
from northroot.durability.manifest import build_tree_manifest
```

```bash
northroot-durability roots
northroot-durability modes
northroot-durability public-check artifacts.json
```

Use this package only for compatibility with existing public/private boundary
checks. Do not add new snapshot, restore, retention, scheduler, or secret
provider vocabulary here.
