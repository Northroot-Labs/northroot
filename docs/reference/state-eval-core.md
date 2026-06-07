# State Eval Core

Status: Incubating open tooling

`northroot-state-eval` provides product-agnostic state and policy evaluation
primitives. It is intentionally separate from the stable Northroot kernel
surface. The kernel still owns canonical identity, proof refs, journal framing,
and offline verification. State/eval owns deterministic predicate composition
and evaluation result shapes over projected state.

## Scope

The crate provides:

- `EventFrame` and `EventCursor` for byte-stream-friendly ordered prefixes;
- `ProjectedState<T>` for derived state identity;
- `TruthValue` and `Satisfaction` for three-valued evaluation;
- `PolicyExpr` for predicate composition;
- `PredicateResult`, `EvaluationNode`, and `EvaluationResult`;
- `EvaluationDelta` derived from an evaluation tree;
- `GateDecision` and `GateResult` for pre-action, pre-transition, or pre-append
  checks.

It does not provide:

- product policy authority;
- a policy language such as Rego, CEL, or Cedar;
- an agent runtime;
- a queue;
- a database adapter;
- telemetry or observability infrastructure;
- provider SDKs or network integrations.

## Evaluation Semantics

Predicate implementations are externally supplied. The crate owns only the
calling convention and deterministic three-valued composition:

```text
PredicateRef(id) -> True | False | Unknown
And([...])
Or([...])
Not(...)
```

The root truth value maps to satisfaction:

```text
True    -> Satisfied
False   -> Unsatisfied
Unknown -> Indeterminate
```

`Pass` and `Fail` are product, CLI, test, or dashboard renderings. They are not
the core semantic values.

## Gate and Outcome Paths

The same predicate machinery can support two paths:

```text
pre-action or pre-append:
  candidate + authority/policy predicates -> Accept | Reject | Hold

post-projection:
  projected state + target policy -> EvaluationResult + EvaluationDelta
```

The crate defines result shapes for both paths. It does not decide which gate,
authority source, or policy is legitimate.

## Policy Engines

OPA/Rego, CEL, Cedar, or other policy engines may earn their keep as adapters.
They should not be required dependencies of `northroot-state-eval`.

An adapter may:

- translate a policy-engine result into `PredicateResult`;
- use Northroot refs as evidence inputs;
- preserve policy engine metadata in outer receipts or product records.

An adapter must not:

- make the policy engine the core state/eval dependency;
- add product semantics to the crate;
- hide non-deterministic network or provider calls inside predicate evaluation.

## Boundary

Downstream concepts such as work items, exception queues, review tasks, agent
leases, notifications, or business completion are derived views over evaluated
state, deltas, authority decisions, and receipts. The crate does not own those
lifecycles.
