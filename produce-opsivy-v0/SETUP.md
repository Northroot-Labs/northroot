# Setup Guide

This guide helps you set up the produce-opsivy-v0 repository for development.

## Prerequisites

- Python 3.9+
- SQLite 3.x
- northroot v0.1.0-alpha (pinned)

## Initial Setup

### 1. Install Dependencies

```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install northroot SDK (from local or PyPI when available)
cd ../sdk/python/northroot
pip install -e .
cd ../../../produce-opsivy-v0
```

### 2. Initialize Database

```bash
# Create database from schema
sqlite3 field_ops.db < schema.sql

# Verify tables created
sqlite3 field_ops.db ".tables"
```

### 3. Set Up Receipt Storage

```bash
# Create receipts directory
mkdir -p receipts
```

### 4. Run Examples

```bash
# Record a load intake event
python examples/record_load_intake.py

# Record a storage placement (requires load_intake first)
python examples/record_storage_placement.py

# Record a quality sample (requires storage_placement first)
python examples/record_quality_sample.py
```

## Project Structure

```
produce-opsivy-v0/
├── schema.sql              # Canonical SQLite schema
├── README.md              # Overview and architecture
├── PAYLOAD_SHAPES.md      # JSON payload specifications
├── VERSION.md             # Version information
├── SETUP.md               # This file
├── .gitignore             # Git ignore rules
└── examples/
    ├── record_load_intake.py
    ├── record_storage_placement.py
    └── record_quality_sample.py
```

## Development Workflow

### 1. Record an Event

```python
from northroot import Client
import sqlite3

# Initialize
client = Client(storage_path="./receipts")
db = sqlite3.connect("field_ops.db")

# Create event payload
event = {
    "event_type": "load_intake",
    "load_id": "LOAD-2025-00123",
    # ... rest of payload
}

# Record receipt
receipt = client.record_work(
    workload_id="load_intake",
    payload=event,
    trace_id=event["load_id"],
    tags=["load", "intake"],
)
client.store_receipt(receipt)

# Store in database
db.execute("INSERT INTO loads (...) VALUES (...)", (...))
db.commit()
```

### 2. Query Receipts

```python
# Get all receipts for a load
receipts = client.list_receipts(trace_id="LOAD-2025-00123")

# Verify receipts
for receipt in receipts:
    assert client.verify_receipt(receipt)
```

### 3. Chain Events

```python
# Load intake (root)
load_receipt = client.record_work(...)

# Storage placement (chained)
placement_receipt = client.record_work(
    ...,
    parent_id=str(load_receipt.get_rid()),
)

# Quality sample (chained)
quality_receipt = client.record_work(
    ...,
    parent_id=str(placement_receipt.get_rid()),
)
```

## Next Steps

1. **API Layer**: Build REST/gRPC server wrapping event recording
2. **QR Codes**: Generate QR codes encoding receipt IDs/hashes
3. **Authentication**: Add multi-tenant support
4. **Reporting**: Build analytics on top of receipt data
5. **Integration**: Connect to external systems

## Troubleshooting

### Database Errors

```bash
# Check foreign key constraints
sqlite3 field_ops.db "PRAGMA foreign_keys;"

# Verify schema
sqlite3 field_ops.db ".schema loads"
```

### Receipt Verification Failures

```python
# Check receipt hash
print(receipt.get_hash())

# Verify manually
is_valid = client.verify_receipt(receipt)
if not is_valid:
    print("Receipt validation failed")
```

### Missing Dependencies

```bash
# Reinstall northroot SDK
cd ../sdk/python/northroot
pip install -e .
```

