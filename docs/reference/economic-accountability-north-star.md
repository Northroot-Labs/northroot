# Economic Accountability North Star

Status: north star
Audience: Northroot maintainers, reviewers, downstream profile authors
Scope: economic activity as a verifiable state-transition capability profile

Northroot exists to make state transitions verifiable. Governed economic
activity is one high-value capability profile over that substrate.

The kernel must remain small. It does not become a product, workflow engine,
agent runtime, bookkeeping app, dashboard, or cloud platform. It provides the
durable building blocks needed for verifiable transition infrastructure:

- canonical identity
- append-only evidence journals
- replay and offline verification
- deterministic projection inputs
- portable attestations

Economic profiles add the primitives needed for governed financial automation:

- persistent economic actor identity
- delegated authority
- budget and scope bounds
- deterministic projections
- policy/evaluation primitives
- hard accounting invariants
- business receipts

Bookkeeping is one capability set over this governed core. It is not the core
itself.

## Verifiable State Transitions

The system-level primitive is a verifiable state transition.

A transition claim should be reconstructable from:

```text
prior state reference
  + ordered event / evidence prefix
  + projection rule
  + predicate or invariant evaluation
  + resulting state reference
  + attestation
```

The kernel does not decide whether a transition is semantically valid. It
verifies parse-safe canonical events, content-derived identity, journal
membership, ordering, and replay inputs. State/eval layers verify projected
claims over event prefixes. Domain packs define what a transition means.
Authority profiles decide whether a transition may be admitted.

An attestation is the system-level statement that a transition, evaluation, or
evidence relation was observed and can be replayed or verified. A receipt is a
business-facing attestation: useful for economic actions, approvals, execution,
payments, reconciliation, work acceptance, or customer/vendor records.

Relays exchange portable attestations and business receipts. They must not
become the source of truth for state; the receiving node verifies the transition
claim against its trusted identity, authority, evidence, journal, and projection
rules.

## Thesis

An automated actor may perform economic work only when all of the following can
be verified:

1. The actor is registered.
2. The actor's credential binding is valid for the action context.
3. The actor has a valid delegation chain from an authority-bearing actor.
4. The proposed action is within delegated scope.
5. The proposed action is within budget and risk bounds.
6. Required evidence is present and content-addressed.
7. Financial mutations preserve hard accounting invariants.
8. Execution emits an attestation or business receipt that can be replayed,
   verified, and linked back to identity, authority, evidence, and journal
   position.

This makes agents persistent economic actors without making Northroot an agent
framework.

## Reproducible Node

The open source unit is a reproducible Northroot node.

A node is a persistent actor with local operational state, stable identity,
append-only evidence journals, and the ability to exchange verifiable
attestations and business receipts with other actors. It should be reproducible
from source, configuration, domain packs, journals, attestations, and receipts
rather than dependent on a central SaaS database.

This inverts the traditional centralized SaaS model:

```text
traditional SaaS:
  vendor database -> product workflow -> customer-facing records

Northroot node model:
  user-controlled node -> portable attestations / receipts -> relay exchange
```

Relays move portable attestations and business receipts between nodes. They are
transport and discovery infrastructure, not authority sources. A relay may
queue, route, or fan out records, but the receiving node must be able to verify
identity, authority, evidence, journal position, transition claims, and
attestation integrity without trusting the relay as the system of record.

This does not mean "no cloud." It means cloud services are optional coordination
and operations layers rather than the primary custody model for customer data.
A centralized service may eventually provide:

- node registry and discovery;
- billing and licensing;
- managed sync;
- managed backup;
- hosted authentication or credential recovery;
- relay operations;
- update distribution;
- support diagnostics over user-approved artifacts.

Those services must not be required to reconstruct the node's authoritative
state. The scale boundary is: cloud can make nodes easier to find, fund, sync,
backup, authenticate, and operate, but the node remains the durable actor and
portable attestations and receipts remain the exchange format. Build the
single-node system end-to-end before promoting any centralized service to
product infrastructure.

The node may use SQLite as its operational database. SQLite is the default fit
for durable local state, queues, projections, attestation indexes, receipt
indexes, and sync cursors. DuckDB may be added for analytics when columnar scans
or larger analytical workloads justify it, but analytics must remain a derived
view over journals, attestations, receipts, and projections.

The product is the node plus domain packs:

```text
Northroot product =
  reproducible node
  + domain pack
  + authority/evaluation profile
  + attestation/receipt exchange
```

Domain packs define product-specific schemas, predicates, projections,
accounting rules, UX, and adapters. They must not become hidden central policy
or a requirement to trust a hosted database.

## System Layers

Build the system as small composable layers. Each layer should earn its place by
enforcing a primitive that later layers cannot safely fake.

