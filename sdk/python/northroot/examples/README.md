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

## produce_operations.py

Demonstrates verifiable receipts for agricultural supply chain operations (potatoes specifically).

**What it demonstrates:**
- Semantic workload_id patterns (harvest_planted, harvest_executed, harvest_outcome, load_shipped, etc.)
- Supply chain lifecycle tracking (harvest → shipping → delivery)
- Receipt chaining via parent_id and trace_id
- Querying by workload_id and trace_id
- Clean data at ingestion

**Run it:**
```bash
cd sdk/python/northroot
source venv/bin/activate
python examples/produce_operations.py
```

**Output:**
- Creates receipts for harvest lifecycle (intent → execution → outcome)
- Creates receipts for shipping lifecycle (preparation → shipment → delivery)
- Demonstrates querying patterns
- Shows three-phase pattern for supply chain events

## produce_telemetry.py

Demonstrates time-series receipt chaining for agricultural telemetry data.

**What it demonstrates:**
- Sensor data collection (soil moisture, temperature, humidity)
- Equipment telemetry (harvester GPS, fuel consumption)
- Weather data integration
- Time-series receipt chaining
- Querying patterns for telemetry data

**Run it:**
```bash
cd sdk/python/northroot
source venv/bin/activate
python examples/produce_telemetry.py
```

**Output:**
- Creates time-series chains of sensor readings
- Creates equipment telemetry receipts
- Creates weather data receipts
- Demonstrates querying by workload_id

