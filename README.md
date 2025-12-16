Overview

Northroot is a neutral, open foundation for verifiable, policy-bound execution.

It defines canonical event formats, deterministic hashing rules, and verification primitives that allow organizations to prove what was authorized, what executed, and under what bounds — independently of models, tools, or agent frameworks.

Northroot is designed for systems where AI actions have real consequences.

⸻

Problem Northroot addresses

Modern AI systems increasingly:
	•	invoke tools
	•	access sensitive resources
	•	incur costs
	•	trigger side effects

Most systems rely on best-effort logging after the fact.

That is insufficient when:
	•	failures are expensive or irreversible
	•	audits are required
	•	trust boundaries matter
	•	automation outpaces human review

Northroot moves accountability to the execution boundary, using enforceable policy and verifiable evidence.

⸻

Who Northroot is for

Northroot is intended for teams that:
	•	Build or operate autonomous or semi-autonomous systems
	•	Need provable accountability, not heuristics
	•	Run in regulated, high-risk, or financially sensitive environments
	•	Require deployment inside their own infrastructure
	•	Want primitives they can integrate without vendor lock-in

Typical roles:
	•	infrastructure engineers
	•	platform / AI systems engineers
	•	security and governance teams
	•	FinOps and compliance stakeholders

⸻

Core concepts (aligned with the spec)

Northroot is built around verifiable events:
	•	Events are canonical JSON objects
	•	Canonical bytes are deterministically hashed
	•	Event identity is the hash
	•	Events are append-only
	•	Verification is offline and replayable

Core event types include:
	•	Authorization events — what was allowed, under what policy and bounds
	•	Execution events — what actually occurred, including metered cost
	•	Checkpoint events — integrity anchors over event sequences
	•	Attestation events — signed statements over checkpoints

Operational metadata (traces, request IDs, retries) is explicitly out of band.

⸻

Core capabilities
	•	Deterministic canonicalization and hashing
	•	Append-only, verifiable event logs
	•	Explicit authorization → execution linkage
	•	Policy-defined bounds on execution
	•	Multi-dimensional cost metering (vector model)
	•	Model-, tool-, and framework-agnostic design
	•	Offline verification and replay
	•	Open-source gateway deployable inside your infrastructure
	•	CLI tooling to inspect and verify events

⸻

What Northroot is
	•	A neutral verifiability and governance substrate
	•	A minimal protocol for evidence, identity, and enforcement
	•	A reference implementation, not a platform mandate
	•	A foundation for accountable AI execution across ecosystems

⸻

What Northroot is not
	•	An AI agent framework
	•	A workflow orchestrator
	•	A model hosting platform
	•	A logging or observability tool
	•	A policy language or OPA replacement
	•	A mass-market AI gateway

Northroot does not interpret intent, make decisions, or manage agents.
It records, verifies, and enforces bounds around execution.

⸻

Design principles
	•	Neutrality — no vendor, model, or framework lock-in
	•	Determinism — same input, same bytes, same hash
	•	Minimalism — only what must be proven belongs in the core
	•	Separation of concerns — protocol ≠ implementation
	•	Auditability first — evidence over convenience

⸻

Roadmap

See ROADMAP.md for planned milestones, extensions, and non-goals.

⸻

Why this matters

Northroot treats AI execution as something that must eventually meet the same standards as:
	•	financial systems
	•	infrastructure control planes
	•	safety-critical automation

The goal is not control for its own sake —
it is making autonomy survivable.