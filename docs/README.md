# Northroot Documentation

This directory contains documentation organized by audience.

Northroot is open governance and accountability infrastructure for verifiable
economic activity. The current stable surface documented here is the trust
kernel: canonical identity, append-only evidence journals, replay, and offline
verification.

Higher layers such as projection, evaluation, authority, receipts, and
financial/accountability profiles are layered over the kernel. The current
project focus remains the core canonicalization and journal reference crates
before state/eval core moves into the stable surface.

## Documentation Structure

### For Users
- [Getting Started](user/getting-started.md) - Tutorial and examples
- [Integration Examples](user/integration-examples.md) - Code samples for integration

### For Developers
- [API Contract](developer/api-contract.md) - Public API surface
- [Architecture](developer/architecture.md) - System design and components
- [Testing Guide](developer/testing.md) - QA harness and test patterns
- [Layering on Northroot](developer/layering.md) - Profile, consumer protocol, custom backend, and filter patterns
- [Script Inventory](developer/script-inventory.md) - Commit-worthy release scripts and local helper boundaries

### Security
- [Security Documentation](security/README.md) - v0.1 kernel security posture and audit notes

### Reference
- [v0.1 Stability Contract](reference/v0.1-stability.md) - Stable kernel and incubating profile boundaries
- [Economic Accountability North Star](reference/economic-accountability-north-star.md) - Governed economic-action direction without product semantics
- [State Eval Core](reference/state-eval-core.md) - Incubating product-agnostic evaluation primitives
- [Core Specification](reference/spec.md) - Protocol specification
- [Journal Format](reference/format.md) - On-disk format
- [Segmented Journals](reference/segmented-journals.md) - Structural segment manifests and checkpoints
- [Canonicalization](reference/canonicalization.md) - Canonical JSON rules
- [Event Model](reference/events.md) - Event types and structure
- [Profiles](reference/profiles.md) - Consumer protocols over the kernel
- [Incubating Substrate Policy and Reference Contracts](reference/incubating-substrate-policy-refs-v0.md) - Portable refs, policy envelopes, and lifecycle records without private policy authority

### QA
- [QA Harness](qa/harness.md) - Quality assurance and testing

## Quick Links

- [Project README](../README.md) - Project overview
- [Contributing Guide](../CONTRIBUTING.md) - Development guidelines
- [Governance](../GOVERNANCE.md) - Project principles
- [Core Invariants](../CORE_INVARIANTS.md) - Non-negotiable kernel constraints

## Documentation Principles

- **Clarity**: Documentation should be clear and accessible
- **Completeness**: All public APIs should be documented
- **Accuracy**: Documentation must match implementation
- **Examples**: Include practical examples where helpful