```text
northroot-canonical
  deterministic bytes
  digests
  content identity

northroot-journal
  append-only ordered events
  stream sequence
  replay
  verification

northroot-identity
  persistent actor ids
  credential bindings
  human / agent / service / organization actor classes
  identity lifecycle events

northroot-authority
  grants
  delegations
  scopes
  revocations
  expiry
  delegation-chain verification

northroot-attestations
  transition attestations
  evaluation attestations
  evidence attestations
  authority-chain references

northroot-receipts
  business-facing attestations
  action receipts
  approval receipts
  execution receipts

northroot-accounting
  accounts
  postings
  balanced transactions
  reconciliation relations
  balance projections

northroot-eval
  predicates
  policy composition
  Satisfied / Unsatisfied / Indeterminate
  EvaluationDelta

northroot-node
  reproducible deployment unit
  local operational state
  attestation / receipt exchange
  relay adapters
  domain-pack loading

northroot-conformance
  replay tests
  journal adapter tests
  node reproducibility tests
  attestation / receipt exchange tests
  identity tests
  authority-chain tests
  accounting-balance tests
```

The order matters. Identity and authority should not be built before journal
ordering and canonical identity are boring. Financial invariants should not be
added before actor identity, authority, attestations, and receipts have a stable
shape.

## Authority Boundary

Authority is not a profile label, runtime config value, or provider metadata.
It is a governed state machine over registered actors, credentials, grants,
delegations, scopes, budgets, revocations, attestations, and receipts.

The authority layer has gateway-like properties:

- It evaluates proposed economic actions before execution.
- It fails closed on unknown actors, missing grants, expired delegations, missing
  evidence, insufficient budget, or out-of-scope actions.
- It emits verifiable decisions and attestations; economic profiles may render
  those as business receipts.
- It never treats execution as authority.
- It never treats provider identity as authority.
- It never lets an agent borrow a human identity without an explicit delegation
  record.

This boundary is correctness-sensitive. Rust is justified here because authority
checks must be typed, deterministic, auditable, dependency-light, and hard to
bypass by accident.

## Accounting Invariants

Northroot should support financial correctness without owning business-specific
bookkeeping semantics.

The core accounting primitive is a balanced transaction:

```text
sum(debits) = sum(credits)
```

Northroot may define neutral accounting shapes:

- account id
- ledger id
- posting
- transaction
- balance projection
- reconciliation relation
- evidence reference

Northroot must not define business categories, vendor workflows, tax treatment,
chart-of-accounts templates, invoice UX, or bank integration behavior as core
semantics.

## Agent Profiles

An agent profile is a projection, not an authority source.

```text
AgentProfile =
  ActorIdentity
  + CredentialBindings
  + DelegatedAuthority
  + BudgetState
  + AttestationHistory
  + ReceiptHistory
  + EvaluationState
```

Profiles help downstream systems reason about persistent economic actors. They
must be rebuildable from journaled identity, authority, budget, action, and
attestation or receipt events.

## Event Families

The following are candidate event families for governed economic-action layers.
They are not all required in the stable kernel at once.

```text
ActorRegistered
CredentialBound
CredentialRotated
AuthorityDelegated
AuthorityRevoked
BudgetAllocated
BudgetAdjusted
ActionProposed
ActionEvaluated
ActionApproved
ActionRejected
ActionExecuted
ReceiptRecorded
AttestationRecorded
TransactionProposed
TransactionPosted
ReconciliationCompleted
```

Each family should be introduced only with conformance fixtures and replay
tests.

## First Useful Milestone

The first meaningful governed economic-action milestone is:

Given:

- a registered actor
- a valid credential binding
- a delegated authority grant
- a budget
- source evidence
- a proposed balanced transaction

Northroot can verify:

- the actor is known
- the credential binding is current
- the delegation chain is valid
- the action is in scope
- the budget is sufficient
- the transaction balances
- the attestation or business receipt links to evidence
- replay produces the same financial state

This is enough to support aggressive bookkeeping automation later without
baking bookkeeping product semantics into the kernel.

## Non-Goals

Northroot does not own:

- agent runtimes
- cloud runtimes
- work queues
- dashboards
- workflow engines
- bank connectors
- QuickBooks or accounting-system adapters
- hosted SaaS databases as the system of record
- relay authority over receipt truth
- document parsing
- entity resolution
- reconciliation UX
- business policy authorship
- tax categorization
- vendor-specific bookkeeping flows

Those are capabilities, profiles, adapters, or products over the governed core.

## Design Rule

Before adding a concept to Northroot, ask:

1. Is it needed to make economic action accountable?
2. Is it a primitive, or can it be derived as a projection?
3. Is it deterministic and replayable?
4. Can it be verified without a vendor, cloud, or agent runtime?
5. Does it enforce a hard invariant that downstream code cannot safely fake?
6. Can it be expressed as identity, authority, journal, receipt, projection,
   attestation, predicate, evaluation result, accounting invariant,
   reproducible node behavior, or domain-pack contract?

If not, keep it outside the kernel.
