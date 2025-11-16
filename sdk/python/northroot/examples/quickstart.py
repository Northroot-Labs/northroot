#!/usr/bin/env python3
"""
Northroot SDK Quickstart Example

This example demonstrates the minimal v0.1 API using the Client class:
- client.record_work: Create verifiable receipts for units of work
- client.verify_receipt: Verify receipt integrity
- client.store_receipt: Store receipts to filesystem
- client.list_receipts: List and filter receipts
- client.record_work_async / client.verify_receipt_async: Async versions

This aligns with Goal Grid P2-T7: Produce a 10-15 line quickstart example.
"""

from northroot import Client
import tempfile
import os

# Create a client with temporary storage for this example
temp_dir = tempfile.mkdtemp()
client = Client(storage_path=temp_dir)
print(f"Using temporary storage: {temp_dir}")

# Example 1: Record a simple unit of work
print("\n1. Recording work")
receipt1 = client.record_work(
    workload_id="normalize-prices",
    payload={"input_hash": "sha256:abc123...", "output_hash": "sha256:def456..."},
    tags=["etl", "batch"],
    trace_id="trace-2025-11-15",
    parent_id=None,
)

print(f"Created receipt: {receipt1.get_rid()}")
print(f"Hash: {receipt1.get_hash()}")

# Store the receipt
client.store_receipt(receipt1)
print("Stored receipt to filesystem")

# Example 2: Verify receipt
print("\n2. Verifying receipt")
is_valid = client.verify_receipt(receipt1)
print(f"Receipt is valid: {is_valid}")

# Example 3: Create a DAG with parent-child relationship
print("\n3. Creating DAG with parent-child")
receipt2 = client.record_work(
    workload_id="aggregate-totals",
    payload={"input_receipt": receipt1.get_rid(), "result": "sum"},
    tags=["etl"],
    trace_id="trace-2025-11-15",  # Same trace
    parent_id=receipt1.get_rid(),  # Parent link
)

print(f"Child receipt: {receipt2.get_rid()}")
print(f"Parent receipt: {receipt1.get_rid()}")

# Store the child receipt
client.store_receipt(receipt2)

# Verify both receipts
print(f"\nParent valid: {client.verify_receipt(receipt1)}")
print(f"Child valid: {client.verify_receipt(receipt2)}")

# Example 3.5: List receipts
print("\n3.5. Listing receipts")
all_receipts = client.list_receipts()
print(f"Total receipts: {len(all_receipts)}")

# Filter by workload_id
normalize_receipts = client.list_receipts(workload_id="normalize-prices")
print(f"Receipts with workload_id='normalize-prices': {len(normalize_receipts)}")

# Filter by trace_id
trace_receipts = client.list_receipts(trace_id="trace-2025-11-15")
print(f"Receipts with trace_id='trace-2025-11-15': {len(trace_receipts)}")

# Example 4: Async API (optional)
print("\n4. Async API")
import asyncio

async def async_example():
    receipt_async = await client.record_work_async(
        workload_id="async-example",
        payload={"async": True},
        tags=["async"]
)
    print(f"Async receipt: {receipt_async.get_rid()}")
    is_valid_async = await client.verify_receipt_async(receipt_async)
    print(f"Async receipt valid: {is_valid_async}")

asyncio.run(async_example())

# Cleanup
print(f"\nCleaning up temporary storage: {temp_dir}")
import shutil
shutil.rmtree(temp_dir)

print("\n✅ Quickstart complete!")
