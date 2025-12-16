Northroot Core – Project Constitution

Version: 1 (Foundational)
Status: Binding

⸻

0. Purpose

This project exists to build neutral, verifiable infrastructure for automated systems.

Specifically, it provides:
	•	Deterministic schemas
	•	Evidence and receipt generation
	•	Policy-bound authorization and execution verification
	•	Offline, replayable verification

It does not exist to:
	•	Make decisions
	•	Optimize outcomes
	•	Replace judgment
	•	Automate execution
	•	Persuade users

If a feature proposal blurs this boundary, it is out of scope.

⸻

1. Core Principle: Neutrality

The system must remain neutral with respect to outcomes.

We commit to:
	•	Proving what was allowed
	•	Proving what happened
	•	Proving under which rules and inputs

We explicitly refuse to:
	•	Decide what is correct
	•	Decide what is optimal
	•	Decide what should have happened
	•	Judge semantic correctness of outputs

Neutrality is not optional.
If neutrality is compromised, trust is lost permanently.

⸻

2. Separation of Responsibility

This project:
	•	Verifies
	•	Attests
	•	Binds
	•	Records

This project does NOT:
	•	Execute actions
	•	Orchestrate workflows
	•	Plan agent behavior
	•	Call external tools on behalf of users
	•	Modify external state

Rule:

We never audit ourselves.

Execution always lives outside the core.

⸻

3. Determinism and Replayability

All core logic must satisfy:
	•	Deterministic inputs → deterministic outputs
	•	Canonical serialization
	•	Stable hashing
	•	Offline verification without network access
	•	No dependence on wall-clock nondeterminism

If a component cannot be replayed and verified offline, it does not belong in the core.

⸻

4. Receipts Are the Primary Artifact

Receipts are:
	•	First-class objects
	•	Immutable once emitted
	•	Append-only
	•	Content-addressed or hash-bound
	•	Sufficient for audit and dispute resolution

Logs, metrics, and telemetry are not substitutes for receipts.

The system may ingest logs, but it only commits receipts.

⸻

5. Policy as Law (Not Product)

Policy evaluation is treated as law, not business logic.

Therefore:
	•	Policy semantics must be inspectable
	•	Policy evaluation must be deterministic
	•	Policy versions are referenced by hash
	•	Policy engines are replaceable

We explicitly prohibit:
	•	Proprietary policy languages
	•	Paywalled policy semantics
	•	Hidden or heuristic policy behavior

We may build tools around policy, but we do not own the law.

⸻

6. Canonicalization Is Mechanical, Not Semantic

Canonicalization exists to ensure:
	•	Stable representations
	•	Consistent hashing
	•	Tamper-evidence

Canonicalization does not:
	•	Infer meaning
	•	Correct data
	•	Resolve disputes
	•	Impose domain semantics

If a transformation changes meaning, it is not canonicalization and does not belong in the core.

⸻

7. No Persuasion, No Optimization, No “Intelligence”

The core must never:
	•	Recommend actions
	•	Rank options
	•	Nudge behavior
	•	Optimize for cost, speed, or quality
	•	Provide “smart” defaults that influence decisions

Those are downstream concerns.

Boring correctness beats clever behavior.

⸻

8. Vendor and Framework Independence

The core must remain:
	•	Provider-agnostic
	•	Framework-agnostic
	•	Deployment-agnostic

Adapters may exist.
Bindings may exist.
The core must not depend on:
	•	Any single AI provider
	•	Any agent framework
	•	Any cloud vendor
	•	Any orchestration system

Vendor capture is a violation of neutrality.

⸻

9. Open Core Boundaries

The following must remain open and inspectable:
	•	Receipt schemas
	•	Canonicalization rules
	•	Hashing rules
	•	Verification logic
	•	Authorization → execution lifecycle

Commercialization is allowed only in:
	•	Hosting
	•	Managed services
	•	UX
	•	Integrations
	•	Vertical packaging

Truth itself is never monetized.

⸻

10. Explicit Non-Goals (Never Add)

The following are permanently out of scope:

❌ Agent planners
❌ Workflow engines
❌ Task schedulers
❌ AI copilots
❌ Decision recommenders
❌ Model evaluation or ranking
❌ Semantic correctness scoring
❌ Engagement optimization
❌ Autonomous execution
❌ Proprietary “magic” logic

If a proposal includes any of these, it must be rejected.

⸻

11. Integrity Over Growth

When forced to choose:
	•	Correctness beats speed
	•	Neutrality beats features
	•	Verifiability beats convenience
	•	Stability beats novelty

This project optimizes for long-term legitimacy, not short-term adoption.

⸻

12. Amendment Rule

These constraints may only be amended if:
	1.	The amendment preserves neutrality
	2.	Determinism is not weakened
	3.	Offline verification remains possible
	4.	The system still does not execute or decide

If an amendment violates any of the above, it is invalid.

⸻

13. North Star Statement

We do not decide what is right.
We prove what was allowed, what happened, and under what rules.

Any contribution, feature, or roadmap item that conflicts with this statement must not be merged.

⸻

Closing note (to future readers)

This project deliberately chooses a narrow, disciplined path.
That discipline is the source of its power.

If you are looking to build agents, workflows, or AI products: this is not that project.
If you are looking to build trustable foundations for automated systems: you are in the right place.