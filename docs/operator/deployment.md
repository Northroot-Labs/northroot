# Deployment Guide

Production deployment guidelines for Northroot.

## Overview

Northroot is designed to be deployed as a library within applications that need verifiable event recording. This guide covers deployment considerations and best practices.

## Deployment Models

### Embedded Library

Most common: Northroot crates are embedded in your application.

**Advantages**:
- Full control over storage and configuration
- No network overhead
- Offline-capable verification

**Considerations**:
- Journal files must be backed up
- Consider journal rotation for large deployments

### Service Deployment

Northroot can be deployed as a service exposing storage/verification APIs.

**Considerations**:
- Journal storage must be durable and backed up
- Consider journal replication for high availability
- API authentication and authorization required

## Storage Configuration

### Journal File Location

Choose a durable, backed-up location:

```rust
// Production: use durable storage
let journal_path = "/var/lib/northroot/events.nrj";

// Development: local file
let journal_path = "./events.nrj";
```

### Journal Rotation

For long-running services, consider journal rotation:

```rust
// Rotate daily or when size threshold reached
if should_rotate(&journal_path) {
    rotate_journal(&journal_path)?;
}
```

### Backup Strategy

- **Frequency**: Daily backups minimum
- **Retention**: Based on compliance requirements
- **Verification**: Periodically verify backup integrity

## Security

### File Permissions

Restrict journal file access:

```bash
chmod 600 events.nrj  # Owner read/write only
```

### Secrets Management

See [Secrets Management](secrets.md) for handling secrets in deployment.

### Kubernetes Security

See [Kubernetes Security](k8s-security.md) for K8s-specific security practices.

## Monitoring

### Metrics to Track

- Events written per second
- Journal file size
- Verification failures
- Storage errors

### Logging

Log important events:
- Journal creation/rotation
- Verification failures
- Storage errors

## Performance Considerations

### Batch Writes

Batch multiple events before flushing:

```rust
let mut writer = JournalBackendWriter::create("events.nrj")?;
for event in events {
    writer.append(&event)?;
}
writer.finish()?;  // Single flush at end
```

### Async I/O

Current implementation is sync. For high-throughput deployments, consider:
- Batching writes
- Using async runtimes for I/O-bound operations
- Separate verification from recording

## High Availability

### Journal Replication

For HA deployments:
- Replicate journal files to multiple locations
- Use distributed storage (S3, GCS) for journal files
- Verify replication integrity periodically

### Verification Redundancy

Run verification in multiple locations:
- Primary verification on write
- Periodic background verification
- Offline verification for audit

## Disaster Recovery

### Backup and Restore

1. **Backup**: Regular journal file backups
2. **Restore**: Restore from backup and verify integrity
3. **Verification**: Verify all events after restore

### Recovery Procedures

1. Identify last known good journal state
2. Restore journal from backup
3. Verify journal integrity
4. Resume event recording

## Compliance

### Audit Requirements

- Journal files are append-only and tamper-evident
- Events can be verified offline
- Receipts provide audit trail

### Retention

Configure retention based on requirements:
- Journal file retention
- Backup retention
- Verification log retention

## Related Documentation

- [Kubernetes Security](k8s-security.md) - K8s deployment security
- [Secrets Management](secrets.md) - Secret handling
- [Core Specification](../reference/spec.md) - Protocol details

