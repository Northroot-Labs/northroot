# Northroot Capability Index

This directory contains public-safe seed capability entries. Private projects
may copy or extend this index with local install refs, backup bindings, and
deployment notes, but those private files should stay outside git or use the
ignored `capabilities/private*.json` pattern.

The index answers a narrow question: what reusable Northroot capability exists,
how can another project absorb it, what does it expose, and what verification
proves it is usable?

It is not a node registry, custody registry, work queue, merge queue, or
governance authority.

Validate the checked-in index with:

```bash
python3 scripts/validate_capability_index.py capabilities/index.public.json
```

For a downstream project that wants stewardship backups, consume
`northroot-custody` from the `northroot.custody` entry and keep project-specific
inventory, policy, secret bindings, repository bindings, and schedule state in
that downstream project or private machine state.
