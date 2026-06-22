# Stewardship Workstream

Status: design context for the next custody/steward change set.

## Problem Statement

Git repository visibility is too coarse for durable machine stewardship. A
Northroot steward needs to know custody state at the object level: which
repositories, databases, journals, artifact directories, generated state,
caches, secret files, and environment files exist; where each physically lives;
what may appear in public summaries; and what restore guarantee is expected.

The current `northroot-custody` package already separates public-safe inventory,
policy, private bindings, delegated plans, scheduled operations, run summaries,
and retention evidence. The next design layer should make object custody an
explicit model inside that stewardship surface rather than treating a whole Git
checkout, state directory, or backup profile as one undifferentiated unit.

This remains a delegated model. Northroot should describe, validate, plan,
summarize, and verify custody. It should not become a backup engine, scheduler,
secret manager, storage transport, database adapter, or monitoring system.

## Object Custody State

The object-level model should be small and boring:

| Field | Meaning |
| --- | --- |
| `object_id` | Stable symbolic object identifier used in public-safe documents. |
| `object_type` | Kind of object, such as `repo`, `sqlite`, `postgres`, `secret-file`, `env-file`, `artifact-dir`, `journal`, `cache`, or `generated-state`. |
| `visibility` | Disclosure class: `public`, `private`, `secret`, `regulated`, or `ephemeral`. |
| `storage_binding` | Symbolic reference to the physical location or provider binding. Real paths, repository URLs, and provider references stay in private binding documents. |
| `custody_policy` | Backup, retention, verification, and restore obligations for this object. |
| `redaction_policy` | What may appear in public summaries, reports, evidence records, and agent-facing capability manifests. |
| `restore_class` | Expected recovery mode: `full-restore`, `metadata-only`, `rehydrate-from-provider`, or `never-export`. |

The model should preserve the difference between state that must be restored,
state that can be rehydrated, state that is intentionally ignored, and state
that must never be exported. That distinction is the practical reason this
layer exists.

## Service Registry

Object custody is still too small to run a durable stewardship service by
itself. The service layer needs a registry that binds a Northroot node to its
projects, objects, destinations, replicas, permissions, legacy imports, and
failure-mode policy.

The public-safe registry contract is `northroot.steward.service-registry.v0`.
It should answer, without exposing private paths:

- which node owns the stewardship service;
- which projects are registered under that node;
- which objects belong to each project;
- which project-level and object-level operations are allowed, blocked, or
  require human clearance;
- which destinations are primary stores, replicas, source bindings, or receipt
  logs;
- which replica checks require external delegated evidence;
- how interrupted runs, disconnected storage, and power loss are resumed or
  held safely;
- which legacy machine durability profile inputs are being migrated through
  symbolic refs.

This registry is not a scheduler or transport. It is the durable control plane
contract that lets steward prove whether the service is configured coherently
before delegated tools are allowed to mutate backup repositories, schedules, or
restore targets.

The current `steward registry` implementation manages this document as durable
state. It initializes a registry from a validated JSON document, reports counts
and validation findings, appends projects and their permission sets atomically,
appends objects, destinations, source bindings, replicas, and legacy import
records, and records operation summaries for each mutation. Registry writes are
atomic and guarded by a single operation lock. If a process dies or the machine
goes offline while the lock exists, later mutations fail closed until
`steward registry recover` validates the current registry and records the
interruption.

The registry also exposes `steward registry authorize` as the first runtime
permission gate. It evaluates an operation against the registered project
permission set and any object permission set for the requested object. The
decision is deterministic and fail-closed: unresolved registry locks, invalid
registries, unknown objects, blocked operations, human-clearance requirements,
and missing allow rules are all non-allowed outcomes.

The delegated execution commands can now consume that gate directly. When
`steward run`, `verify`, `restore`, or `restore-drill` are invoked with a
registry state, project id, and optional object id, authorization runs before
preflight and before any external delegated tool is called. Denials are recorded
as run summaries rather than disappearing as local control-flow.

Delegated execution has its own operation lock in the steward state directory.
If a machine stops mid-run, later executions fail closed until
`steward recover-operation` records the interrupted lock as a run summary and
clears it for a deliberate retry.

The registry still does not execute replica sync, inspect private LaunchAgent
state, or import raw legacy run directories by itself. Private adapters should
first extract those inputs into a sanitized
`northroot.steward.legacy-profile-import.v0` bundle, then apply it through
`steward registry import-legacy-profile` as one atomic registry mutation.
Raw per-run evidence import remains a later adapter layer over the now-durable
registry state.

## Legacy Import Context

The private Northroot-Labs environment currently has a legacy hourly machine
durability profile. It is useful evidence for the next implementation, but its
raw paths, launchd labels, logs, external volume names, and backup receipt
locations are private deployment state and must not be copied into public
fixtures or examples.

The import/refactor should start from the private scheduler registration and
ingest the associated private machine profile:

- scheduler registration and runner command;
- machine node document;
- project node document;
- current runner state;
- per-run result directories;
- primary repository binding;
- backup receipt directory;
- stdout/stderr log bindings.

The public output of that import should be symbolic and sanitized:

- object custody inventory with `object_id`, `object_type`, `visibility`, and
  `restore_class`;
- private storage bindings that map object IDs to real machine paths and
  provider locations outside the public repo;
- custody policy that states retention, verification, redaction, and restore
  expectations;
- a steward state directory that can be checked with `steward preflight`,
  reported with `steward report`, and scheduled through `steward schedule`;
- run summaries that say what happened without exposing raw secret material,
  private paths, or live machine custody data.

## Change Set Goal

The active change set is converting the legacy hourly durability profile into
the cleaner steward model. The public repo owns the sanitized import contract
and registry mutation; private adapters own extracting raw local files into that
contract. The workstream still needs to answer these questions with code, tests,
and sanitized examples:

1. How does a private legacy profile become a public-safe object custody
   inventory plus private bindings?
2. How are object visibility, redaction, and restore class validated before a
   steward plan is rendered?
3. Which objects are backed up, which are metadata-only, which are rehydrated
   from providers, and which are excluded or never exported?
4. How does a scheduled steward run prove readiness without depending on an
   interactive shell, raw LaunchAgent knowledge, or Git-level visibility?
5. What evidence from old per-run directories can be imported as historical
   run summaries, and what evidence must be regenerated under the new steward
   contract?

## Non-Goals

- Do not import real machine paths, external drive names, secrets, LaunchAgent
  labels, raw logs, or backup receipts into this public repository.
- Do not add a custom backup engine or scheduler.
- Do not make kernel crates understand custody semantics.
- Do not collapse object visibility into Git visibility or repository privacy.
- Do not treat a successful hourly command as proof of restore readiness unless
  the required verification and restore evidence exists.

## Implementation Boundary

Public code should live in `packages/northroot-custody` and the `nr steward`
wrapper. Private imports may live in the Northroot-Labs deployment repository
or another private state root, then feed sanitized documents into this package.

The stable kernel remains unchanged. Object custody is a promoted package and
CLI concern above records, node manifests, and state/eval primitives.
