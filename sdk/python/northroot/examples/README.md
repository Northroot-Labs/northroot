# Northroot SDK Examples

## receipts_vs_logging.py

A comprehensive demo comparing traditional logging with verifiable receipts.

**What it demonstrates:**
- Traditional logging approach (simple, but no integrity)
- Verifiable receipts (tamper-evident, cryptographically verifiable)
- Tamper detection (shows how receipts detect modifications)
- Querying receipts (by trace_id, workload_id)
- Composition (chaining receipts via dom/cod)
- Storage comparison (logs vs receipts)

**Run it:**
```bash
cd sdk/python/northroot
source venv/bin/activate
python examples/receipts_vs_logging.py
```

**Output:**
- Creates `demo.log` (traditional logging)
- Creates `receipts_demo/` directory with stored receipts
- Shows side-by-side comparison of approaches

## quickstart.py

Minimal quickstart example showing basic SDK usage.

## hello_receipts.py

Simplest possible demo: 3 steps → 3 receipts.

