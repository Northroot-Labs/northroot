# Northroot Python SDK

Python SDK for the Northroot proof algebra system, providing high-level Python bindings to the Rust engine.

## Features

- **Minimal API (v0.1)**: Simple `record_work` and `verify_receipt` functions for verifiable proofs
- **Receipt Storage**: Filesystem-based storage with querying and filtering
- **Async/Sync Support**: Both synchronous and asynchronous APIs available
- **OpenTelemetry Integration**: Optional OTEL span → receipt conversion
- **Delta Compute**: Reuse decision logic, Jaccard similarity, economic delta computation
- **Data Shapes**: Compute data and method shape hashes from files, bytes, or signatures

## Installation

### From PyPI (Recommended)

```bash
pip install northroot
```

### From Source (Development)

```bash
# Quick setup (recommended)
cd sdk/python/northroot
./setup-dev.sh

# Or manual setup
pip install maturin
cd sdk/python/northroot
maturin develop
```

## Quick Start

### Minimal API (v0.1)

```python
from northroot import Client

# Create a client (storage is optional)
client = Client()
# client = Client(storage_path="./receipts")  # With filesystem storage

# Record a unit of work and get a verifiable receipt
receipt = client.record_work(
    workload_id="normalize-prices",
    payload={"input_hash": "sha256:abc...", "output_hash": "sha256:def..."},
    tags=["etl", "batch"],
    trace_id="trace-2025-01-17",
)

print(f"Receipt ID: {receipt.get_rid()}")
print(f"Hash: {receipt.get_hash()}")

# Verify receipt integrity
is_valid = client.verify_receipt(receipt)
print(f"Receipt is valid: {is_valid}")

# Store receipt (if storage_path was provided)
client.store_receipt(receipt)

# List receipts with filtering
receipts = client.list_receipts(
    workload_id="normalize-prices",
    trace_id="trace-2025-01-17",
    limit=10
)

# Async versions are also available
receipt_async = await client.record_work_async(...)
is_valid_async = await client.verify_receipt_async(receipt_async)
```

See `examples/quickstart.py` for a complete example.

## Usage Examples

### Receipt Storage and Querying

```python
from northroot import Client

client = Client(storage_path="./receipts")

# Record and store receipts
receipt1 = client.record_work("workload-1", {"data": "value1"})
client.store_receipt(receipt1)

receipt2 = client.record_work("workload-1", {"data": "value2"}, trace_id="trace-123")
client.store_receipt(receipt2)

# Query receipts
all_receipts = client.list_receipts()
workload_receipts = client.list_receipts(workload_id="workload-1")
trace_receipts = client.list_receipts(trace_id="trace-123")
```

### OpenTelemetry Integration

```python
from northroot import Client
from northroot.otel import trace_work, span_to_receipt
from opentelemetry import trace

client = Client()

# Option 1: Decorator
@trace_work(client, workload_id="my-workload")
def my_function():
    # Your code here
    return {"result": "data"}

# Option 2: Manual conversion
tracer = trace.get_tracer(__name__)
with tracer.start_as_current_span("my-operation") as span:
    # Your code here
    receipt = span_to_receipt(span, client, "my-workload", {"data": "value"})
```

See `examples/otel_integration.py` for more examples.

### Delta Compute

```python
import northroot as nr

# Decide whether to reuse based on overlap
cost_model = {
    "c_id": 10.0,  # Identity cost
    "c_comp": 100.0,  # Compute cost
    "alpha": 0.9,  # Incrementality factor
}
overlap_j = 0.15  # 15% Jaccard overlap

result = nr.delta.decide_reuse(overlap_j, cost_model)
print(f"Decision: {result['decision']}")  # "reuse" or "recompute"

# Compute economic delta (savings estimate)
delta = nr.delta.economic_delta(overlap_j, cost_model)
print(f"Economic delta: {delta}")  # Positive = savings

# Compute Jaccard similarity between two sets
set1 = ["chunk1", "chunk2", "chunk3"]
set2 = ["chunk2", "chunk3", "chunk4"]
jaccard = nr.delta.jaccard_similarity(set1, set2)
print(f"Jaccard similarity: {jaccard}")  # 0.5 (2/4)
```

### Data Shapes

