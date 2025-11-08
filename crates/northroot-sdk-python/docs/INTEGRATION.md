# Python SDK Integration Guide

## Architecture

The Python SDK will use PyO3 or CFFI to call into the Rust `northroot-engine` crate, providing:

1. **Decorator-based instrumentation** (`@delta_compute`)
2. **Context manager pattern** (`DeltaContext`)
3. **Operator wrapper pattern** (`DeltaOperator`)

## Implementation Status

- [ ] PyO3 bindings for Rust engine
- [ ] Decorator implementation
- [ ] Context manager implementation
- [ ] Operator wrapper implementation
- [ ] Spark integration module
- [ ] Dagster integration module
- [ ] Tests and examples

## Next Steps

1. Set up PyO3 project structure
2. Expose core Rust functions (decide_reuse, jaccard_similarity, receipt generation)
3. Implement Python decorator/context manager/operator patterns
4. Add Spark and Dagster integration modules

