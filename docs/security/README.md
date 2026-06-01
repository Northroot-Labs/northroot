# Security Notes

Northroot v0.1 has a small security surface: deterministic canonicalization,
content-derived event identity, `.nrj` journal framing, and offline
verification. It is not a network service, API deployment, container workload,
scheduler, runtime, or policy engine.

## v0.1 Threat Model

The v0.1 kernel assumes an adversary may provide malformed JSON, malformed
journal bytes, truncated frames, oversized payloads, or events with incorrect
`event_id` values. Kernel verification is responsible for rejecting corrupted
frames, recomputing event identity from canonical bytes, and preserving journal
readability boundaries.

The v0.1 kernel does not decide whether a profile event is semantically valid.
Schema conformance, policy admissibility, actor authority, work acceptance, and
projection correctness belong to profiles and consuming repositories.

## Hardening Expectations

- Keep verification offline and deterministic.
- Keep writes single-writer for v0.1.
- Preserve bounded frame validation in `northroot-journal`.
- Treat `list` as a dumb journal read, not a projection.
- Treat structural checkpoints as verified-prefix records, not semantic state.
- Keep deployment, secret, runtime, and API-service guidance outside this repo until a separate product surface exists.

## Related Documentation

- [Git Signing & Merge Policy](signing-policy.md) - Two-tier signing policy for audit-grade provenance.
- [v0.1 Stability Contract](../reference/v0.1-stability.md) - Stable kernel and incubating profile boundaries.
- [Segmented Journals](../reference/segmented-journals.md) - Structural segment and checkpoint contract.
