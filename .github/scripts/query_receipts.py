#!/usr/bin/env python3
"""Query receipts by run_id for debugging and investigation."""
import json
import os
import sys
from pathlib import Path

def query_receipts(run_id: str, storage_path: str = None, from_artifacts: str = None):
    """Query all receipts for a given run_id.
    
    Args:
        run_id: GitHub Actions run ID
        storage_path: Path to receipt storage (SQLite DB or filesystem)
        from_artifacts: Path to downloaded artifacts directory
    """
    try:
        from northroot import Client  # type: ignore[import-untyped]
    except ImportError:
        print("ERROR: northroot not installed")
        print("Install with: pip install northroot")
        return None
    
    # Use provided storage or try to find from artifacts
    if storage_path:
        client = Client(storage_path=storage_path)
    elif from_artifacts:
        # Load receipts from JSON files in artifacts
        import glob
        receipt_files = glob.glob(f"{from_artifacts}/**/*.json", recursive=True)
        if not receipt_files:
            print(f"❌ No receipt JSON files found in {from_artifacts}")
            return []
        
        print(f"📥 Loading {len(receipt_files)} receipt(s) from artifacts...")
        receipts = []
        for receipt_file in receipt_files:
            try:
                with open(receipt_file, 'r') as f:
                    receipt_data = json.load(f)
                    # Create receipt from JSON using northroot SDK
                    receipt_json_str = json.dumps(receipt_data)
                    try:
                        from northroot.receipts import receipt_from_json  # type: ignore[import-untyped]
                        receipt = receipt_from_json(receipt_json_str)
                        receipts.append(receipt)
                    except ImportError:
                        # Fallback: use simple wrapper if import fails
                        class ReceiptWrapper:
                            def __init__(self, data):
                                self.data = data
                            def get_rid(self):
                                return self.data.get("rid", "unknown")
                            def get_trace_id(self):
                                payload = self.data.get("payload", {})
                                if isinstance(payload, dict):
                                    return payload.get("trace_id")
                                return None
                            def get_hash(self):
                                return self.data.get("hash", "unknown")
                        receipt = ReceiptWrapper(receipt_data)
                        receipts.append(receipt)
            except Exception as e:
                print(f"⚠️  Could not load {receipt_file}: {e}")
        
        # Filter by trace_id
        trace_id = f"run-{run_id}"
        filtered = [r for r in receipts if r.get_trace_id() == trace_id]
        return filtered
    else:
        # Try to find receipts in current directory or default storage
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
    import argparse
    
    parser = argparse.ArgumentParser(description="Query receipts by run_id")
    parser.add_argument("run_id", help="GitHub Actions run ID")
    parser.add_argument("--storage", help="Storage path (SQLite DB or filesystem)")
    parser.add_argument("--artifacts", help="Path to downloaded artifacts directory")
    
    args = parser.parse_args()
    
    receipts = query_receipts(args.run_id, storage_path=args.storage, from_artifacts=args.artifacts)
    
    if receipts:
        print(f"\n✅ Found {len(receipts)} receipt(s) for run {args.run_id}")
    else:
        print(f"\n❌ No receipts found for run {args.run_id}")

