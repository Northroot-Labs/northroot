# Northroot Python SDK

Python SDK for the Northroot proof algebra system, providing high-level Python bindings to the Rust engine.

## Features

- **Minimal API (v0.1)**: Simple `record_work` and `verify_receipt` functions for verifiable proofs
- **Delta Compute**: Reuse decision logic, Jaccard similarity, economic delta computation
- **Data Shapes**: Compute data and method shape hashes from files, bytes, or signatures
- **Receipts**: Create, validate, and serialize receipts

## Installation

```bash
# Build from source (requires Rust and maturin)
maturin develop

# Or install in development mode
pip install -e .
```

## Usage

### Minimal API (v0.1) - Quick Start

The minimal v0.1 API uses the `Client` class for a simple, consistent interface:

```python
from northroot import Client

# Create a client (storage is decoupled and optional for v0.1)
client = Client()
# client = Client(storage_path="./receipts")  # With filesystem storage (future)

# Record a unit of work and get a verifiable receipt
receipt = client.record_work(
    workload_id="normalize-prices",
    payload={"input_hash": "sha256:abc...", "output_hash": "sha256:def..."},
    tags=["etl", "batch"],
    trace_id="trace-2025-01-17",
    parent_id=None,  # Optional: link to parent receipt for DAGs
)

print(f"Receipt ID: {receipt.get_rid()}")
print(f"Hash: {receipt.get_hash()}")

# Verify receipt integrity
is_valid = client.verify_receipt(receipt)
print(f"Receipt is valid: {is_valid}")

# Create a DAG: child receipt linked to parent
child_receipt = client.record_work(
    workload_id="aggregate-totals",
    payload={"input_receipt": receipt.get_rid(), "result": "sum"},
    tags=["etl"],
    trace_id="trace-2025-01-17",  # Same trace
    parent_id=receipt.get_rid(),   # Parent link
)

# Async versions are also available
receipt_async = await client.record_work_async(...)
is_valid_async = await client.verify_receipt_async(receipt_async)
```

See `examples/quickstart.py` for a complete example.

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
print(f"Justification: {result['justification']}")

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
method_hash = northroot.shapes.compute_method_shape_hash_from_code(
    code_hash,
    params={"batch_size": 1000}
)

# Compute method shape hash from signature
method_hash2 = northroot.shapes.compute_method_shape_hash_from_signature(
    "normalize_ledger",
    ["Vec<Transaction>", "Config"],
    "Ledger"
)
```

### Receipts

```python
import northroot
import json

# Create receipt from JSON
receipt_json = json.dumps({
    "rid": "01234567-89ab-cdef-0123-456789abcdef",
    "version": "0.3.0",
    "kind": "execution",
    # ... rest of receipt structure
})

receipt = northroot.receipts.receipt_from_json(receipt_json)

# Validate receipt
receipt.validate()  # Raises ValueError if invalid

# Compute hash
hash_value = receipt.compute_hash()
print(f"Receipt hash: {hash_value}")

# Serialize to JSON
json_str = receipt.to_json()

# Access receipt properties
print(f"RID: {receipt.get_rid()}")
print(f"Kind: {receipt.get_kind()}")
print(f"Version: {receipt.get_version()}")
print(f"Hash: {receipt.get_hash()}")
```

## API Reference

### Delta Module

- `decide_reuse(overlap_j: float, cost_model: dict, row_count: int | None = None) -> dict`
  - Decide whether to reuse based on overlap and cost model
  - Returns: `{"decision": str, "justification": dict}`

- `economic_delta(overlap_j: float, cost_model: dict, row_count: int | None = None) -> float`
  - Compute economic delta (savings estimate)
  - Returns: Economic delta (positive = savings)

- `jaccard_similarity(set1: list[str], set2: list[str]) -> float`
  - Compute Jaccard similarity between two sets
  - Returns: Similarity value in [0, 1]

### Shapes Module

- `compute_data_shape_hash_from_file(path: str, chunk_scheme: dict | None = None) -> str`
  - Compute data shape hash from file
  - Returns: Hash in format `sha256:<64hex>`

- `compute_data_shape_hash_from_bytes(data: bytes, chunk_scheme: dict | None = None) -> str`
  - Compute data shape hash from bytes
  - Returns: Hash in format `sha256:<64hex>`

- `compute_method_shape_hash_from_code(code_hash: str, params: dict | None = None) -> str`
  - Compute method shape hash from code hash and parameters
  - Returns: Hash in format `sha256:<64hex>`

- `compute_method_shape_hash_from_signature(function_name: str, input_types: list[str], output_type: str) -> str`
  - Compute method shape hash from function signature
  - Returns: Hash in format `sha256:<64hex>`

### Receipts Module

- `receipt_from_json(json_str: str) -> PyReceipt`
  - Create receipt from JSON string
  - Returns: PyReceipt object

#### PyReceipt Class

- `validate() -> None`
  - Validate receipt (raises ValueError if invalid)

- `compute_hash() -> str`
  - Compute hash from canonical body
  - Returns: Hash in format `sha256:<64hex>`

- `to_json() -> str`
  - Serialize receipt to JSON string
  - Returns: JSON string

- `get_rid() -> str`
  - Get receipt ID (RID)

- `get_kind() -> str`
  - Get receipt kind

- `get_version() -> str`
  - Get receipt version

- `get_hash() -> str`
  - Get receipt hash

## Development

```bash
# Build in development mode
maturin develop

# Run tests
pytest

# Format code
black .

# Type check
mypy .
```

## Status

**Alpha** - This SDK is in early development. API may change.

## Installation

### From Source (Development)

```bash
# Install maturin
pip install maturin

# Build and install
cd sdk/northroot-sdk-python
maturin develop
```

### From PyPI (Future)

```bash
pip install northroot-sdk
```

## Quick Start

```python
import northroot

# Delta compute operations
from northroot.delta import decide_reuse, jaccard_similarity

# Receipt operations
from northroot.receipts import Receipt

# Shape computation
from northroot.shapes import compute_data_shape_hash
```

## Architecture

This SDK provides Python bindings to the Rust `northroot-engine` crate via PyO3.

- **SDK Location**: `sdk/northroot-sdk-python/` (not `crates/`)
- **Core Engine**: `crates/northroot-engine/` (Rust)
- **Clear Boundaries**: SDK = language bindings, Engine = core logic

## Development

### Prerequisites

- Rust toolchain (latest stable)
- Python 3.8+
- maturin

### Building

```bash
maturin develop
```

### Testing

```bash
# Rust tests
cargo test

# Python tests (when implemented)
pytest tests/
```

## License

Apache 2.0

