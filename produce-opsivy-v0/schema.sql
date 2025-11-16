-- field-ops-ivy-v0 canonical schema

-- SQLite-compatible

----------------------------------------------------------------------
-- 0. PRAGMAS (optional, often set in app code instead of schema)
----------------------------------------------------------------------

PRAGMA foreign_keys = ON;

----------------------------------------------------------------------
-- 1. Reference Tables
----------------------------------------------------------------------

CREATE TABLE varieties (
    variety_id      TEXT PRIMARY KEY,         -- e.g. "VAR-RUSSET-B123"
    name            TEXT NOT NULL,
    seed_supplier   TEXT,
    seed_lot_id     TEXT,
    notes           TEXT
);

CREATE TABLE growers (
    grower_id       TEXT PRIMARY KEY,         -- e.g. "GROWER-102"
    farm_name       TEXT NOT NULL,
    contact_name    TEXT,
    contact_phone   TEXT,
    contact_email   TEXT
);

CREATE TABLE fields (
    field_id        TEXT PRIMARY KEY,         -- e.g. "FIELD-47"
    grower_id       TEXT NOT NULL,
    field_name      TEXT NOT NULL,
    acreage         REAL,
    gps_polygon     TEXT,                     -- GeoJSON/WKT as TEXT
    soil_type       TEXT,
    notes           TEXT,
    FOREIGN KEY (grower_id) REFERENCES growers(grower_id)
);

CREATE TABLE field_zones (
    zone_id         TEXT PRIMARY KEY,         -- e.g. "ZONE-47A"
    field_id        TEXT NOT NULL,
    zone_name       TEXT NOT NULL,
    acreage         REAL,
    notes           TEXT,
    FOREIGN KEY (field_id) REFERENCES fields(field_id)
);

----------------------------------------------------------------------
-- 2. Seed + Crop Inputs
----------------------------------------------------------------------

CREATE TABLE seed_allocations (
    allocation_id       TEXT PRIMARY KEY,
    variety_id          TEXT NOT NULL,
    grower_id           TEXT NOT NULL,
    field_id            TEXT NOT NULL,
    field_zone_id       TEXT,                 -- optional
    quantity_lbs        REAL NOT NULL,
    planting_date       TEXT,                 -- ISO-8601 string
    source_system       TEXT NOT NULL,        -- "manual" | "croptrack" | "coolform" | ...
    external_id         TEXT,                 -- upstream system ID
    notes               TEXT,
    FOREIGN KEY (variety_id)    REFERENCES varieties(variety_id),
    FOREIGN KEY (grower_id)     REFERENCES growers(grower_id),
    FOREIGN KEY (field_id)      REFERENCES fields(field_id),
    FOREIGN KEY (field_zone_id) REFERENCES field_zones(zone_id)
);

CREATE TABLE crop_inputs (
    input_id            TEXT PRIMARY KEY,
    field_id            TEXT NOT NULL,
    field_zone_id       TEXT,                 -- optional
    chemical_name       TEXT NOT NULL,
    rate_per_acre       REAL,                 -- nullable if unknown
    unit                TEXT,                 -- "lbs_acre", "gal_acre", ...
    application_date    TEXT NOT NULL,        -- ISO-8601 string
    applicator_id       TEXT,
    source_system       TEXT NOT NULL,
    external_id         TEXT,
    notes               TEXT,
    FOREIGN KEY (field_id)      REFERENCES fields(field_id),
    FOREIGN KEY (field_zone_id) REFERENCES field_zones(zone_id)
);

----------------------------------------------------------------------
-- 3. Loads + Field Contributions
----------------------------------------------------------------------

CREATE TABLE loads (
    load_id             TEXT PRIMARY KEY,     -- e.g. "LOAD-2025-00123"
    grower_id           TEXT NOT NULL,
    field_id            TEXT,                 -- may be null if mixed; see contributions
    field_zone_id       TEXT,
    variety_id          TEXT,
    harvest_date        TEXT NOT NULL,        -- ISO-8601 string
    gross_weight_lbs    REAL NOT NULL,
    tare_weight_lbs     REAL NOT NULL,
    net_weight_lbs      REAL NOT NULL,
    destination_type    TEXT NOT NULL,        -- "storage" | "processor" | "dump" | "other"
    destination_id      TEXT,                 -- storage_unit_id or processor code
    truck_id            TEXT,
    ticket_number       TEXT,
    source_system       TEXT NOT NULL,
    external_id         TEXT,
    created_at          TEXT NOT NULL,        -- ISO-8601 string
    notes               TEXT,
    FOREIGN KEY (grower_id)     REFERENCES growers(grower_id),
    FOREIGN KEY (field_id)      REFERENCES fields(field_id),
    FOREIGN KEY (field_zone_id) REFERENCES field_zones(zone_id),
    FOREIGN KEY (variety_id)    REFERENCES varieties(variety_id)
);

