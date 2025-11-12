use jsonschema::JSONSchema;
use northroot_receipts::adapters::json;
use northroot_receipts::*;
use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Load a JSON file from the given path.
fn load(path: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// Load and compile a JSON schema, using robust path resolution.
///
/// Uses compile-time path resolution via `env!` macro when possible,
/// falling back to runtime resolution via `CARGO_MANIFEST_DIR` or relative paths.
fn schema(id: &str) -> JSONSchema {
    // Build schema path (schemas are now in ../../schemas/receipts/)
    let schema_path = if id.starts_with("schemas/") {
        format!("../../{}", id)
    } else {
        format!("../../schemas/receipts/{}", id)
    };

    // Try multiple path resolutions (most robust first)
    let sch: Value = {
        // First, try using compile-time manifest dir (most reliable)
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let full_path = PathBuf::from(manifest_dir).join(&schema_path);
        if full_path.exists() {
            load(full_path.to_str().unwrap())
        } else if let Ok(content) = fs::read_to_string(&schema_path) {
            // Try relative path from current directory
            serde_json::from_str(&content).unwrap()
        } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            // Fallback to runtime manifest dir
            let full_path = PathBuf::from(manifest_dir).join(&schema_path);
            load(full_path.to_str().unwrap())
        } else {
            panic!("Cannot find schema file: {}", schema_path);
        }
    };

    JSONSchema::compile(&sch).unwrap()
}

/// Validate a payload against a schema, collecting all errors immediately.
///
/// This helper function ensures errors are collected before lifetimes expire,
/// fixing jsonschema 0.18 API lifetime issues.
fn validate_payload_against_schema(schema: &JSONSchema, payload: &Value, kind: &str) {
    match schema.validate(payload) {
        Ok(()) => {}
        Err(errors) => {
            // Collect errors immediately to avoid lifetime issues
            let error_vec: Vec<_> = errors.collect();
            panic!("{} payload validation failed: {:?}", kind, error_vec);
        }
    }
}

#[test]
fn vectors_validate_payloads() {
    // point to repo root when running from receipts/ with `cargo test`
    let base = Path::new("../../vectors");

    // Per-kind payload schemas (note: actual filenames use _schema.json, not .schema.json)
    let ds = schema("data_shape_schema.json");
    let ms = schema("method_shape_schema.json");
    let rs = schema("reasoning_shape_schema.json");
    let ex = schema("execution_schema.json");
    let sp = schema("spend_schema.json");
    let st = schema("settlement_schema.json");

    // Load full receipts then validate just the payloads against their schema
    let data_shape = load(base.join("data_shape.json").to_str().unwrap());
    let method_shape = load(base.join("method_shape.json").to_str().unwrap());
    let reasoning_shape = load(base.join("reasoning_shape.json").to_str().unwrap());
    let execution = load(base.join("execution.json").to_str().unwrap());
    let spend = load(base.join("spend.json").to_str().unwrap());
    let settlement = load(base.join("settlement.json").to_str().unwrap());

    // Extract payloads (clone to own the values)
    let data_shape_payload = data_shape.get("payload").unwrap().clone();
    let method_shape_payload = method_shape.get("payload").unwrap().clone();
    let reasoning_shape_payload = reasoning_shape.get("payload").unwrap().clone();
    let execution_payload = execution.get("payload").unwrap().clone();
    let spend_payload = spend.get("payload").unwrap().clone();
    let settlement_payload = settlement.get("payload").unwrap().clone();

    // Validate payloads using helper function to avoid lifetime issues
    validate_payload_against_schema(&ds, &data_shape_payload, "DataShape");
    validate_payload_against_schema(&ms, &method_shape_payload, "MethodShape");
    validate_payload_against_schema(&rs, &reasoning_shape_payload, "ReasoningShape");
    validate_payload_against_schema(&ex, &execution_payload, "Execution");
    validate_payload_against_schema(&sp, &spend_payload, "Spend");
    validate_payload_against_schema(&st, &settlement_payload, "Settlement");
}

#[test]
fn vectors_validate_full_receipts() {
    let base = Path::new("../../vectors");

    let vectors = [
        base.join("data_shape.json"),
        base.join("method_shape.json"),
        base.join("reasoning_shape.json"),
        base.join("execution.json"),
        base.join("spend.json"),
        base.join("settlement.json"),
    ];

    for path in &vectors {
        let json_str = fs::read_to_string(path).unwrap();
        // Test vectors now have CBOR-based hashes
        let receipt = json::receipt_from_json(&json_str)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));

        // Validate full receipt structure
        receipt
            .validate()
            .unwrap_or_else(|e| panic!("Validation failed for {}: {}", path.display(), e));
    }
}

#[test]
fn test_invalid_payload_rejected() {
    let _base = Path::new("../vectors");
    let schema = schema("data_shape_schema.json");

    // Invalid payload: missing required schema_hash
    let invalid_payload = serde_json::json!({});

    // Validate and check for errors (collect immediately to avoid lifetime issues)
    let is_err = match schema.validate(&invalid_payload) {
        Ok(()) => false,
        Err(errors) => {
            let _error_vec: Vec<_> = errors.collect(); // Consume iterator
            true
        }
    };
    assert!(is_err, "Expected validation errors for invalid payload");
}

#[test]
fn test_missing_required_fields() {
    let _base = Path::new("../vectors");
    let schema = schema("execution_schema.json");

    // Missing required field: trace_id
    let invalid_payload = serde_json::json!({
        "method_ref": {
            "method_id": "test",
            "version": "1.0.0",
            "method_shape_root": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        },
        "data_shape_hash": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "span_commitments": [],
        "roots": {
            "trace_set_root": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "identity_root": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        }
    });

    // Validate and check for errors (collect immediately to avoid lifetime issues)
    let is_err = match schema.validate(&invalid_payload) {
        Ok(()) => false,
        Err(errors) => {
            let _error_vec: Vec<_> = errors.collect(); // Consume iterator
            true
        }
    };
    assert!(
        is_err,
        "Expected validation errors for missing required field"
    );
}
