# Security Documentation

Security documentation for Northroot, focusing on threat modeling, attack surface analysis, and hardening requirements.

## Threat Model

- **[Threat Model (JSON)](threat-model.json)** - Structured threat model analysis
- **[Threat Model Summary](threat-model.md)** - Human-readable summary

The threat model documents:
- Adversary capabilities and goals
- Attack surface analysis for CLI commands
- Security hardening requirements (critical, high, medium priority)
- Sandbox requirements for safe execution
- Audit guarantees and compliance notes

## Key Security Findings

### Critical Issues (P0)
1. **Path validation** - All file path arguments must be validated to prevent path traversal
2. **Memory exhaustion in verify** - `verify` command loads entire journal into memory (needs streaming fix)

### High Priority (P1)
1. **Resource limits** - Add configurable limits for journal size, event count, memory usage
2. **Symlink handling** - Resolve symlinks before opening files

### Security Posture
- **Memory safety**: Excellent (Rust, no unsafe code)
- **Attack surface**: Minimal (read-only CLI, offline verification)
- **Journal format**: Tamper-evident, append-only, bounded payloads

## Related Documentation

- [Kubernetes Security](../operator/k8s-security.md) - K8s-specific security practices
- [Secrets Management](../operator/secrets.md) - Secret handling
- [Deployment Guide](../operator/deployment.md) - Production deployment security

