# Testing Guide

How to write tests, run the QA harness, and add golden tests.

## Quick Start

Run all fast checks before pushing:

```bash
just qa
```

This runs: format check, clippy, tests, and golden tests.

## Test Types

### Unit Tests

Located alongside source code in `src/` with `#[cfg(test)]` modules.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalization() {
        // Test implementation
    }
}
```

Run with:
```bash
cargo test
```

### Integration Tests

Located in `tests/` directories at crate root.

```rust
// tests/integration.rs
use northroot_store::*;

#[test]
fn test_journal_roundtrip() {
    // Integration test
}
```

Run with:
```bash
cargo test --test integration
```

### Golden Tests

Golden tests verify canonicalization stability and hash determinism.

Located in `crates/northroot-canonical/tests/golden.rs`.

Run with:
```bash
just golden
```

To update golden files after intentional changes:
```bash
UPDATE_GOLDEN=1 cargo test --test golden
```

## Running Tests

For information on running tests, CI workflows, and the QA harness, see [QA Harness](../qa/harness.md).

## Writing Tests

### Test Structure

1. **Arrange**: Set up test data and state
2. **Act**: Execute the code under test
3. **Assert**: Verify expected outcomes

### Example: Testing Event Verification

```rust
#[test]
fn test_verify_authorization() {
    // Arrange
    let profile = ProfileId::parse("northroot-canonical-v1").unwrap();
    let canonicalizer = Canonicalizer::new(profile);
    let verifier = Verifier::new(canonicalizer);
    let event = create_test_authorization_event();
    
    // Act
    let (digest, verdict) = verifier.verify_authorization(&event).unwrap();
    
    // Assert
    assert_eq!(verdict, VerificationVerdict::Ok);
    assert_eq!(digest, event.event_id);
}
```

### Example: Testing Error Cases

```rust
#[test]
fn test_invalid_event_id() {
    let event = create_event_with_invalid_id();
    let profile = ProfileId::parse("northroot-canonical-v1").unwrap();
    let canonicalizer = Canonicalizer::new(profile);
    let verifier = Verifier::new(canonicalizer);
    
    let result = verifier.verify_authorization(&event);
    assert!(result.is_err());
}
```

## Property-Based Testing

For critical paths (canonicalization, hashing), consider property-based tests:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn canonical_bytes_are_deterministic(input in any::<serde_json::Value>()) {
        let c1 = canonicalize(&input)?;
        let c2 = canonicalize(&input)?;
        assert_eq!(c1, c2);
    }
}
```

## Best Practices

1. **Test public APIs**: Focus on public interfaces, not implementation details
2. **Test error cases**: Verify error handling and edge cases
3. **Keep tests fast**: Unit tests should run in milliseconds
4. **Use descriptive names**: Test names should describe what they verify
5. **Avoid test interdependencies**: Tests should be independent and runnable in any order

## Running Tests and CI

For information on running tests, CI workflows, coverage reports, and the QA harness, see [QA Harness](../qa/harness.md).

