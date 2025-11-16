# Receipt Collection Architecture for CI/CD

## The Challenge

**Problem:** GitHub Actions runners are ephemeral. When a workflow completes, the runner is destroyed. How do we collect receipts that are created during workflow execution?

**Constraints:**
- Runners are temporary (destroyed after job completion)
- Receipts are created on the runner's filesystem
- We need persistent storage accessible after workflow completes
- Must handle failures gracefully (receipts shouldn't break workflows)

---

## Pragmatic Solution: Multi-Layer Approach

### Layer 1: Local Storage (Runner Filesystem)
**Purpose:** Immediate storage during workflow execution

```python
# On runner
client = Client(storage_path="./receipts")
receipt = client.record_work(...)
client.store_receipt(receipt)  # Stores to ./receipts/*.db or ./receipts/*.json
```

**Pros:**
- Fast, local access
- Enables querying during workflow (`list_receipts()`)
- No network dependency

**Cons:**
- Ephemeral - lost when runner is destroyed
- Not accessible after workflow completes

---

### Layer 2: Artifact Upload (GitHub Artifacts)
**Purpose:** Backup and retrieval after workflow completes

```yaml
- name: Upload receipts
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: receipts-run-${{ github.run_id }}
    path: receipts/*
    retention-days: 90
```

**Pros:**
- Automatic backup
- Accessible via GitHub UI/API
- 90-day retention (configurable)
- Zero infrastructure

**Cons:**
- Requires manual download
- Not queryable (just files)
- Limited retention

---

### Layer 3: Webhook/API (Optional - Real-time)
**Purpose:** Real-time receipt ingestion to persistent database

```yaml
- name: Send receipt to webhook
  if: always()
  run: |
    python3 << 'PYEOF'
    import json
    import os
    import urllib.request
    
    receipt_file = "receipts/latest.json"
    if os.path.exists(receipt_file):
        with open(receipt_file) as f:
            receipt_data = json.load(f)
        
        webhook_url = os.environ.get('RECEIPT_WEBHOOK_URL')
        if webhook_url:
            req = urllib.request.Request(
                webhook_url,
                data=json.dumps(receipt_data).encode(),
                headers={'Content-Type': 'application/json'}
            )
            urllib.request.urlopen(req)
    PYEOF
  env:
    RECEIPT_WEBHOOK_URL: ${{ secrets.RECEIPT_WEBHOOK_URL }}
  continue-on-error: true
```

**Pros:**
- Real-time ingestion
- Queryable database
- Long-term storage
- Cross-run analysis

**Cons:**
- Requires infrastructure (webhook server, database)
- Network dependency
- More complex setup

---

## Recommended Approach: Start Simple, Scale Up

### Phase 1: Artifacts Only (Current)
**Best for:** Getting started, minimal infrastructure

```yaml
steps:
  - name: Record receipt
    run: |
      python3 << 'PYEOF'
      from northroot import Client
      client = Client(storage_path="./receipts")
      receipt = client.record_work(...)
      client.store_receipt(receipt)
      PYEOF
  
  - name: Upload receipts
    if: always()
    uses: actions/upload-artifact@v4
    with:
      name: receipts-run-${{ github.run_id }}
      path: receipts/*
```

**When to use:**
- Initial implementation
- Low-volume workflows
- Manual investigation

---

### Phase 2: Artifacts + Webhook (Recommended)
**Best for:** Production use, automated analysis

```yaml
steps:
  - name: Record receipt
    run: |
      python3 << 'PYEOF'
      from northroot import Client
      import json
      import os
      
      client = Client(storage_path="./receipts")
      receipt = client.record_work(...)
      client.store_receipt(receipt)
      
      # Also send to webhook if configured
      webhook_url = os.environ.get('RECEIPT_WEBHOOK_URL')
      if webhook_url:
          receipt_json = receipt.to_json()
          # Send to webhook (async, non-blocking)
          # ... webhook code ...
      PYEOF
    env:
      RECEIPT_WEBHOOK_URL: ${{ secrets.RECEIPT_WEBHOOK_URL }}
    continue-on-error: true
  
  - name: Upload receipts (backup)
    if: always()
    uses: actions/upload-artifact@v4
    with:
      name: receipts-run-${{ github.run_id }}
      path: receipts/*
```

**When to use:**
- Production workflows
- Need for automated analysis
- Cross-run correlation

---

## Handling Failures

### Principle: Receipts Never Break Workflows

```python
try:
    from northroot import Client
    client = Client(storage_path="./receipts")
    receipt = client.record_work(...)
    client.store_receipt(receipt)
    print('[OK] Receipt recorded')
except ImportError:
    print('[SKIP] northroot not available')
    sys.exit(0)  # Success - graceful degradation
except Exception as e:
    print(f'[WARN] Receipt failed: {e}')
    sys.exit(0)  # Success - don't break workflow
```

**Key points:**
- Always exit 0 (success) on receipt errors
- Use `continue-on-error: true` in workflow
- Log warnings, don't fail

---

## Workflow Lifecycle

### 1. Workflow Start (Intent)
```python
# Record: "We intend to run this workflow"
receipt = client.record_work(
    workload_id="workflow-start",
    payload={
        "workflow": "pypi-publish",
        "run_id": run_id,
        "commit": commit_sha,
        "trigger": "manual"  # or "push", "schedule", etc.
    },
    trace_id=f"run-{run_id}"
)
```

### 2. Job Steps (Execution)
```python
# Record: "This step completed"
receipt = client.record_work(
    workload_id="build-wheel",
    payload={
        "step": "build-complete",
        "status": "success",
        "platform": "linux",
        "duration_sec": 120
    },
    trace_id=f"run-{run_id}",
    parent_id=previous_receipt.get_rid()  # Chain receipts
)
```

### 3. Workflow End (Outcome)
```python
# Record: "Workflow completed with this outcome"
receipt = client.record_work(
    workload_id="workflow-complete",
    payload={
        "status": "success",
        "total_duration_sec": 600,
        "jobs_completed": 3,
        "artifacts_uploaded": 5
    },
    trace_id=f"run-{run_id}"
)
```

---

## Querying Receipts

### From Artifacts (Manual)
```bash
# Download artifact
gh run download <run_id> -n receipts-run-<run_id>

# Query locally
python3 -c "
from northroot import Client
client = Client(storage_path='./receipts')
receipts = client.list_receipts(trace_id='run-<run_id>')
for r in receipts:
    print(r.get_rid())
"
```

### From Database (Automated)
```python
# If webhook stores to database
from northroot import Client
client = Client(storage_path="postgresql://...")  # Future: DB backend
receipts = client.list_receipts(trace_id="run-12345")
```

---

## Implementation Checklist

### Immediate (Fix Current Issues)
- [x] Fix `northroot` installation (ensure it installs from PyPI)
- [x] Use inline Python (no script files)
- [x] Graceful degradation (exit 0 on errors)
- [x] ASCII-only output (no Unicode issues)

### Short-term (Make It Work)
- [ ] Verify receipts are stored to filesystem
- [ ] Verify artifacts are uploaded
- [ ] Test receipt querying from artifacts
- [ ] Document artifact download/query process

### Medium-term (Make It Production-Ready)
- [ ] Add webhook support (optional)
- [ ] Create receipt query script
- [ ] Add receipt verification step
- [ ] Document workflow integration patterns

### Long-term (Make It Scalable)
- [ ] Database backend for receipts
- [ ] Receipt dashboard/UI
- [ ] Automated failure analysis
- [ ] Cross-run correlation

---

## Decision Matrix

| Requirement | Artifacts | Webhook | Database |
|------------|-----------|---------|----------|
| Zero infrastructure | ✅ | ❌ | ❌ |
| Real-time access | ❌ | ✅ | ✅ |
| Queryable | ❌ | ✅ | ✅ |
| Long-term storage | ❌ | ✅ | ✅ |
| Easy setup | ✅ | ⚠️ | ❌ |
| Cross-run analysis | ❌ | ✅ | ✅ |

**Recommendation:** Start with artifacts, add webhook when needed.

---

## Example: Complete Workflow Integration

```yaml
name: Example with Receipts

on: [push]

jobs:
  build:
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      
      - name: Install northroot
        run: pip install northroot
        continue-on-error: true
      
      - name: Setup receipt storage
        run: mkdir -p ./receipts
      
      - name: Record workflow start
        if: always()
        run: |
          python3 << 'PYEOF'
          import os
          from northroot import Client
          client = Client(storage_path="./receipts")
          receipt = client.record_work(
              workload_id="workflow-start",
              payload={"run_id": os.environ.get("GITHUB_RUN_ID")},
              trace_id=f"run-{os.environ.get('GITHUB_RUN_ID')}"
          )
          client.store_receipt(receipt)
          print(f"[OK] {receipt.get_rid()}")
          PYEOF
        continue-on-error: true
      
      - name: Build
        run: echo "Building..."
      
      - name: Record build complete
        if: always()
        run: |
          python3 << 'PYEOF'
          import os
          from northroot import Client
          client = Client(storage_path="./receipts")
          receipt = client.record_work(
              workload_id="build",
              payload={"status": os.environ.get("JOB_STATUS")},
              trace_id=f"run-{os.environ.get('GITHUB_RUN_ID')}"
          )
          client.store_receipt(receipt)
          PYEOF
        env:
          JOB_STATUS: ${{ job.status }}
        continue-on-error: true
      
      - name: Upload receipts
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: receipts-run-${{ github.run_id }}
          path: receipts/*
          retention-days: 90
```

---

## Next Steps

1. **Fix immediate issue:** Ensure `northroot` installs correctly
2. **Verify storage:** Confirm receipts are written to filesystem
3. **Test artifacts:** Download and query receipts from artifacts
4. **Document patterns:** Create reusable workflow snippets
5. **Consider webhook:** If real-time access is needed

