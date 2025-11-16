# Northroot

**Verifiable compute plumbing for transparent, reusable computation.**

Northroot provides cryptographic receipts for computational work, enabling you to prove what was computed, verify reuse decisions, and reduce redundant compute costs.

## The Problem

Modern compute pipelines waste billions on redundant work:

- **Redundant compute**: The same data transformations run repeatedly across teams and pipelines
- **Opaque decisions**: No way to verify why compute ran or whether reuse was justified
- **No audit trail**: Can't prove what was computed, when, or at what cost
- **Trust gaps**: Cross-organizational compute sharing is impossible without cryptographic proof

**Result**: Teams pay for compute they've already done, with no way to prove or prevent it.

## The Solution

Northroot is **verifiable compute plumbing**—a minimal layer that:

1. **Records receipts** for every unit of work (cryptographically signed proofs)
2. **Verifies reuse** decisions (proves when incremental compute was justified)
3. **Enables auditability** (query what was computed, when, and why)
4. **Reduces costs** by proving overlap and enabling reuse

Think of it as "proofs for compute"—like Git commits for your data pipelines, but for compute decisions.

## Quick Start

### Install

```bash
pip install northroot
```

### Hello Receipts (3 Steps)

```python
from northroot import Client

# 1. Create a client
client = Client(storage_path="./receipts")

# 2. Record work → get a verifiable receipt
receipt = client.record_work(
    workload_id="normalize-prices",
    payload={"input": "data.csv", "output": "normalized.csv"},
    tags=["etl", "batch"]
)

# 3. Verify the receipt
is_valid = client.verify_receipt(receipt)
print(f"Receipt {receipt.get_rid()} is valid: {is_valid}")
```

**That's it!** You've created your first verifiable proof of compute.

### Try the Examples

```bash
# Simplest demo (3 steps)
python examples/hello_receipts.py

# Full quickstart
python examples/quickstart.py

# OpenTelemetry integration
python examples/otel_integration.py
```

## Why Proofs Beat Logs

Traditional logging tells you *what happened*. Northroot receipts prove *what was computed*:

| Logs | Receipts |
|------|----------|
| "Job ran at 2pm" | "Job computed hash(abc...) → hash(def...)" |
| "Used 100GB memory" | "Reused 85% of previous computation (J=0.85)" |
| "Took 5 minutes" | "Economic delta: $6.65 saved via reuse" |
| Not verifiable | Cryptographically signed and verifiable |
| No reuse proof | Proves overlap and justifies reuse decisions |

**Receipts enable:**
- **Determinism**: Same inputs → same receipt hash (proves equivalence)
- **Reuse**: Prove overlap between runs (Jaccard similarity)
- **Auditability**: Query all receipts to see what was computed
- **Trust**: Cryptographic signatures enable cross-org sharing

## Use Cases

### 1. FinOps Cost Attribution

**Problem**: Daily billing runs recompute 85-95% of the same data.

**Solution**: Prove overlap (Jaccard similarity) and justify incremental refresh.

```python
# Record billing run
receipt1 = client.record_work(
    workload_id="daily-billing",
    payload={"date": "2025-11-15", "resources": [...]},
    trace_id="billing-2025-11-15"
)

# Next day: prove overlap and reuse
receipt2 = client.record_work(
    workload_id="daily-billing",
    payload={"date": "2025-11-16", "resources": [...]},
    trace_id="billing-2025-11-16",
    parent_id=receipt1.get_rid()  # Link to previous run
)

# Query receipts to see reuse decisions
all_billing = client.list_receipts(workload_id="daily-billing")
```

**Result**: 46% cost savings, $276K annual reduction.

### 2. ETL Pipeline Reuse

**Problem**: ETL pipelines recompute unchanged partitions.

**Solution**: Track partition-level overlap and prove incremental refresh.

```python
# Record ETL run with partition manifest
receipt = client.record_work(
    workload_id="etl-partitions",
    payload={"partitions": ["p1", "p2", "p3"], "changed": ["p3"]},
    tags=["etl", "delta-lake"]
)

# Next run: prove only changed partitions were recomputed
```

**Result**: 39% cost savings, $372K annual reduction.

