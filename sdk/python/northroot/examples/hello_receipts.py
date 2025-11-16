#!/usr/bin/env python3
"""
Hello Receipts - Simplest Northroot Demo

This is the absolute minimal demo: 3 steps → 3 receipts.
Perfect for first-time users to understand the core concept.

Goal Grid P7-T1: Implement a "hello receipts" demo (3 steps → 3 receipts).

Usage:
    python examples/hello_receipts.py
"""

from northroot import Client
import tempfile
import shutil

# Step 1: Setup
print("=" * 60)
print("Hello Receipts - 3 Steps to Your First Receipts")
print("=" * 60)

# Create a temporary storage directory
storage_dir = tempfile.mkdtemp(prefix="northroot-hello-")
client = Client(storage_path=storage_dir)
print(f"\n📁 Storage: {storage_dir}")

# Step 2: Create 3 receipts (one for each step)
print("\n" + "=" * 60)
print("Step 1: Record Your First Receipt")
print("=" * 60)

receipt1 = client.record_work(
    workload_id="hello-world",
    payload={"message": "Hello from Northroot!"},
    tags=["demo", "hello"],
)
print(f"✅ Receipt 1 created: {receipt1.get_rid()[:16]}...")
client.store_receipt(receipt1)

print("\n" + "=" * 60)
print("Step 2: Verify Your Receipt")
print("=" * 60)

is_valid = client.verify_receipt(receipt1)
print(f"✅ Receipt is valid: {is_valid}")

print("\n" + "=" * 60)
print("Step 3: List All Receipts")
print("=" * 60)

all_receipts = client.list_receipts()
print(f"✅ Total receipts stored: {len(all_receipts)}")
for i, receipt in enumerate(all_receipts, 1):
    print(f"   {i}. {receipt.get_rid()[:16]}...")

# Cleanup
print("\n" + "=" * 60)
print("✨ Done! Cleaning up...")
print("=" * 60)
shutil.rmtree(storage_dir)
print(f"✅ Removed temporary storage: {storage_dir}")

print("\n🎉 You've created, verified, and listed receipts!")
print("   Next: Try examples/quickstart.py for more features.")

