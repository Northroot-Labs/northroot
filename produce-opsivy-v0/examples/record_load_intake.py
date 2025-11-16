#!/usr/bin/env python3
"""
Example: Record Load Intake Event

This example demonstrates recording a load intake event using the canonical
JSON payload format, storing it in the database, and creating a verifiable receipt.
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

# Example: Load Intake Event
load_event = {
    "event_type": "load_intake",
    "load_id": "LOAD-2025-00123",
    "grower_id": "GROWER-102",
    "field_id": "FIELD-47",
    "field_zone_id": "ZONE-47A",
    "variety_id": "VAR-RUSSET-B123",
    "harvest_date": "2025-09-28T14:30:00Z",
    "gross_weight_lbs": 42000.0,
    "tare_weight_lbs": 12000.0,
    "net_weight_lbs": 30000.0,
    "destination_type": "storage",
    "destination_id": "BIN-07",
    "truck_id": "TRUCK-14",
    "ticket_number": "TICKET-88341",
    "source_system": "field-ops-ivy",
    "external_id": None,
    "created_at": datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
    "notes": "Late-day harvest; slightly wet"
}

# Step 1: Record verifiable receipt
print("1. Recording verifiable receipt...")
receipt = client.record_work(
    workload_id="load_intake",
    payload=load_event,
    trace_id=load_event["load_id"],
    tags=["load", "intake"],
)
client.store_receipt(receipt)
print(f"   ✓ Receipt ID: {receipt.get_rid()}")
print(f"   ✓ Hash: {receipt.get_hash()[:32]}...")
print(f"   ✓ Trace ID: {load_event['load_id']}")

# Step 2: Store in database
print("\n2. Storing in database...")
db.execute("""
    INSERT INTO loads (
        load_id, grower_id, field_id, field_zone_id, variety_id,
        harvest_date, gross_weight_lbs, tare_weight_lbs, net_weight_lbs,
        destination_type, destination_id, truck_id, ticket_number,
        source_system, external_id, created_at, notes
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
""", (
    load_event["load_id"],
    load_event["grower_id"],
    load_event["field_id"],
    load_event.get("field_zone_id"),
    load_event.get("variety_id"),
    load_event["harvest_date"],
    load_event["gross_weight_lbs"],
    load_event["tare_weight_lbs"],
    load_event["net_weight_lbs"],
    load_event["destination_type"],
    load_event["destination_id"],
    load_event.get("truck_id"),
    load_event.get("ticket_number"),
    load_event["source_system"],
    load_event.get("external_id"),
    load_event["created_at"],
    load_event.get("notes"),
))
db.commit()
print("   ✓ Load stored in database")

# Step 3: Verify receipt
print("\n3. Verifying receipt...")
is_valid = client.verify_receipt(receipt)
print(f"   ✓ Receipt valid: {is_valid}")

# Step 4: Query by trace_id
print("\n4. Querying receipts by trace_id...")
all_receipts = client.list_receipts(trace_id=load_event["load_id"])
print(f"   ✓ Found {len(all_receipts)} receipts for load {load_event['load_id']}")

print("\n✅ Load intake event recorded and verified!")

