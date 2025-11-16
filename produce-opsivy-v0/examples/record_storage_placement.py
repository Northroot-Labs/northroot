#!/usr/bin/env python3
"""
Example: Record Storage Placement Event

This example demonstrates recording a storage placement event, chaining it
to a previous load intake receipt.
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

# Example: Storage Placement Event
# This would typically be chained to a load_intake receipt
placement_event = {
    "event_type": "storage_placement",
    "placement_id": "PLAC-2025-00091",
    "load_id": "LOAD-2025-00123",  # Links to previous load_intake
    "storage_unit_id": "BIN-07",
    "event_type_detail": "inbound",
    "quantity_lbs": 30000.0,
    "previous_storage_unit_id": None,
    "operator_id": "OP-22",
    "event_timestamp": "2025-09-28T16:10:00Z",
    "source_system": "field-ops-ivy",
    "external_id": None,
    "notes": "Full load into BIN-07",
    "created_at": datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
}

# Step 1: Get parent receipt (load_intake)
# In production, you'd look this up from storage
print("1. Looking up parent receipt (load_intake)...")
parent_receipts = client.list_receipts(
    workload_id="load_intake",
    trace_id=placement_event["load_id"]
)
if not parent_receipts:
    print("   ⚠ No parent receipt found. Creating standalone placement.")
    parent_id = None
else:
    parent_receipt = parent_receipts[0]
    parent_id = str(parent_receipt.get_rid())
    print(f"   ✓ Found parent receipt: {parent_id[:16]}...")

# Step 2: Record verifiable receipt
print("\n2. Recording verifiable receipt...")
receipt = client.record_work(
    workload_id="storage_placement",
    payload=placement_event,
    trace_id=placement_event["load_id"],
    parent_id=parent_id,
    tags=["storage", placement_event["event_type_detail"]],
)
client.store_receipt(receipt)
print(f"   ✓ Receipt ID: {receipt.get_rid()}")
print(f"   ✓ Hash: {receipt.get_hash()[:32]}...")
if parent_id:
    print(f"   ✓ Linked to parent receipt (dom matches cod)")

# Step 3: Store in database
print("\n3. Storing in database...")
db.execute("""
    INSERT INTO storage_placements (
        placement_id, load_id, storage_unit_id, event_type,
        quantity_lbs, previous_storage_unit_id, operator_id,
        event_timestamp, source_system, external_id, notes
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
""", (
    placement_event["placement_id"],
    placement_event["load_id"],
    placement_event["storage_unit_id"],
    placement_event["event_type_detail"],  # Maps to event_type in DB
    placement_event["quantity_lbs"],
    placement_event.get("previous_storage_unit_id"),
    placement_event.get("operator_id"),
    placement_event["event_timestamp"],
    placement_event["source_system"],
    placement_event.get("external_id"),
    placement_event.get("notes"),
))
db.commit()
print("   ✓ Storage placement stored in database")

# Step 4: Verify receipt
print("\n4. Verifying receipt...")
is_valid = client.verify_receipt(receipt)
print(f"   ✓ Receipt valid: {is_valid}")

# Step 5: Query complete trace
print("\n5. Querying complete trace...")
all_receipts = client.list_receipts(trace_id=placement_event["load_id"])
print(f"   ✓ Found {len(all_receipts)} receipts in trace")
for r in all_receipts:
    print(f"   - Receipt {r.get_rid()[:16]}...")

print("\n✅ Storage placement event recorded and verified!")

