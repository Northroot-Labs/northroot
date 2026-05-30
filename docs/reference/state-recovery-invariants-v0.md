# State Recovery Invariants v0

Status: Binding for Northroot substrate changes

This document narrows the state-recovery contract for `northroot`. It keeps
event logs, projections, snapshots, and backups compatible without turning the
trust kernel into an orchestration runtime.

## Invariants

SR-1: `.nrj` is the canonical append-only event log format.

SR-2: Event identity is derived from canonical JSON and a domain-separated
hash. Storage adapters may change, but event identity must not.

SR-3: Projections are derived read models. They are rebuildable and are never
the source of truth.

SR-4: Snapshots are content-addressed recovery artifacts. They accelerate
startup and replay from a covered journal position.

SR-5: Snapshot projection and evidence-index sections are accelerators only.
They are not authoritative and must be rebuildable from events.

SR-6: Backups are disaster-recovery byte stores. Restore must verify journals,
verify snapshots, replay the journal tail, and rebuild projections.

SR-7: `northroot` may record, verify, replay, derive, and restore deterministic
state. It must not schedule, dispatch, lease, approve, execute, or decide
product work.

SR-8: Orchestration belongs outside `northroot`. Queueing, leases, workers,
capability brokering, provider adapters, tenant workflows, and product UI must
compose over the substrate through explicit contracts.

## Placement

`northroot` owns deterministic substrate primitives and proof tooling.

`northroot-agent` owns control-plane orchestration, queueing, dispatch, leases,
worker contracts, and capability brokering.

ClearlyOps owns product workflows, tenant/customer evidence, operator UX, and
typed application surfaces.

`northroot-foundation` owns doctrine and placement policy that can reference
these invariants without absorbing runtime code.