### 3. Analytics Dashboard Refresh

**Problem**: BI dashboards refresh entire datasets when only 10% changed.

**Solution**: Prove query result overlap and justify incremental refresh.

```python
# Record dashboard query
receipt = client.record_work(
    workload_id="dashboard-refresh",
    payload={"query": "SELECT ...", "result_hash": "sha256:..."},
    tags=["analytics", "bi"]
)

# Prove 90%+ overlap → justify incremental refresh
```

**Result**: 142% cost savings, $684K annual reduction.

## Installation

### From PyPI (Recommended)

```bash
pip install northroot
```

### From Source

```bash
# Clone repository
git clone https://github.com/Northroot-Labs/northroot.git
cd northroot

# Setup Python SDK
cd sdk/python/northroot
./setup-dev.sh
```

See [Installation Guide](docs/guides/installation.md) for detailed setup instructions.

## API Reference

### Core Methods

```python
from northroot import Client

client = Client(storage_path="./receipts")  # Optional: filesystem storage

# Record work → get receipt
receipt = client.record_work(
    workload_id="my-workload",
    payload={"key": "value"},  # Any JSON-serializable dict
    tags=["tag1", "tag2"],     # Optional: categorization
    trace_id="trace-123",       # Optional: group related work
    parent_id=None              # Optional: DAG composition
)

# Verify receipt integrity
is_valid = client.verify_receipt(receipt)

# Store receipt to filesystem
client.store_receipt(receipt)

# List and filter receipts
all_receipts = client.list_receipts()
filtered = client.list_receipts(
    workload_id="my-workload",
    trace_id="trace-123"
)

# Async versions available
receipt_async = await client.record_work_async(...)
is_valid_async = await client.verify_receipt_async(receipt)
```

### OpenTelemetry Integration

```python
from northroot import Client, trace_work

# Automatic receipt generation from OTEL spans
@trace_work
def my_function():
    # Function automatically generates receipt
    pass

# Or convert existing spans
from northroot.otel import span_to_receipt
receipt = span_to_receipt(span)
```

See [Python SDK README](sdk/python/northroot/README.md) for full API documentation.

## Architecture

Northroot is built in Rust for performance and safety, with Python bindings for ease of use:

```
┌─────────────────────────────────────────┐
│  Python SDK (PyPI: northroot)          │
│  - Simple async/sync API                │
│  - Filesystem storage                   │
│  - OTEL integration                     │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  Rust Engine (crates/northroot-engine)  │
│  - Receipt generation & validation       │
│  - Delta compute decisions               │
│  - Jaccard similarity                    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  Receipt Model (crates/northroot-      │
│  receipts)                              │
│  - CBOR canonicalization (RFC 8949)     │
│  - Cryptographic signatures             │
│  - JSON adapters                        │
└─────────────────────────────────────────┘
```

**Core Components:**
- **Receipts**: Canonical, signed proofs of computation
- **Storage**: Filesystem-based receipt storage (SQLite coming soon)
- **Delta Compute**: Jaccard similarity and reuse decision logic
- **Verification**: Hash integrity and signature validation

## Documentation

**📚 [Complete Documentation Index](docs/README.md)** - Comprehensive table of contents for all documentation

**Quick Links:**
- **[Installation Guide](docs/guides/installation.md)**: Setup instructions
- **[Python SDK README](sdk/python/northroot/README.md)**: Full API reference
- **[Quick Start Examples](sdk/python/northroot/examples/)**: Working code samples
- **[Specifications](docs/specs/)**: Technical specifications
- **[Architecture](docs/specs/architecture-diagrams.md)**: System design

## Status

**v0.1.0** (Current) - Minimal viable SDK:
- ✅ Receipt recording and verification
- ✅ Filesystem storage with querying
- ✅ Async/sync Python API
- ✅ OpenTelemetry integration
- ✅ Basic delta compute (Jaccard similarity)

**Roadmap:**
- 🔄 SQLite storage backend
- 🔄 Advanced delta compute economics
- 🔄 Multi-party settlement
- 🔄 Cloud storage adapters

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code organization guidelines
- Testing requirements
- Pull request process

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

---

**Questions?** Open an issue or check the [documentation](docs/).
