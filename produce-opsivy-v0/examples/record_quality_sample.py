#!/usr/bin/env python3
"""
Example: Record Quality Sample Event

This example demonstrates recording a quality sample event, chaining it
to a previous storage placement receipt.
"""

from northroot import Client
import sqlite3
import json
from datetime import datetime, timezone

# Initialize northroot client
client = Client(storage_path="./receipts")

# Initialize database
db = sqlite3.connect("field_ops.db")
db.execute("PRAGMA foreign_keys = ON")

# Example: Quality Sample Event
# This would typically be chained to a storage_placement receipt
quality_event = {
    "event_type": "quality_sample",
    "sample_id": "SAMPLE-2025-00451",
    "load_id": "LOAD-2025-00123",  # Links to load trace
    "storage_unit_id": "BIN-07",
    "sample_timestamp": "2025-10-15T09:20:00Z",
    "sample_stage": "storage",
    "sample_method": "core",
    "bruise_pct": 19.5,
    "rot_pct": 3.2,
    "glucose_pct": 1.1,
    "sucrose_pct": 2.3,
    "dry_matter_pct": 20.8,
    "pulp_temp_c": 6.2,
    "ambient_temp_c": 5.5,
    "lab_id": "LAB-01",
    "technician_id": "TECH-07",
    "source_system": "field-ops-ivy",
    "external_id": None,
    "notes": "Bruise slightly high; borderline for processor X",
    "created_at": datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
}

# Step 1: Get parent receipt (storage_placement)
# In production, you'd look this up from storage
print("1. Looking up parent receipt (storage_placement)...")
parent_receipts = client.list_receipts(
    workload_id="storage_placement",
    trace_id=quality_event["load_id"]
)
if not parent_receipts:
    print("   ⚠ No parent receipt found. Creating standalone quality sample.")
    parent_id = None
else:
    parent_receipt = parent_receipts[0]
    parent_id = str(parent_receipt.get_rid())
    print(f"   ✓ Found parent receipt: {parent_id[:16]}...")

# Step 2: Record verifiable receipt
print("\n2. Recording verifiable receipt...")
receipt = client.record_work(
    workload_id="quality_sample",
    payload=quality_event,
    trace_id=quality_event["load_id"],
    parent_id=parent_id,
    tags=["quality", quality_event["sample_stage"]],
)
client.store_receipt(receipt)
print(f"   ✓ Receipt ID: {receipt.get_rid()}")
print(f"   ✓ Hash: {receipt.get_hash()[:32]}...")
if parent_id:
    print(f"   ✓ Linked to parent receipt (dom matches cod)")

# Step 3: Store in database
print("\n3. Storing in database...")
db.execute("""
    INSERT INTO quality_samples (
        sample_id, load_id, storage_unit_id, sample_timestamp,
        sample_stage, sample_method, bruise_pct, rot_pct,
        glucose_pct, sucrose_pct, dry_matter_pct,
        pulp_temp_c, ambient_temp_c, lab_id, technician_id,
        source_system, external_id, notes
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
""", (
    quality_event["sample_id"],
    quality_event["load_id"],
    quality_event.get("storage_unit_id"),
    quality_event["sample_timestamp"],
    quality_event["sample_stage"],
    quality_event.get("sample_method"),
    quality_event.get("bruise_pct"),
    quality_event.get("rot_pct"),
    quality_event.get("glucose_pct"),
    quality_event.get("sucrose_pct"),
    quality_event.get("dry_matter_pct"),
    quality_event.get("pulp_temp_c"),
    quality_event.get("ambient_temp_c"),
    quality_event.get("lab_id"),
    quality_event.get("technician_id"),
    quality_event["source_system"],
    quality_event.get("external_id"),
    quality_event.get("notes"),
))
db.commit()
print("   ✓ Quality sample stored in database")

# Step 4: Verify receipt
print("\n4. Verifying receipt...")
is_valid = client.verify_receipt(receipt)
print(f"   ✓ Receipt valid: {is_valid}")

# Step 5: Query complete trace
print("\n5. Querying complete trace...")
all_receipts = client.list_receipts(trace_id=quality_event["load_id"])
print(f"   ✓ Found {len(all_receipts)} receipts in trace")
for r in all_receipts:
    print(f"   - Receipt {r.get_rid()[:16]}...")

print("\n✅ Quality sample event recorded and verified!")

