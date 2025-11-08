# northroot-policy

[![MSRV](https://img.shields.io/badge/MSRV-1.86-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-internal-orange)](https://github.com/Northroot-Labs/northroot)

**Type:** Library  
**Publish:** No (internal)  
**MSRV:** 1.86 (Rust 1.91.0 recommended)

**Policies and strategies: cost models, reuse thresholds, allow/deny rules, FP tolerances.**

This crate defines policies and strategies for the Northroot proof algebra system, including cost models, reuse thresholds, allow/deny rules, and floating-point tolerances.

## Purpose

The `northroot-policy` crate provides:

- **Policy validation**: Policy reference format validation, determinism class enforcement
- **Constraint checking**: Tool and region constraint validation
- **Policy registry**: Policy loading and management (planned)
- **Cost models**: Resource pricing and cost computation (planned)
- **Reuse thresholds**: Policy-driven delta compute decisions (planned)
- **Allow/deny rules**: Policy enforcement and validation
- **FP tolerances**: Floating-point comparison rules for deterministic computation (planned)

## Current Implementation

The crate currently provides:
- `validate_policy_ref_format()`: Validates policy reference format (strict and legacy)
- `validate_determinism()`: Validates determinism class against policy requirements
- `validate_policy()`: Validates receipt against policy constraints
- `validate_tool_constraints()`: Tool constraint validation (stub)
- `validate_region_constraints()`: Region constraint validation (stub)
- `load_policy()`: Policy loading from registry (stub)

## Architecture Boundaries

**What this crate does**: Policy validation and enforcement
- Policy reference format validation (authoritative, detailed errors)
- Determinism class enforcement
- Tool and region constraint checking
- Policy registry lookups
- Cost models and reuse thresholds (planned)
- Allow/deny rules and FP tolerances (planned)

**What this crate does NOT do**:
- **Receipt structure validation** (see `northroot-receipts`) - answers "is this well-formed?" (syntactic)
- **Computation logic** (see `northroot-engine`) - answers "how do I compute this?"

**Dependencies** (per [ADR Playbook](../../docs/ADR_PLAYBOOK.md)):
- **Depends on**: `commons`, `receipts`
- **Must NOT depend on**: `engine` (forbidden - creates circular dependency)
- **Can be depended on by**: `engine`, `planner`, `sdk/*`, `apps/*`

**Validation layers**:
1. **Syntactic (northroot-receipts)**: Format, structure, schema - "is this well-formed?"
2. **Semantic (this crate)**: Policy compliance, business rules - "is this allowed?"
3. **Computation (northroot-engine)**: Execution logic - "how do I compute this?"

**Key principle**: Policy validation answers "is this allowed?" (semantic validation),
not "how do I compute this?" (engine concern). This separation allows policy validation
to be used independently by SDK, apps, and planner without pulling in engine internals.

## Documentation

- **[ADR Playbook](../../docs/ADR_PLAYBOOK.md)**: Repository structure and code placement guide

