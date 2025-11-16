# Canonical JSON Payload Shapes

This document defines the canonical JSON payload shapes for the three core events. These payloads are used both for:

1. **Database insertion** (1:1 mapping to SQL tables)
2. **Northroot receipt creation** (via `client.record_work()`)

## Design Principles

- **1:1 Mapping**: JSON fields map directly to database columns
- **Canonical Format**: Same payload used for DB and receipts
- **Traceability**: All events share `trace_id` (typically the `load_id`)
- **Composition**: Events chain via `parent_id` in receipts

---

## 1. Load Intake Event

**Table:** `loads` (+ optional `load_field_contributions`)

**Workload ID:** `load_intake`

**Trace ID:** `load_id` (e.g., `"LOAD-2025-00123"`)

**Parent ID:** `None` (root event)

### Payload Shape

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

### SDK Usage

```python
receipt = client.record_work(
    workload_id="load_intake",
    payload=load_event_json,
    trace_id=load_event_json["load_id"],
    tags=["load", "intake"],
)
```

### Database Mapping

| JSON Field | DB Column | Notes |
|------------|-----------|-------|
| `load_id` | `load_id` | PRIMARY KEY |
| `grower_id` | `grower_id` | FOREIGN KEY |
| `field_id` | `field_id` | FOREIGN KEY, nullable |
| `field_zone_id` | `field_zone_id` | FOREIGN KEY, nullable |
| `variety_id` | `variety_id` | FOREIGN KEY, nullable |
| `harvest_date` | `harvest_date` | ISO-8601 string |
| `gross_weight_lbs` | `gross_weight_lbs` | REAL |
| `tare_weight_lbs` | `tare_weight_lbs` | REAL |
| `net_weight_lbs` | `net_weight_lbs` | REAL |
| `destination_type` | `destination_type` | TEXT |
| `destination_id` | `destination_id` | TEXT, nullable |
| `truck_id` | `truck_id` | TEXT, nullable |
| `ticket_number` | `ticket_number` | TEXT, nullable |
| `source_system` | `source_system` | TEXT |
| `external_id` | `external_id` | TEXT, nullable |
| `created_at` | `created_at` | ISO-8601 string |
| `notes` | `notes` | TEXT, nullable |

### Mixed Load Handling

If the load contains contributions from multiple fields, include a `field_contributions` array:

```json
{
  "event_type": "load_intake",
  "load_id": "LOAD-2025-00123",
  // ... other fields ...
  "field_contributions": [
    {
      "contribution_id": "CONT-001",
      "field_id": "FIELD-47",
      "field_zone_id": "ZONE-47A",
      "variety_id": "VAR-RUSSET-B123",
      "contribution_weight_lbs": 20000.0,
      "contribution_pct": 66.67
    },
    {
      "contribution_id": "CONT-002",
      "field_id": "FIELD-48",
      "field_zone_id": null,
      "variety_id": "VAR-RUSSET-B123",
      "contribution_weight_lbs": 10000.0,
      "contribution_pct": 33.33
    }
  ]
}
```

This array is then inserted into `load_field_contributions` table separately.

---

## 2. Storage Placement Event

**Table:** `storage_placements`

**Workload ID:** `storage_placement`

**Trace ID:** `load_id` (same as load_intake)

**Parent ID:** `load_intake` receipt `rid`

### Payload Shape

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

### SDK Usage

```python
receipt = client.record_work(
    workload_id="storage_placement",
    payload=placement_event_json,
    trace_id=placement_event_json["load_id"],
    parent_id=load_intake_receipt.rid,  # Chain to load_intake
    tags=["storage", placement_event_json["event_type_detail"]],
)
```

### Database Mapping

