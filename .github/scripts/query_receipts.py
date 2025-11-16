#!/usr/bin/env python3
"""Query receipts by run_id for debugging and investigation."""
import json
import os
import sys
from pathlib import Path

def query_receipts(run_id: str, storage_path: str = None):
    """Query all receipts for a given run_id."""
    try:
        from northroot import Client
    except ImportError:
        print("ERROR: northroot not installed")
        print("Install with: pip install northroot")
        return None
    
    # Use provided storage or default
    if storage_path:
        client = Client(storage_path=storage_path)
    else:
        # Try to find receipts in artifacts
        client = Client()
    
    trace_id = f"run-{run_id}"
    print(f"Querying receipts for trace_id: {trace_id}")
    
    receipts = client.list_receipts(trace_id=trace_id)
    
    if not receipts:
        print(f"❌ No receipts found for run_id: {run_id}")
        return []
    
    print(f"✅ Found {len(receipts)} receipt(s)")
    print()
    
    for i, receipt in enumerate(receipts, 1):
        rid = receipt.get_rid() if hasattr(receipt, 'get_rid') else "unknown"
        is_valid = client.verify_receipt(receipt) if hasattr(client, 'verify_receipt') else None
        
        print(f"Receipt {i}:")
        print(f"  RID: {rid}")
        print(f"  Valid: {'✅' if is_valid else '❌' if is_valid is False else '?'}")
        
        # Try to get payload info
        try:
            if hasattr(receipt, 'get_payload'):
                payload = receipt.get_payload()
                if isinstance(payload, dict):
                    step = payload.get('step', 'unknown')
                    status = payload.get('status', 'unknown')
                    print(f"  Step: {step}")
                    print(f"  Status: {status}")
        except:
            pass
        
        print()
    
    return receipts

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: query_receipts.py <run_id> [storage_path]")
        print("Example: query_receipts.py 19402527077")
        sys.exit(1)
    
    run_id = sys.argv[1]
    storage_path = sys.argv[2] if len(sys.argv) > 2 else None
    
    query_receipts(run_id, storage_path)

