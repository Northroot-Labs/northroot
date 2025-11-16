# Receipt Integration Plan for GitHub Actions

**Status:** Phase 1 (Minimal) - Current  
**Goal:** Systematic, production-ready receipt recording for CI/CD workflows

---

## Current State (Phase 1)

✅ **What Works:**
- Receipt recording at critical steps (build start/complete)
- Receipts stored as artifacts (90-day retention)
- Query script for debugging by `run_id`
- Graceful fallback if northroot unavailable

⚠️ **Limitations:**
- No persistent storage (artifacts only)
- Manual query process
- No failure investigation automation
- No cross-run analysis

---

## Recommended Architecture

### Phase 1: Minimal (Current) ✅
**Goal:** Prove concept, minimal overhead

**Implementation:**
- Record receipts at 3-4 critical steps per workflow
- Store as artifacts (no infrastructure)
- Manual query via script

**Steps to Record:**
1. Workflow start (`workflow-start`)
2. Build completion (`build-complete`)
3. Test completion (`test-complete`) - if applicable
4. Publish completion (`publish-complete`)

**Storage:** GitHub Artifacts (90-day retention)

---

### Phase 2: Persistent Storage (Recommended Next)
**Goal:** Enable cross-run analysis and automated investigation

**Implementation:**
- Use filesystem storage in workflow runner
- Upload receipts to persistent storage (S3, GCS, or GitHub Releases)
- Or: Use SQLite backend for querying

**Storage Options:**

**Option A: GitHub Releases (Simplest)**
```yaml
- name: Upload receipts to release
  if: github.event_name == 'release'
  uses: softprops/action-gh-release@v1
  with:
    files: receipts/*.json
```

**Option B: S3/GCS (Scalable)**
```yaml
- name: Upload receipts to S3
  run: |
    aws s3 sync receipts/ s3://northroot-receipts/runs/${{ github.run_id }}/
```

**Option C: SQLite Backend (Queryable)**
```python
# In workflow
client = Client(storage_path="./receipts.db")
receipt = client.record_work(...)
client.store_receipt(receipt)  # Stores in SQLite

# Query later
receipts = client.list_receipts(trace_id=f"run-{run_id}")
```

---

### Phase 3: Automated Investigation (Future)
**Goal:** Automatic failure analysis and alerts

**Components:**
1. **Webhook Handler** - Triggered on workflow completion
2. **Receipt Analyzer** - Queries receipts, identifies failure points
3. **Alert System** - Notifies on failures with receipt chain

**Implementation:**
```python
# .github/webhooks/workflow_webhook.py
@app.route("/webhook/workflow", methods=["POST"])
def handle_workflow_event():
    event = request.json
    run_id = event["workflow_run"]["id"]
    
    # Query all receipts for this run
    client = Client(storage_path="./receipts.db")
    receipts = client.list_receipts(trace_id=f"run-{run_id}")
    
    # Analyze failure chain
    if event["workflow_run"]["conclusion"] == "failure":
        failed_steps = analyze_failure_chain(receipts)
        send_alert(run_id, failed_steps)
```

---

## Concrete Implementation Plan

### Step 1: Improve Current Script (Immediate)

**Enhance `record_receipt.py`:**
- Add parent_id tracking for DAG composition
- Better error handling and logging
- Support for storage_path configuration

**Enhance `query_receipts.py`:**
- Support querying from artifacts (download first)
- Support querying from persistent storage
- Better output formatting

### Step 2: Standardize Workflow Integration

**Create reusable workflow step:**
```yaml
# .github/workflows/reusable-record-receipt.yml
- name: Record receipt
  uses: ./.github/actions/record-receipt
  with:
    workload_id: ${{ inputs.workload_id }}
    step_name: ${{ inputs.step_name }}
    status: ${{ job.status }}
    metadata: ${{ inputs.metadata }}
```

**Benefits:**
- Consistent receipt recording across workflows
- Single source of truth for receipt format
- Easier to maintain and update

### Step 3: Add Persistent Storage (Recommended)