| JSON Field | DB Column | Notes |
|------------|-----------|-------|
| `placement_id` | `placement_id` | PRIMARY KEY |
| `load_id` | `load_id` | FOREIGN KEY |
| `storage_unit_id` | `storage_unit_id` | FOREIGN KEY |
| `event_type_detail` | `event_type` | Maps to DB `event_type` |
| `quantity_lbs` | `quantity_lbs` | REAL, nullable |
| `previous_storage_unit_id` | `previous_storage_unit_id` | TEXT, nullable |
| `operator_id` | `operator_id` | TEXT, nullable |
| `event_timestamp` | `event_timestamp` | ISO-8601 string |
| `source_system` | `source_system` | TEXT |
| `external_id` | `external_id` | TEXT, nullable |
| `notes` | `notes` | TEXT, nullable |

### Event Types

- `"inbound"`: Load placed into storage
- `"move"`: Load moved between storage units
- `"outbound"`: Load removed from storage

---

## 3. Quality Sample Event

**Table:** `quality_samples`

**Workload ID:** `quality_sample`

**Trace ID:** `load_id` (same as load_intake)

**Parent ID:** `storage_placement` receipt `rid` (or `load_intake` if no placement)

### Payload Shape

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

### SDK Usage

```python
receipt = client.record_work(
    workload_id="quality_sample",
    payload=quality_event_json,
    trace_id=quality_event_json["load_id"],
    parent_id=storage_placement_receipt.rid,  # Chain to storage_placement
    tags=["quality", quality_event_json["sample_stage"]],
)
```

### Database Mapping

| JSON Field | DB Column | Notes |
|------------|-----------|-------|
| `sample_id` | `sample_id` | PRIMARY KEY |
| `load_id` | `load_id` | FOREIGN KEY |
| `storage_unit_id` | `storage_unit_id` | FOREIGN KEY, nullable |
| `sample_timestamp` | `sample_timestamp` | ISO-8601 string |
| `sample_stage` | `sample_stage` | TEXT |
| `sample_method` | `sample_method` | TEXT, nullable |
| `bruise_pct` | `bruise_pct` | REAL, nullable |
| `rot_pct` | `rot_pct` | REAL, nullable |
| `glucose_pct` | `glucose_pct` | REAL, nullable |
| `sucrose_pct` | `sucrose_pct` | REAL, nullable |
| `dry_matter_pct` | `dry_matter_pct` | REAL, nullable |
| `pulp_temp_c` | `pulp_temp_c` | REAL, nullable |
| `ambient_temp_c` | `ambient_temp_c` | REAL, nullable |
| `lab_id` | `lab_id` | TEXT, nullable |
| `technician_id` | `technician_id` | TEXT, nullable |
| `source_system` | `source_system` | TEXT |
| `external_id` | `external_id` | TEXT, nullable |
| `notes` | `notes` | TEXT, nullable |

### Sample Stages

- `"intake"`: Sample taken at load intake
- `"storage"`: Sample taken during storage
- `"pre_shipment"`: Sample taken before shipping
- `"processor"`: Sample taken at processor

---

## Receipt Chaining Pattern

All three events share the same `trace_id` (the `load_id`) and chain via `parent_id`:

```
load_intake (receipt A)
    trace_id: "LOAD-2025-00123"
    parent_id: None
    
    └── storage_placement (receipt B)
        trace_id: "LOAD-2025-00123"
        parent_id: receipt A.rid
        
        └── quality_sample (receipt C)
            trace_id: "LOAD-2025-00123"
            parent_id: receipt B.rid
```

### Querying Complete Trace

```python
# Get all receipts for a load
all_receipts = client.list_receipts(trace_id="LOAD-2025-00123")

# Verify all receipts in chain
for receipt in all_receipts:
    assert client.verify_receipt(receipt)
```

---

## Common Fields

All events include these common fields:

- `event_type`: Event type identifier
- `source_system`: System that generated the event (e.g., `"field-ops-ivy"`)
- `external_id`: Optional external system ID
- `created_at`: ISO-8601 timestamp of event creation
- `notes`: Optional free-text notes

These fields ensure traceability and integration with external systems.

