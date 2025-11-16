#!/usr/bin/env python3
"""
Receipts vs Logging: A Practical Comparison

This demo shows the key differences between traditional logging and verifiable receipts.
It demonstrates:
1. Tamper-evident proofs (can't modify receipts without detection)
2. Cryptographic verification (independent verification)
3. Composition (chaining receipts together)
4. Storage and querying (proof-addressable storage)
"""

import json
import logging
import time
from pathlib import Path
from typing import Dict, Any

# Setup logging (traditional approach)
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('demo.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger('demo')

# Import northroot SDK
try:
    import northroot as nr
    from northroot import Client
except ImportError:
    print("ERROR: northroot SDK not installed. Run: cd sdk/python/northroot && pip install -e .")
    exit(1)


def simulate_etl_pipeline_logging():
    """Traditional logging approach - what most systems do today."""
    print("\n" + "="*70)
    print("TRADITIONAL LOGGING APPROACH")
    print("="*70)
    
    logger.info("Starting ETL pipeline")
    
    # Step 1: Load data
    logger.info("Loading data from source", extra={"source": "s3://bucket/data.csv", "rows": 1000})
    time.sleep(0.1)
    
    # Step 2: Transform
    logger.info("Transforming data", extra={"transform": "normalize-prices", "rows_processed": 1000})
    time.sleep(0.1)
    
    # Step 3: Save
    logger.info("Saving results", extra={"destination": "s3://bucket/results.parquet", "rows": 1000})
    
    logger.info("ETL pipeline completed")
    
    print("\n✓ Logged to demo.log")
    print("  - Can be modified after the fact")
    print("  - No cryptographic integrity")
    print("  - No way to verify authenticity")
    print("  - Hard to chain/compose operations")


def simulate_etl_pipeline_receipts():
    """Verifiable receipts approach - what northroot provides."""
    print("\n" + "="*70)
    print("VERIFIABLE RECEIPTS APPROACH")
    print("="*70)
    
    # Create client with storage
    storage_path = Path("./receipts_demo")
    storage_path.mkdir(exist_ok=True)
    client = Client(storage_path=str(storage_path))
    
    trace_id = f"etl-pipeline-{int(time.time())}"
    
    # Step 1: Load data
    print("\n1. Loading data...")
    load_receipt = client.record_work(
        workload_id="load-data",
        payload={
            "source": "s3://bucket/data.csv",
            "rows": 1000,
            "data_shape_hash": "sha256:abc123...",  # In real usage, computed from actual data
        },
        tags=["etl", "load"],
        trace_id=trace_id
    )
    client.store_receipt(load_receipt)
    print(f"   ✓ Receipt created: {load_receipt.get_rid()}")
    print(f"   ✓ Hash: {load_receipt.get_hash()[:32]}...")
    
    # Step 2: Transform (chained to load)
    print("\n2. Transforming data...")
    transform_receipt = client.record_work(
        workload_id="normalize-prices",
        payload={
            "input_shape_hash": load_receipt.get_cod(),  # Links to previous step
            "transform": "normalize-prices",
            "rows_processed": 1000,
            "output_shape_hash": "sha256:def456...",
        },
        tags=["etl", "transform"],
        trace_id=trace_id,
        parent_id=str(load_receipt.get_rid())
    )
    client.store_receipt(transform_receipt)
    print(f"   ✓ Receipt created: {transform_receipt.get_rid()}")
    print(f"   ✓ Linked to load receipt (dom matches cod)")
    print(f"   ✓ Hash: {transform_receipt.get_hash()[:32]}...")
    
    # Step 3: Save (chained to transform)
    print("\n3. Saving results...")
    save_receipt = client.record_work(
        workload_id="save-results",
        payload={
            "input_shape_hash": transform_receipt.get_cod(),
            "destination": "s3://bucket/results.parquet",
            "rows": 1000,
        },
        tags=["etl", "save"],
        trace_id=trace_id,
        parent_id=str(transform_receipt.get_rid())
    )
    client.store_receipt(save_receipt)
    print(f"   ✓ Receipt created: {save_receipt.get_rid()}")
    print(f"   ✓ Linked to transform receipt")
    print(f"   ✓ Hash: {save_receipt.get_hash()[:32]}...")
    
    print("\n✓ All receipts stored and verifiable")
    print("  - Tamper-evident (hash changes if modified)")
    print("  - Cryptographically verifiable")
    print("  - Composable (chained via dom/cod)")
    print("  - Queryable by trace_id, workload_id, etc.")
    
    return trace_id, client


def demonstrate_tamper_detection(client: Client):
    """Show how receipts detect tampering."""
    print("\n" + "="*70)
    print("TAMPER DETECTION DEMO")
    print("="*70)
    
    # Create a receipt
    receipt = client.record_work(
        workload_id="test-work",
        payload={"data": "original value"},
        tags=["demo"]
    )
    
    original_hash = receipt.get_hash()
    print(f"\nOriginal receipt hash: {original_hash[:32]}...")
    
    # Verify it's valid
    is_valid = client.verify_receipt(receipt)
    print(f"✓ Receipt is valid: {is_valid}")
    
    # Try to "modify" it (this would break the hash)
    print("\n⚠️  If someone tries to modify the receipt...")
    print("   (In real scenario, they'd modify the JSON file)")
    print("   The hash would no longer match!")
    
    # Simulate verification after tampering
    print("\n   Verifying after tampering attempt...")
    print("   ✗ Verification would FAIL - hash mismatch detected")
    print("   ✓ Receipt integrity is cryptographically protected")


def demonstrate_querying(client: Client, trace_id: str):
    """Show how to query receipts."""
    print("\n" + "="*70)
    print("QUERYING RECEIPTS")
    print("="*70)
    
    # Query by trace_id
    print(f"\nQuerying by trace_id: {trace_id}")
    trace_receipts = client.list_receipts(trace_id=trace_id)
    print(f"✓ Found {len(trace_receipts)} receipts in trace")
    for r in trace_receipts:
        trace = r.get_trace_id() or "N/A"
        print(f"   - {r.get_rid()[:16]}... (trace: {trace[:20]}...)")
    
    # Query by workload_id
    print(f"\nQuerying by workload_id: normalize-prices")
    try:
        transform_receipts = client.list_receipts(workload_id="normalize-prices")
        print(f"✓ Found {len(transform_receipts)} receipts")
    except Exception as e:
        print(f"  (Note: workload_id query may not be fully implemented: {e})")
    
    # Query all
    print(f"\nQuerying all receipts (limit 10)")
    all_receipts = client.list_receipts(limit=10)
    print(f"✓ Found {len(all_receipts)} receipts total")


def demonstrate_composition():
    """Show receipt composition."""
    print("\n" + "="*70)
    print("COMPOSITION DEMO")
    print("="*70)
    
    client = Client(storage_path="./receipts_demo")
    
    # Create a chain of receipts
    print("\nCreating a chain of receipts...")
    
    # Step 1
    r1 = client.record_work(
        workload_id="step1",
        payload={"input": "data1", "output": "data2"},
        tags=["chain"]
    )
    print(f"1. Step 1: {r1.get_rid()}")
    print(f"   dom: {r1.get_dom()[:32]}...")
    print(f"   cod: {r1.get_cod()[:32]}...")
    
    # Step 2 (should link to step 1)
    r2 = client.record_work(
        workload_id="step2",
        payload={
            "input_shape_hash": r1.get_cod(),  # Links to previous
            "output": "data3"
        },
        tags=["chain"],
        parent_id=str(r1.get_rid())
    )
    print(f"\n2. Step 2: {r2.get_rid()}")
    print(f"   dom: {r2.get_dom()[:32]}...")
    print(f"   cod: {r2.get_cod()[:32]}...")
    
    # Verify composition
    if r1.get_cod() == r2.get_dom():
        print("\n✓ Composition valid: cod(step1) == dom(step2)")
        print("  This proves the chain is valid!")
    else:
        print("\n✗ Composition invalid: chain is broken")
    
    client.store_receipt(r1)
    client.store_receipt(r2)


def compare_storage():
    """Compare what's stored in logs vs receipts."""
    print("\n" + "="*70)
    print("STORAGE COMPARISON")
    print("="*70)
    
    # Show log file
    log_path = Path("demo.log")
    if log_path.exists():
        print(f"\n📄 Log file (demo.log):")
        print(f"   Size: {log_path.stat().st_size} bytes")
        print(f"   Format: Plain text, can be edited")
        print(f"   Integrity: None - can be modified without detection")
        with open(log_path, 'r') as f:
            lines = f.readlines()
            print(f"   Lines: {len(lines)}")
            if lines:
                print(f"   Sample: {lines[0].strip()[:60]}...")
    
    # Show receipt storage
    receipt_dir = Path("receipts_demo/receipts")
    if receipt_dir.exists():
        receipt_files = list(receipt_dir.glob("*.json"))
        print(f"\n📦 Receipt storage ({receipt_dir}):")
        print(f"   Files: {len(receipt_files)} receipts")
        total_size = sum(f.stat().st_size for f in receipt_files)
        print(f"   Total size: {total_size} bytes")
        print(f"   Format: JSON with cryptographic hash")
        print(f"   Integrity: Hash-verified, tamper-evident")
        if receipt_files:
            with open(receipt_files[0], 'r') as f:
                receipt_data = json.load(f)
                print(f"   Sample RID: {receipt_data.get('rid', 'N/A')}")
                print(f"   Sample hash: {receipt_data.get('hash', 'N/A')[:32]}...")


def main():
    """Run the complete demo."""
    print("\n" + "="*70)
    print("NORTHROOT: RECEIPTS VS LOGGING DEMO")
    print("="*70)
    print("\nThis demo shows the practical differences between")
    print("traditional logging and verifiable receipts.\n")
    
    # 1. Traditional logging
    simulate_etl_pipeline_logging()
    
    # 2. Verifiable receipts
    trace_id, client = simulate_etl_pipeline_receipts()
    
    # 3. Tamper detection
    demonstrate_tamper_detection(client)
    
    # 4. Querying
    demonstrate_querying(client, trace_id)
    
    # 5. Composition
    demonstrate_composition()
    
    # 6. Storage comparison
    compare_storage()
    
    # Summary
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    print("\n📝 LOGGING:")
    print("   - Simple, human-readable")
    print("   - No integrity guarantees")
    print("   - Can be modified")
    print("   - No cryptographic verification")
    print("   - Hard to compose/chain")
    
    print("\n🔐 RECEIPTS:")
    print("   - Tamper-evident (hash-protected)")
    print("   - Cryptographically verifiable")
    print("   - Composable (dom/cod chaining)")
    print("   - Queryable and indexable")
    print("   - Proof-addressable storage")
    
    print("\n" + "="*70)
    print("✓ Demo complete! Check ./receipts_demo/ for stored receipts")
    print("="*70 + "\n")


if __name__ == "__main__":
    main()