```python
import northroot as nr

# Compute data shape hash from file
hash1 = nr.shapes.compute_data_shape_hash_from_file(
    "data.csv",
    chunk_scheme={"type": "cdc", "avg_size": 65536}
)

# Compute data shape hash from bytes
data = b"some binary data"
hash2 = nr.shapes.compute_data_shape_hash_from_bytes(
    data,
    chunk_scheme={"type": "fixed", "size": 1024}
)

# Compute method shape hash from code hash
code_hash = "sha256:abc123..."
method_hash = nr.shapes.compute_method_shape_hash_from_code(
    code_hash,
    params={"batch_size": 1000}
)
```

## Development

### Prerequisites

- Rust toolchain (latest stable)
- Python 3.10+ (3.12 recommended)
- maturin

### Quick Setup

```bash
cd sdk/python/northroot
./setup-dev.sh
```

This creates a virtual environment, installs dependencies, and builds the package.

### Manual Setup

```bash
cd sdk/python/northroot

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install development dependencies
pip install -e ".[dev]"

# Build in development mode
maturin develop
```

### Development Workflow

```bash
# Activate environment
cd sdk/python/northroot
source venv/bin/activate

# Rebuild after code changes
maturin develop
# Or use make
make develop

# Run examples
./run-example.sh
# Or manually
python examples/quickstart.py

# Build release wheels
maturin build --release

# Run tests
pytest tests/

# Format code
black .

# Type check
mypy .
```

### Helper Scripts

- `setup-dev.sh` - One-command development environment setup
- `run-example.sh` - Run examples with proper environment activation
- `Makefile` - Convenience targets for common tasks

### Troubleshooting

**Build fails with "missing field `package`":**
- Make sure you're in `sdk/python/northroot` directory
- The `Cargo.toml` should have `[package]` section, not `[workspace]`

**Import errors:**
- Activate venv: `source venv/bin/activate`
- Rebuild: `maturin develop`
- Check that `northroot/__init__.py` exists

**Rust compilation errors:**
- Make sure Rust toolchain is installed: `rustc --version`
- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && maturin develop`

## API Reference

### Client Class

- `Client(storage_path: Optional[str] = None)` - Create a new client
- `record_work(workload_id: str, payload: Dict, tags: Optional[List[str]] = None, trace_id: Optional[str] = None, parent_id: Optional[str] = None) -> PyReceipt`
- `verify_receipt(receipt: PyReceipt) -> bool`
- `store_receipt(receipt: PyReceipt) -> None`
- `list_receipts(workload_id: Optional[str] = None, trace_id: Optional[str] = None, limit: Optional[int] = None) -> List[PyReceipt]`
- `record_work_async(...)` - Async version of `record_work`
- `verify_receipt_async(...)` - Async version of `verify_receipt`
- `list_receipts_async(...)` - Async version of `list_receipts`

### Delta Module

- `decide_reuse(overlap_j: float, cost_model: dict, row_count: int | None = None) -> dict`
- `economic_delta(overlap_j: float, cost_model: dict, row_count: int | None = None) -> float`
- `jaccard_similarity(set1: list[str], set2: list[str]) -> float`

### Shapes Module

- `compute_data_shape_hash_from_file(path: str, chunk_scheme: dict | None = None) -> str`
- `compute_data_shape_hash_from_bytes(data: bytes, chunk_scheme: dict | None = None) -> str`
- `compute_method_shape_hash_from_code(code_hash: str, params: dict | None = None) -> str`
- `compute_method_shape_hash_from_signature(function_name: str, input_types: list[str], output_type: str) -> str`

### Receipts Module

- `receipt_from_json(json_str: str) -> PyReceipt`

#### PyReceipt Class

- `validate() -> None` - Validate receipt (raises ValueError if invalid)
- `compute_hash() -> str` - Compute hash from canonical body
- `to_json() -> str` - Serialize receipt to JSON string
- `get_rid() -> str` - Get receipt ID (RID)
- `get_kind() -> str` - Get receipt kind
- `get_version() -> str` - Get receipt version
- `get_hash() -> str` - Get receipt hash

## Architecture

This SDK provides Python bindings to the Rust `northroot-engine` crate via PyO3.

- **SDK Location**: `sdk/python/northroot/` (not `crates/`)
- **Core Engine**: `crates/northroot-engine/` (Rust)
- **Clear Boundaries**: SDK = language bindings, Engine = core logic

## Status

**v0.1.0 (Alpha)** - This SDK is in early development. API may change.

### What's New in v0.1.0

- ✅ Minimal API: `record_work()` and `verify_receipt()` for verifiable proofs
- ✅ Receipt storage and listing with filtering (workload_id, trace_id)
- ✅ Async/sync support for all operations
- ✅ OpenTelemetry integration (optional)
- ✅ Filesystem-based receipt storage

## License

Apache 2.0
