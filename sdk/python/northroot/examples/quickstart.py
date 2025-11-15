#!/usr/bin/env python3
"""
Northroot SDK Quickstart Example

This example demonstrates the minimal v0.1 API with the thin client:
- record_work: Create verifiable receipts for units of work
- verify_receipt: Verify receipt integrity
- Client class: Optional client wrapper (storage decoupled)

This aligns with Harada P2-T7: Produce a 10-15 line quickstart example.
"""

from northroot import Client
import northroot as nr

# --- Thin Client API (Recommended) ---
# Simple, ergonomic API - no need for 'nr.receipts.record_work'
print("=== Thin Client API ===")

# Example 1: Record a simple unit of work
print("\n1. Recording work")
receipt1 = nr.record_work(
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
is_valid = nr.verify_receipt(receipt1)
print(f"Receipt is valid: {is_valid}")

# Example 3: Create a DAG with parent-child relationship
print("\n3. Creating DAG with parent-child")
receipt2 = nr.record_work(
    workload_id="aggregate-totals",
    payload={"input_receipt": receipt1.get_rid(), "result": "sum"},
    tags=["etl"],
    trace_id="trace-2025-01-17",  # Same trace
    parent_id=receipt1.get_rid(),  # Parent link
)

print(f"Child receipt: {receipt2.get_rid()}")
print(f"Parent receipt: {receipt1.get_rid()}")

# Verify both receipts
print(f"\nParent valid: {nr.verify_receipt(receipt1)}")
print(f"Child valid: {nr.verify_receipt(receipt2)}")

# --- Client Class (Optional) ---
# The Client class provides the same API but can be extended with storage later
# For v0.1, storage is decoupled and optional
print("\n=== Client Class (Optional) ===")
client = Client()  # No storage (storage is decoupled)
# client = Client(storage_path="./receipts")  # With filesystem storage (future)

receipt_via_client = client.record_work(
    workload_id="client-example",
    payload={"example": "data"},
    tags=["demo"]
)
print(f"Client receipt: {receipt_via_client.get_rid()}")
print(f"Client receipt valid: {client.verify_receipt(receipt_via_client)}")

# --- Async API (Optional) ---
print("\n=== Async API ===")
import asyncio

async def async_example():
    # Async versions run sync functions in a thread pool
    receipt_async = await nr.record_work_async(
        workload_id="async-example",
        payload={"async": True},
        tags=["async"]
    )
    print(f"Async receipt: {receipt_async.get_rid()}")
    is_valid_async = await nr.verify_receipt_async(receipt_async)
    print(f"Async receipt valid: {is_valid_async}")

# Run async example
asyncio.run(async_example())

print("\n✅ Quickstart complete!")
