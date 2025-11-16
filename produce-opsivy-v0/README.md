# Produce Operations Ivy v0

**Codename:** `produce-opsivy-v0`  
**Base:** northroot v0.1.0-alpha  
**Status:** Private repository for produce distributor operations

## Overview

This is the private repository for produce distributor operations, built on top of northroot v0.1.0-alpha. It provides:

- **Canonical Data Model**: SQLite schema for field operations, loads, storage, quality, and sensors
- **Event-Driven Architecture**: JSON payload shapes that map 1:1 to database tables
- **Verifiable Receipts**: All events recorded as cryptographic receipts via northroot SDK
- **Traceability**: Full supply chain traceability from field to storage to quality samples

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Application Layer (REST/gRPC API, QR Codes, etc.)      │
└─────────────────────────────────────────────────────────┘
                        │
┌─────────────────────────────────────────────────────────┐
│  Event Layer (Canonical JSON Payloads)                  │
│  - load_intake                                          │
│  - storage_placement                                    │
│  - quality_sample                                       │
└─────────────────────────────────────────────────────────┘
                        │
        ┌───────────────┴───────────────┐
        │                                 │
┌───────────────────┐         ┌───────────────────┐
│  Database Layer   │         │  Receipt Layer    │
│  (SQLite Schema)  │         │  (northroot SDK)  │
└───────────────────┘         └───────────────────┘
```

## Core Events

### 1. Load Intake Event

Records when a load arrives at the facility.

**Database:** `loads` table (+ optional `load_field_contributions`)

**JSON Payload:**
```json
{
  "event_type": "load_intake",
  "load_id": "LOAD-2025-00123",
  "grower_id": "GROWER-102",
  "field_id": "FIELD-47",
  "field_zone_id": "ZONE-47A",
  "variety_id": "VAR-RUSSET-B123",
  "harvest_date": "2025-09-28T14:30:00Z",
  "gross_weight_lbs": 42000.0,
  "tare_weight_lbs": 12000.0,
  "net_weight_lbs": 30000.0,
  "destination_type": "storage",
  "destination_id": "BIN-07",
  "truck_id": "TRUCK-14",
  "ticket_number": "TICKET-88341",
  "source_system": "field-ops-ivy",
  "external_id": null,
  "created_at": "2025-09-28T15:00:00Z",
  "notes": "Late-day harvest; slightly wet"
}
```

**SDK Call:**
```python
receipt = client.record_work(
    workload_id="load_intake",
    payload=load_event_json,
    trace_id=load_event_json["load_id"],
    tags=["load", "intake"],
)
```

### 2. Storage Placement Event

Records when a load is placed into storage.

**Database:** `storage_placements` table

**JSON Payload:**
```json
{
  "event_type": "storage_placement",
  "placement_id": "PLAC-2025-00091",
  "load_id": "LOAD-2025-00123",
  "storage_unit_id": "BIN-07",
  "event_type_detail": "inbound",
  "quantity_lbs": 30000.0,
  "previous_storage_unit_id": null,
  "operator_id": "OP-22",
  "event_timestamp": "2025-09-28T16:10:00Z",
  "source_system": "field-ops-ivy",
  "external_id": null,
  "notes": "Full load into BIN-07",
  "created_at": "2025-09-28T16:11:05Z"
}
```

**SDK Call:**
```python
receipt = client.record_work(
    workload_id="storage_placement",
    payload=placement_event_json,
    trace_id=placement_event_json["load_id"],
    parent_id=load_intake_receipt.rid,
    tags=["storage", placement_event_json["event_type_detail"]],
)
```

### 3. Quality Sample Event

Records quality measurements for a load.

**Database:** `quality_samples` table

**JSON Payload:**
```json
{
  "event_type": "quality_sample",
  "sample_id": "SAMPLE-2025-00451",
  "load_id": "LOAD-2025-00123",
  "storage_unit_id": "BIN-07",
  "sample_timestamp": "2025-10-15T09:20:00Z",
  "sample_stage": "storage",
  "sample_method": "core",
  "bruise_pct": 19.5,
  "rot_pct": 3.2,
  "glucose_pct": 1.1,
  "sucrose_pct": 2.3,
  "dry_matter_pct": 20.8,
  "pulp_temp_c": 6.2,
  "ambient_temp_c": 5.5,
  "lab_id": "LAB-01",
  "technician_id": "TECH-07",
  "source_system": "field-ops-ivy",
  "external_id": null,
  "notes": "Bruise slightly high; borderline for processor X",
  "created_at": "2025-10-15T09:30:00Z"
}
```

**SDK Call:**
```python
receipt = client.record_work(
    workload_id="quality_sample",
    payload=quality_event_json,
    trace_id=quality_event_json["load_id"],
    parent_id=storage_placement_receipt.rid,
    tags=["quality", quality_event_json["sample_stage"]],
)
```

## Data Model Alignment

The architecture ensures:

- **SQLite schema** is fixed and inspectable
- **JSON event shapes** mirror the tables 1:1
- **Northroot SDK** sees the same canonical payload the DB stores
- **Future features** (QR codes, UX, API) can be added without touching the core data model

## Receipt Chaining

Events are chained via `parent_id` to create a verifiable trace:

```
load_intake (receipt A)
    └── storage_placement (receipt B, parent=A)
            └── quality_sample (receipt C, parent=B)
```

All receipts share the same `trace_id` (the `load_id`) for querying the complete lifecycle.

## Dependencies

- **northroot v0.1.0-alpha**: Core receipt engine (pinned)
- **SQLite**: Database backend
- **Python 3.9+**: Runtime environment

## Next Steps

1. **API Layer**: Build REST/gRPC server wrapping event recording
2. **QR Codes**: Generate QR codes encoding receipt IDs/hashes
3. **Authentication**: Add multi-tenant support
4. **Reporting**: Build analytics on top of receipt data
5. **Integration**: Connect to external systems (croptrack, coolform, etc.)

## Versioning

- **Base**: northroot v0.1.0-alpha (frozen)
- **This Repo**: Independent versioning starting at v0.1.0
- **Migration**: Update northroot dependency when v0.1.0 stable is released