CREATE TABLE load_field_contributions (
    contribution_id             TEXT PRIMARY KEY,
    load_id                     TEXT NOT NULL,
    field_id                    TEXT NOT NULL,
    field_zone_id               TEXT,
    variety_id                  TEXT,
    contribution_weight_lbs     REAL,         -- one of weight or pct can be null
    contribution_pct            REAL,         -- 0–100, nullable
    harvest_pass_id             TEXT,
    source_system               TEXT NOT NULL,
    external_id                 TEXT,
    notes                       TEXT,
    FOREIGN KEY (load_id)       REFERENCES loads(load_id),
    FOREIGN KEY (field_id)      REFERENCES fields(field_id),
    FOREIGN KEY (field_zone_id) REFERENCES field_zones(zone_id),
    FOREIGN KEY (variety_id)    REFERENCES varieties(variety_id)
);

----------------------------------------------------------------------
-- 4. Storage
----------------------------------------------------------------------

CREATE TABLE storage_units (
    storage_unit_id     TEXT PRIMARY KEY,     -- e.g. "BIN-07" or "ROOM-3"
    warehouse_id        TEXT NOT NULL,        -- site identifier
    description         TEXT,
    capacity_lbs        REAL,
    notes               TEXT
);

CREATE TABLE storage_placements (
    placement_id            TEXT PRIMARY KEY,
    load_id                 TEXT NOT NULL,
    storage_unit_id         TEXT NOT NULL,
    event_type              TEXT NOT NULL,    -- "inbound" | "move" | "outbound"
    quantity_lbs            REAL,             -- if partial, else full load
    previous_storage_unit_id TEXT,
    operator_id             TEXT,
    event_timestamp         TEXT NOT NULL,    -- ISO-8601
    source_system           TEXT NOT NULL,
    external_id             TEXT,
    notes                   TEXT,
    FOREIGN KEY (load_id)          REFERENCES loads(load_id),
    FOREIGN KEY (storage_unit_id)  REFERENCES storage_units(storage_unit_id)
);

----------------------------------------------------------------------
-- 5. Quality + Sensors
----------------------------------------------------------------------

CREATE TABLE quality_samples (
    sample_id           TEXT PRIMARY KEY,
    load_id             TEXT NOT NULL,
    storage_unit_id     TEXT,
    sample_timestamp    TEXT NOT NULL,        -- ISO-8601
    sample_stage        TEXT NOT NULL,        -- "intake" | "storage" | "pre_shipment" | "processor"
    sample_method       TEXT,                 -- "core" | "grab" | etc.

    bruise_pct          REAL,
    rot_pct             REAL,
    glucose_pct         REAL,
    sucrose_pct         REAL,
    dry_matter_pct      REAL,

    pulp_temp_c         REAL,
    ambient_temp_c      REAL,

    lab_id              TEXT,
    technician_id       TEXT,

    source_system       TEXT NOT NULL,
    external_id         TEXT,
    notes               TEXT,

    FOREIGN KEY (load_id)         REFERENCES loads(load_id),
    FOREIGN KEY (storage_unit_id) REFERENCES storage_units(storage_unit_id)
);

CREATE TABLE sensor_readings (
    reading_id          TEXT PRIMARY KEY,
    source_type         TEXT NOT NULL,        -- "storage" | "field" | "truck"
    storage_unit_id     TEXT,
    field_id            TEXT,
    field_zone_id       TEXT,
    truck_id            TEXT,

    sensor_type         TEXT NOT NULL,        -- "temperature_c" | "humidity_pct" | "co2_ppm" | ...
    value               REAL NOT NULL,
    unit                TEXT NOT NULL,

    device_id           TEXT NOT NULL,
    reading_timestamp   TEXT NOT NULL,        -- sensor's timestamp
    created_at          TEXT NOT NULL,        -- ingest timestamp

    raw_payload         TEXT,                 -- vendor JSON as TEXT
    source_system       TEXT NOT NULL,

    FOREIGN KEY (storage_unit_id) REFERENCES storage_units(storage_unit_id),
    FOREIGN KEY (field_id)        REFERENCES fields(field_id),
    FOREIGN KEY (field_zone_id)   REFERENCES field_zones(zone_id)
);

----------------------------------------------------------------------
-- 6. Indexes for queries + analytics
----------------------------------------------------------------------

CREATE INDEX idx_loads_field_zone
    ON loads(field_zone_id);

CREATE INDEX idx_loads_variety
    ON loads(variety_id);

CREATE INDEX idx_storage_placements_load
    ON storage_placements(load_id);

CREATE INDEX idx_storage_placements_unit
    ON storage_placements(storage_unit_id);

CREATE INDEX idx_quality_samples_load
    ON quality_samples(load_id);

CREATE INDEX idx_quality_samples_unit
    ON quality_samples(storage_unit_id);

CREATE INDEX idx_sensor_readings_unit_time
    ON sensor_readings(storage_unit_id, reading_timestamp);

