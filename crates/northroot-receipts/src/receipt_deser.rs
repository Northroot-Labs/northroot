//! Custom deserialization for Receipt to handle kind/payload mapping.
//!
//! This module provides format-agnostic deserialization that works with both
//! JSON and CBOR formats. It extracts the `kind` field first, then deserializes
//! the `payload` field as the appropriate payload type based on the kind.

use crate::{Payload, Receipt, ReceiptKind};
use ciborium::value::Value as CborValue;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

/// Deserialize a Receipt from any format (JSON or CBOR), handling the kind/payload mapping.
///
/// This function extracts the `kind` field first, then deserializes the `payload`
/// field as the appropriate payload type based on the kind. It works with any
/// Serde-compatible format (JSON, CBOR, etc.) by using ciborium::value::Value
/// as a format-agnostic intermediate representation.
pub fn deserialize_receipt<'de, D>(deserializer: D) -> Result<Receipt, D::Error>
where
    D: Deserializer<'de>,
{
    // Check if this is a human-readable format (JSON) or binary (CBOR)
    // For JSON, we can use serde_json::Value as intermediate
    // For CBOR, we use CborValue
    if deserializer.is_human_readable() {
        // JSON path: deserialize directly from JSON to preserve custom deserializers
        // This ensures that custom deserializers (like bytes32_serde) work correctly
        // with JSON format (base64url strings) instead of being converted to CBOR first
        deserialize_from_json_value(deserializer)
    } else {
        // CBOR path: deserialize directly as CborValue
        let value = CborValue::deserialize(deserializer)?;
        deserialize_from_cbor_value(value)
    }
}

/// Deserialize Receipt directly from JSON (preserves custom deserializers).
fn deserialize_from_json_value<'de, D>(deserializer: D) -> Result<Receipt, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value as JsonValue;
    
    // Deserialize as JSON value first
    let json_value: JsonValue = JsonValue::deserialize(deserializer)?;
    
    // Extract fields from JSON
    let obj = json_value.as_object().ok_or_else(|| {
        Error::invalid_type(serde::de::Unexpected::Other("expected object"), &"an object")
    })?;
    
    // Extract kind
    let kind_str = obj.get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("kind"))?;
    let kind = match kind_str {
        "data_shape" => ReceiptKind::DataShape,
        "method_shape" => ReceiptKind::MethodShape,
        "reasoning_shape" => ReceiptKind::ReasoningShape,
        "execution" => ReceiptKind::Execution,
        "spend" => ReceiptKind::Spend,
        "settlement" => ReceiptKind::Settlement,
        _ => return Err(Error::invalid_value(
            serde::de::Unexpected::Str(kind_str),
            &"one of: data_shape, method_shape, reasoning_shape, execution, spend, settlement",
        )),
    };
    
    // Deserialize payload directly from JSON value using a JSON deserializer
    // This preserves the is_human_readable() context for custom deserializers
    let payload_value = obj.get("payload")
        .ok_or_else(|| Error::missing_field("payload"))?;
    
    // Convert JSON value to string and deserialize with proper JSON deserializer
    // This ensures custom deserializers see is_human_readable() == true
    let payload_json_str = serde_json::to_string(payload_value)
        .map_err(|e| Error::custom(format!("Failed to serialize payload to JSON: {}", e)))?;
    
    let payload = match kind {
        ReceiptKind::DataShape => Payload::DataShape(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize DataShapePayload: {}", e))
            })?,
        ),
        ReceiptKind::MethodShape => Payload::MethodShape(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize MethodShapePayload: {}", e))
            })?,
        ),
        ReceiptKind::ReasoningShape => Payload::ReasoningShape(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize ReasoningShapePayload: {}", e))
            })?,
        ),
        ReceiptKind::Execution => Payload::Execution(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize ExecutionPayload: {}", e))
            })?,
        ),
        ReceiptKind::Spend => Payload::Spend(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize SpendPayload: {}", e))
            })?,
        ),
        ReceiptKind::Settlement => Payload::Settlement(
            serde_json::from_str(&payload_json_str).map_err(|e| {
                Error::custom(format!("Failed to deserialize SettlementPayload: {}", e))
            })?,
        ),
    };
    
    // Extract other fields
    let rid_str = obj.get("rid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("rid"))?;
    let rid = Uuid::parse_str(rid_str)
        .map_err(|e| Error::custom(format!("Invalid UUID in rid: {}", e)))?;
    
    let version = obj.get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("version"))?
        .to_string();
    let dom = obj.get("dom")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("dom"))?
        .to_string();
    let cod = obj.get("cod")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("cod"))?
        .to_string();
    
    // Extract links (array of UUID strings)
    let links = obj.get("links")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.as_str()
                        .and_then(|s| Uuid::parse_str(s).ok())
                })
                .collect()
        })
        .unwrap_or_default();
    
    // Extract ctx - deserialize directly from JSON
    let ctx_value = obj.get("ctx")
        .ok_or_else(|| Error::missing_field("ctx"))?;
    let ctx_json = serde_json::to_string(ctx_value)
        .map_err(|e| Error::custom(format!("Failed to serialize ctx to JSON: {}", e)))?;
    let ctx: crate::Context = serde_json::from_str(&ctx_json)
        .map_err(|e| Error::custom(format!("Failed to deserialize Context: {}", e)))?;
    
    // Extract attest (optional)
    let attest = obj.get("attest").map(|v| {
        // Convert JSON to CBOR Value for attest
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(v, &mut cbor_bytes).ok()?;
        ciborium::de::from_reader(cbor_bytes.as_slice()).ok()
    }).flatten();
    
    // Extract sig (optional)
    let sig = obj.get("sig").and_then(|v| {
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(v, &mut cbor_bytes).ok()?;
        ciborium::de::from_reader(cbor_bytes.as_slice()).ok()
    });
    
    let hash = obj.get("hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("hash"))?
        .to_string();
    
    Ok(Receipt {
        rid,
        version,
        kind,
        dom,
        cod,
        links,
        ctx,
        payload,
        attest,
        sig,
        hash,
    })
}

