#!/usr/bin/env python3
"""
Northroot SDK Quickstart Example

This example demonstrates the minimal v0.1 API using the Client class:
- client.record_work: Create verifiable receipts for units of work
- client.verify_receipt: Verify receipt integrity
- client.record_work_async / client.verify_receipt_async: Async versions

This aligns with Harada P2-T7: Produce a 10-15 line quickstart example.
"""

from northroot import Client

# Create a client (storage is decoupled and optional for v0.1)
client = Client()
# client = Client(storage_path="./receipts")  # With filesystem storage (future)

# Example 1: Record a simple unit of work
print("1. Recording work")
receipt1 = client.record_work(
    workload_id="normalize-prices",
    payload={"input_hash": "sha256:abc123...", "output_hash": "sha256:def456..."},
    tags=["etl", "batch"],
    trace_id="trace-2025-01-17",
    parent_id=None,
)

print(f"Created receipt: {receipt1.get_rid()}")
print(f"Hash: {receipt1.get_hash()}")

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
    trace_id="trace-2025-01-17",  # Same trace
    parent_id=receipt1.get_rid(),  # Parent link
)

print(f"Child receipt: {receipt2.get_rid()}")
print(f"Parent receipt: {receipt1.get_rid()}")

# Verify both receipts
print(f"\nParent valid: {client.verify_receipt(receipt1)}")
print(f"Child valid: {client.verify_receipt(receipt2)}")

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

print("\n✅ Quickstart complete!")
