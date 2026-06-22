# Northroot Documentation

This directory contains documentation organized by audience.

Northroot is open governance and accountability infrastructure for verifiable
state transitions. The stable kernel is canonical identity plus `.nrj` journal
verification. Current repository code also includes record streams, node and
workspace manifests, governance/execution/exchange profile helpers, a sanitized
ag-domain crate, the promoted `northroot.custody` stewardship package, and the
legacy-compatibility `northroot.durability` Python package.

## Documentation Structure

### For Users

- [Getting Started](user/getting-started.md) - tutorial and examples
- [Integration Examples](user/integration-examples.md) - code samples for integration

### For Developers

- [Environment and Setup](developer/environment.md) - neutral setup and verification entrypoints
- [API Contract](developer/api-contract.md) - public API surface
- [Architecture](developer/architecture.md) - system design and components
- [Stewardship Workstream](developer/stewardship-workstream.md) - object custody and legacy import context
- [Testing Guide](developer/testing.md) - QA harness and test patterns
- [Layering on Northroot](developer/layering.md) - profile, consumer protocol, custom backend, and filter patterns
- [Script Inventory](developer/script-inventory.md) - release scripts, setup scripts, and helper boundaries

### Security

- [Security Documentation](security/README.md) - kernel security posture and audit notes

### Reference

- [v0.1 Stability Contract](reference/v0.1-stability.md) - stable kernel and incubating profile boundaries
- [Record V0 Stack](reference/record-v0/stack.md) - neutral records over `.nrj` plus JSONL interchange
- [Economic Accountability North Star](reference/economic-accountability-north-star.md)
- [State Eval Core](reference/state-eval-core.md)
- [Core Specification](reference/spec.md)
- [Journal Format](reference/format.md)
- [Segmented Journals](reference/segmented-journals.md)
- [Canonicalization](reference/canonicalization.md)
- [Event Model](reference/events.md)
- [Profiles](reference/profiles.md)
- [Incubating Substrate Policy and Reference Contracts](reference/incubating-substrate-policy-refs-v0.md)

### QA

- [QA Harness](qa/harness.md) - quality assurance and testing

## Quick Links

- [Project README](../README.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Governance](../GOVERNANCE.md)
- [Core Invariants](../CORE_INVARIANTS.md)

## Documentation Principles

- **Accuracy**: docs must match the implementation and current crate layout.
- **Boundary clarity**: public kernel, incubating capability, and private deployment language must stay separate.
- **Environment neutrality**: setup docs should work for humans, CI, containers, and agents without requiring Codex.
- **Examples**: include practical examples where helpful, but keep private adapters and real operational data out of public docs.