**Option: SQLite Backend**
```yaml
- name: Setup receipt storage
  run: |
    mkdir -p ${{ runner.temp }}/receipts
    echo "RECEIPT_STORAGE=${{ runner.temp }}/receipts" >> $GITHUB_ENV

- name: Record receipt
  env:
    RECEIPT_STORAGE: ${{ env.RECEIPT_STORAGE }}
  run: |
    python .github/scripts/record_receipt.py \
      --storage "$RECEIPT_STORAGE" \
      "build-wheel" "build-start" "${{ job.status }}"

- name: Upload receipt database
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: receipts-db-run-${{ github.run_id }}
    path: ${{ env.RECEIPT_STORAGE }}/*.db
```

**Benefits:**
- Queryable via `list_receipts()`
- Faster than parsing JSON artifacts
- Enables cross-run analysis

### Step 4: Add Failure Investigation (Future)

**Create investigation script:**
```python
# .github/scripts/investigate_failure.py
def investigate_failure(run_id: str):
    client = Client(storage_path="./receipts.db")
    receipts = client.list_receipts(trace_id=f"run-{run_id}")
    
    # Find failure point
    for receipt in receipts:
        if receipt.payload.get("status") == "failure":
            print(f"❌ Failure at: {receipt.payload['step']}")
            print(f"   RID: {receipt.get_rid()}")
            print(f"   Hash: {receipt.get_hash()}")
            # Verify receipt integrity
            is_valid = client.verify_receipt(receipt)
            print(f"   Valid: {is_valid}")
```

---

## Recommended Next Steps

### Immediate (This Week)
1. ✅ Fix receipt script path (done)
2. ✅ Test workflow run (in progress)
3. **Enhance scripts** with better error handling
4. **Add parent_id tracking** for receipt chains

### Short-term (Next Sprint)
1. **Add SQLite storage option** for queryable receipts
2. **Standardize receipt format** across all workflows
3. **Create reusable workflow action** for receipt recording
4. **Document receipt query patterns**

### Long-term (Future)
1. **Webhook handler** for automated failure investigation
2. **Receipt dashboard** for visualizing workflow chains
3. **Cross-run analysis** (compare receipts across runs)
4. **Integration with monitoring** (Slack, PagerDuty, etc.)

---

## Decision Points

### Storage Strategy
**Recommendation:** Start with artifacts, upgrade to SQLite when needed

**Rationale:**
- Artifacts: Zero infrastructure, works immediately
- SQLite: Queryable, enables automation, still simple
- S3/GCS: Only needed for very high volume or long-term retention

### Which Steps to Record
**Recommendation:** Critical path only (3-4 steps per workflow)

**Rationale:**
- Too many receipts = overhead
- Too few = not useful for debugging
- Focus on: start, major milestones, completion

### Receipt Granularity
**Recommendation:** One receipt per logical step, not per command

**Rationale:**
- Each GitHub Actions step = one receipt
- Group related commands into single step
- Use metadata to capture details

---

## Example: Complete Workflow Integration

```yaml
jobs:
  build:
    steps:
      - name: Setup receipt storage
        run: |
          mkdir -p ./receipts
          echo "RECEIPT_STORAGE=./receipts" >> $GITHUB_ENV
      
      - name: Record workflow start
        run: |
          python .github/scripts/record_receipt.py \
            --storage "$RECEIPT_STORAGE" \
            "workflow" "start" "in_progress" \
            '{"workflow": "${{ github.workflow }}", "run_id": "${{ github.run_id }}"}'
      
      - name: Build
        run: |
          # ... build commands ...
          BUILD_STATUS="success"
      
      - name: Record build completion
        if: always()
        run: |
          python .github/scripts/record_receipt.py \
            --storage "$RECEIPT_STORAGE" \
            "build" "complete" "${{ job.status }}" \
            '{"build_status": "'"$BUILD_STATUS"'"}'
      
      - name: Upload receipts
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: receipts-run-${{ github.run_id }}
          path: ${{ env.RECEIPT_STORAGE }}/*
```

---

## Success Metrics

**Phase 1 (Current):**
- ✅ Receipts recorded at critical steps
- ✅ Receipts queryable by run_id
- ✅ Zero workflow failures due to receipts

**Phase 2 (Next):**
- Receipts queryable from persistent storage
- Failure investigation script works
- Receipts used for debugging at least once

**Phase 3 (Future):**
- Automated failure alerts with receipt chains
- Receipt dashboard showing workflow health
- Cross-run analysis identifying patterns

