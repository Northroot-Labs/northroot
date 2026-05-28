# Incubating Substrate Policy and Reference Contracts v0

Status: Incubating
Owner: Northroot substrate maintainers

## Purpose

This note records the first narrow substrate extraction from NRX,
`northroot-runtime`, `northroot-agent`, and ClearlyOps convergence work. It
adds portable refs, policy envelopes, and lifecycle records as platform
contracts without turning Northroot into a scheduler, provider adapter, or
private policy authority.

## Boundary

Northroot may verify:

- schema shape;
- strict parse and canonicalization mechanics;
- content-derived hashes and identity;
- deterministic fixtures;
- receipt linkage;
- policy envelope and outcome record structure.

Northroot must not decide:

- who has Northroot Labs authority;
- whether a human approval is sufficient;
- which private policy bundle is legitimate;
- which credential scope is acceptable;
- how a provider resource is dereferenced or mutated;
- how a task is scheduled, retried, billed, or accepted.

Private policy authority remains in `northroot-foundation`. Product execution
and customer/domain semantics remain in downstream repos such as ClearlyOps.

## Contract Set

- `schemas/platform/v1/refs.schema.json`
  - `ActorRef`
  - `SourceRef`
  - `ArtifactRef`
  - `ReceiptRef`
  - `ContextBundleRef`
- `schemas/platform/v1/policy.schema.json`
  - `CapabilityGrantRef`
  - `PolicyOutcomeRef`
  - `PolicyEnvelope`
- `schemas/platform/v1/lifecycle.schema.json`
  - `RunLifecycleRecord`

## Consumer Vocabulary

Downstream repos should use the contract names above when recording extraction
or routing decisions. In particular:

- skill packages may declare expected substrate refs such as `SourceRef`,
  `ReceiptRef`, `CapabilityGrantRef`, `PolicyOutcomeRef`, and
  `RunLifecycleRecord`;
- `northroot-agent` may map deployed broker and queue records to these refs,
  but queueing, dispatch, worker leases, provider calls, and acceptance remain
  local;
- `northroot-foundation` may define authority doctrine for capabilities and
  policy bundles, but Northroot only verifies the portable envelope shape and
  receipt linkage.

## Non-Goals

- No queue, scheduler, worker runtime, hosted service, or provider SDK.
- No private Northroot Labs policy authority.
- No tenant, workspace, APD, CropTrak, customer, billing, or product
  projection semantics.
- No live credential, provider account, or approval workflow implementation.

## Promotion Gate

These contracts can leave incubation only after:

- at least two downstream lanes use the same refs without importing product
  semantics;
- schema validation and deterministic fixtures pass in Northroot;
- downstream policy bundles remain external inputs rather than Northroot-owned
  authority;
- the Northroot README clearly distinguishes substrate verification from
  organizational authorization.