/// Deserialize Receipt from a CborValue (internal helper).
fn deserialize_from_cbor_value<D>(value: CborValue) -> Result<Receipt, D>
where
    D: serde::de::Error,
{
    // Convert to a map-like structure we can work with
    let map = value.as_map().ok_or_else(|| {
        Error::invalid_type(
            serde::de::Unexpected::Other("expected object/map"),
            &"an object",
        )
    })?;

    // Helper to extract string value from map
    fn get_string(
        map: &[(CborValue, CborValue)],
        key: &'static str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        map.iter()
            .find(|(k, _)| {
                if let CborValue::Text(s) = k {
                    s == key
                } else {
                    false
                }
            })
            .and_then(|(_, v)| {
                if let CborValue::Text(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("Missing field: {}", key).into())
    }

    // Extract kind first (required for payload deserialization)
    let kind_str = get_string(map, "kind").map_err(|e| Error::custom(format!("{}", e)))?;
    let kind = match kind_str.as_str() {
        "data_shape" => ReceiptKind::DataShape,
        "method_shape" => ReceiptKind::MethodShape,
        "reasoning_shape" => ReceiptKind::ReasoningShape,
        "execution" => ReceiptKind::Execution,
        "spend" => ReceiptKind::Spend,
        "settlement" => ReceiptKind::Settlement,
        _ => {
            return Err(Error::invalid_value(
                serde::de::Unexpected::Str(&kind_str),
                &"one of: data_shape, method_shape, reasoning_shape, execution, spend, settlement",
            ));
        }
    };

    // Extract payload based on kind
    let payload_value = map
        .iter()
        .find(|(k, _)| {
            if let CborValue::Text(s) = k {
                s == "payload"
            } else {
                false
            }
        })
        .map(|(_, v)| v)
        .ok_or_else(|| Error::missing_field("payload"))?;

    // Serialize payload_value to CBOR bytes, then deserialize as the specific payload type
    let mut payload_cbor = Vec::new();
    ciborium::ser::into_writer(payload_value, &mut payload_cbor)
        .map_err(|e| Error::custom(format!("Failed to serialize payload: {}", e)))?;

    // Deserialize payload based on kind
    let payload = match kind {
        ReceiptKind::DataShape => Payload::DataShape(
            ciborium::de::from_reader(payload_cbor.as_slice()).map_err(|e| {
                Error::custom(format!("Failed to deserialize DataShapePayload: {}", e))
            })?,
        ),
        ReceiptKind::MethodShape => {
            Payload::MethodShape(ciborium::de::from_reader(payload_cbor.as_slice()).map_err(
                |e| Error::custom(format!("Failed to deserialize MethodShapePayload: {}", e)),
            )?)
        }
        ReceiptKind::ReasoningShape => Payload::ReasoningShape(
            ciborium::de::from_reader(payload_cbor.as_slice()).map_err(|e| {
                Error::custom(format!(
                    "Failed to deserialize ReasoningShapePayload: {}",
                    e
                ))
            })?,
        ),
        ReceiptKind::Execution => Payload::Execution(
            ciborium::de::from_reader(payload_cbor.as_slice()).map_err(|e| {
                Error::custom(format!("Failed to deserialize ExecutionPayload: {}", e))
            })?,
        ),
        ReceiptKind::Spend => Payload::Spend(
            ciborium::de::from_reader(payload_cbor.as_slice())
                .map_err(|e| Error::custom(format!("Failed to deserialize SpendPayload: {}", e)))?,
        ),
        ReceiptKind::Settlement => {
            Payload::Settlement(ciborium::de::from_reader(payload_cbor.as_slice()).map_err(
                |e| Error::custom(format!("Failed to deserialize SettlementPayload: {}", e)),
            )?)
        }
    };

    // Extract rid (UUID) - handle both Text (from JSON) and Array (from CBOR conversion)
    let rid = map
        .iter()
        .find(|(k, _)| {
            if let CborValue::Text(s) = k {
                s == "rid"
            } else {
                false
            }
        })
        .and_then(|(_, v)| {
            match v {
                CborValue::Text(s) => {
                    // Text format (from JSON) - parse as UUID string
                    Uuid::parse_str(s).ok()
                }
                CborValue::Array(_arr) => {
                    // Array format - this shouldn't happen for UUIDs with proper serialization
                    // With uuid_serde, UUIDs should be Bytes in CBOR, not Arrays
                    None
                }
                CborValue::Bytes(b) => {
                    // Bytes format - direct UUID bytes
                    if b.len() == 16 {
                        let mut bytes = [0u8; 16];
                        bytes.copy_from_slice(&b[..16]);
                        Some(Uuid::from_bytes(bytes))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .ok_or_else(|| Error::missing_field("rid"))?;

    let version = get_string(map, "version").map_err(|e| Error::custom(format!("{}", e)))?;
    let dom = get_string(map, "dom").map_err(|e| Error::custom(format!("{}", e)))?;
    let cod = get_string(map, "cod").map_err(|e| Error::custom(format!("{}", e)))?;

    // Extract links (array of UUIDs) - handle both Text (JSON) and Bytes (CBOR) formats
    // Defensive matching: keys can be Text("links"), value should be Array
    // Array elements can be Text (JSON->CBOR conversion) or Bytes (direct CBOR serialization)
    let links = {
        let mut found_links = None;
        for (k, v) in map.iter() {
            // Match on Text key with defensive check
            let is_links_key = match k {
                CborValue::Text(s) => s == "links",
                _ => false,
            };

            if !is_links_key {
                continue;
            }

            // Value should be an Array; elements can be Text (JSON) or Bytes (CBOR)
            match v {
                CborValue::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        match item {
                            CborValue::Text(s) => {
                                // JSON format: string (from JSON->CBOR conversion)
                                if let Ok(uuid) = Uuid::parse_str(s) {
                                    out.push(uuid);
                                } else {
                                    // Invalid UUID string, skip this item
                                    continue;
                                }
                            }
                            CborValue::Bytes(b) => {
                                // CBOR format: 16-byte byte string (from direct CBOR serialization)
                                if b.len() == 16 {
                                    let mut bytes = [0u8; 16];
                                    bytes.copy_from_slice(&b[..16]);
                                    out.push(Uuid::from_bytes(bytes));
                                } else {
                                    // Invalid length, skip this item
                                    continue;
                                }
                            }
                            _ => {
                                // Unexpected element type, skip this item
                                continue;
                            }
                        }
                    }
                    found_links = Some(out);
                    break; // Found links, no need to continue
                }
                _ => {
                    // links key exists but value is not an array - treat as empty
                    found_links = Some(Vec::new());
                    break;
                }
            }
        }
        found_links.unwrap_or_default()
    };

    // Extract ctx - serialize to CBOR then deserialize as Context
    let ctx_value = map
        .iter()
        .find(|(k, _)| {
            if let CborValue::Text(s) = k {
                s == "ctx"
            } else {
                false
            }
        })
        .map(|(_, v)| v)
        .ok_or_else(|| Error::missing_field("ctx"))?;

    let mut ctx_cbor = Vec::new();
    ciborium::ser::into_writer(ctx_value, &mut ctx_cbor)
        .map_err(|e| Error::custom(format!("Failed to serialize ctx: {}", e)))?;

    let ctx = ciborium::de::from_reader(ctx_cbor.as_slice())
        .map_err(|e| Error::custom(format!("Failed to deserialize Context: {}", e)))?;

    // Extract attest (optional) - keep as CBOR Value
    let attest = map
        .iter()
        .find(|(k, _)| {
            if let CborValue::Text(s) = k {
                s == "attest"
            } else {
                false
            }
        })
        .map(|(_, v)| v.clone());

    // Extract sig (optional)
    let sig = map
        .iter()
        .find(|(k, _)| {
            if let CborValue::Text(s) = k {
                s == "sig"
            } else {
                false
            }
        })
        .and_then(|(_, v)| {
            let mut sig_cbor = Vec::new();
            ciborium::ser::into_writer(v, &mut sig_cbor).ok()?;
            ciborium::de::from_reader(sig_cbor.as_slice()).ok()
        });

    let hash = get_string(map, "hash").map_err(|e| Error::custom(format!("{}", e)))?;

    Ok(Receipt {
        rid,
        version,
        kind,
        dom,
        cod,
        links,
        ctx,
        payload,
        attest,
        sig,
        hash,
    })
}
