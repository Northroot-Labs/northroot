#!/usr/bin/env python3
"""Minimal receipt recording for GitHub Actions workflows."""
import json
import os
import sys
from pathlib import Path

def record_step(workload_id: str, step_name: str, status: str, metadata: dict = None):
    """Record a workflow step as a receipt."""
    try:
        from northroot import Client
    except ImportError:
        print("⚠️  northroot not installed, skipping receipt recording")
        return None
    
    run_id = os.environ.get("GITHUB_RUN_ID", "unknown")
    commit_sha = os.environ.get("GITHUB_SHA", "unknown")
    workflow = os.environ.get("GITHUB_WORKFLOW", "unknown")
    job_name = os.environ.get("GITHUB_JOB", "unknown")
    runner_os = os.environ.get("RUNNER_OS", os.environ.get("ImageOS", "unknown"))
    
    client = Client()
    
    payload = {
        "step": step_name,
        "status": status,
        "workflow": workflow,
        "job": job_name,
        "commit_sha": commit_sha,
        "run_id": run_id,
        "runner_os": runner_os,
        **(metadata or {})
    }
    
    receipt = client.record_work(
        workload_id=workload_id,
        payload=payload,
        trace_id=f"run-{run_id}",  # All receipts in this run share trace_id
        tags=["ci", "github-actions", workflow, job_name]
    )
    
    # Store receipt JSON for artifact upload
    receipt_dir = Path(os.environ.get("GITHUB_STEP_SUMMARY", "./receipts"))
    receipt_dir = receipt_dir.parent / "receipts"  # Use receipts/ subdirectory
    receipt_dir.mkdir(parents=True, exist_ok=True)
    
    receipt_file = receipt_dir / f"{workload_id}-{receipt.get_rid()}.json"
    with open(receipt_file, "w") as f:
        # Use the receipt's to_json() method
        try:
            receipt_json_str = receipt.to_json()
            receipt_dict = json.loads(receipt_json_str)
        except Exception as e:
            # Fallback: create minimal receipt record
            print(f"⚠️  Could not serialize receipt to JSON: {e}")
            receipt_dict = {
                "rid": str(receipt.get_rid()),
                "hash": receipt.get_hash() if hasattr(receipt, 'get_hash') else "unknown",
                "trace_id": f"run-{run_id}",
                "workload_id": workload_id,
                "payload": payload
            }
        json.dump(receipt_dict, f, indent=2)
    
    print(f"✅ Recorded receipt: {receipt.get_rid()}")
    print(f"📄 Saved to: {receipt_file}")
    print(f"🔗 Trace ID: run-{run_id}")
    
    return receipt.get_rid()

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: record_receipt.py <workload_id> <step_name> <status> [metadata_json]")
        sys.exit(1)
    
    workload_id = sys.argv[1]
    step_name = sys.argv[2]
    status = sys.argv[3]
    metadata = json.loads(sys.argv[4]) if len(sys.argv) > 4 else None
    
    record_step(workload_id, step_name, status, metadata)

