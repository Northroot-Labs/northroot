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

- **Cost models**: Resource pricing and cost computation
- **Reuse thresholds**: Policy-driven delta compute decisions
- **Allow/deny rules**: Policy enforcement and validation
- **FP tolerances**: Floating-point comparison rules for deterministic computation

## Status

This crate is currently in development. See the [ADR Playbook](../../docs/ADR_PLAYBOOK.md) for code placement guidance.

## Documentation

- **[ADR Playbook](../../docs/ADR_PLAYBOOK.md)**: Repository structure and code placement guide

