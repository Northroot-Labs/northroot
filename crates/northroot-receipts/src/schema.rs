//! JSON Schema validation for receipts (feature-gated).
//!
//! This module provides runtime JSON Schema validation when the `jsonschema` feature is enabled.
//! It's used for validating untrusted input at API boundaries.

#[cfg(feature = "jsonschema")]
use crate::error::ValidationError;
#[cfg(feature = "jsonschema")]
use crate::Receipt;
#[cfg(feature = "jsonschema")]
use jsonschema::JSONSchema;
#[cfg(feature = "jsonschema")]
use serde_json::Value;
#[cfg(feature = "jsonschema")]
use std::env;
#[cfg(feature = "jsonschema")]
use std::fs;
#[cfg(feature = "jsonschema")]
use std::path::PathBuf;

#[cfg(feature = "jsonschema")]
/// Load and compile a JSON schema from the schemas directory.
///
/// Schemas are located at the workspace root: `schemas/receipts/`
/// From the crate manifest directory, we need to go up two levels to reach the workspace root.
fn load_schema(schema_name: &str) -> Result<JSONSchema, ValidationError> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // From crates/northroot-receipts/, go up to workspace root, then to schemas/receipts/
    let schema_path = PathBuf::from(manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .ok_or_else(|| {
            ValidationError::SerializationError(
                "Failed to resolve workspace root from manifest directory".to_string(),
            )
        })?
        .join("schemas")
        .join("receipts")
        .join(schema_name);

    let schema_content = fs::read_to_string(&schema_path).map_err(|e| {
        ValidationError::SerializationError(format!(
            "Failed to read schema {}: {}",
            schema_path.display(),
            e
        ))
    })?;

    let schema_value: Value = serde_json::from_str(&schema_content).map_err(|e| {
        ValidationError::SerializationError(format!(
            "Failed to parse schema {}: {}",
            schema_path.display(),
            e
        ))
    })?;

    JSONSchema::compile(&schema_value).map_err(|e| {
        ValidationError::SchemaViolation(format!("Failed to compile schema {}: {}", schema_name, e))
    })
}

#[cfg(feature = "jsonschema")]
/// Validate a receipt payload against its JSON schema.
pub fn validate_payload_schema(receipt: &Receipt) -> Result<(), ValidationError> {
    let schema_name = match receipt.kind {
        crate::ReceiptKind::DataShape => "data_shape_schema.json",
        crate::ReceiptKind::MethodShape => "method_shape_schema.json",
        crate::ReceiptKind::ReasoningShape => "reasoning_shape_schema.json",
        crate::ReceiptKind::Execution => "execution_schema.json",
        crate::ReceiptKind::Spend => "spend_schema.json",
        crate::ReceiptKind::Settlement => "settlement_schema.json",
    };

    let schema = load_schema(schema_name)?;

    // Serialize payload to JSON value
    let payload_value: Value = serde_json::to_value(&receipt.payload).map_err(|e| {
        ValidationError::SerializationError(format!("Failed to serialize payload: {}", e))
    })?;

    // Validate against schema - collect errors immediately to avoid lifetime issues
    let validation_result = schema.validate(&payload_value);
    match validation_result {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_messages: Vec<String> = errors.map(|e| format!("{}", e)).collect();
            Err(ValidationError::SchemaViolation(format!(
                "Schema validation failed for {}: {}",
                schema_name,
                error_messages.join("; ")
            )))
        }
    }
}

#[cfg(not(feature = "jsonschema"))]
/// JSON Schema validation is not available (jsonschema feature not enabled).
///
/// Enable the `jsonschema` feature to use this function.
pub fn validate_payload_schema(_receipt: &Receipt) -> Result<(), ValidationError> {
    Err(ValidationError::SchemaViolation(
        "JSON Schema validation requires the 'jsonschema' feature".to_string(),
    ))
}
