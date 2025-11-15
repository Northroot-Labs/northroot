#!/usr/bin/env python3
"""
Northroot SDK Quickstart Example

This example demonstrates the minimal v0.1 API:
- record_work: Create verifiable receipts for units of work
- verify_receipt: Verify receipt integrity

This aligns with Harada P2-T7: Produce a 10-15 line quickstart example.
"""

import northroot_sdk

# Example 1: Record a simple unit of work
print("Example 1: Recording work")
receipt1 = northroot_sdk.receipts.record_work(
    workload_id="normalize-prices",
    payload={"input_hash": "sha256:abc123...", "output_hash": "sha256:def456..."},
    tags=["etl", "batch"],
    trace_id="trace-2025-01-17",
    parent_id=None,
)

print(f"Created receipt: {receipt1.get_rid()}")
print(f"Hash: {receipt1.get_hash()}")

# Example 2: Verify receipt
print("\nExample 2: Verifying receipt")
is_valid = northroot_sdk.receipts.verify_receipt(receipt1)
print(f"Receipt is valid: {is_valid}")

# Example 3: Create a DAG with parent-child relationship
print("\nExample 3: Creating DAG with parent-child")
receipt2 = northroot_sdk.receipts.record_work(
    workload_id="aggregate-totals",
    payload={"input_receipt": receipt1.get_rid(), "result": "sum"},
    tags=["etl"],
    trace_id="trace-2025-01-17",  # Same trace
    parent_id=receipt1.get_rid(),  # Parent link
)

print(f"Child receipt: {receipt2.get_rid()}")
print(f"Parent receipt: {receipt1.get_rid()}")

# Verify both receipts
print(f"\nParent valid: {northroot_sdk.receipts.verify_receipt(receipt1)}")
print(f"Child valid: {northroot_sdk.receipts.verify_receipt(receipt2)}")

print("\n✅ Quickstart complete!")

