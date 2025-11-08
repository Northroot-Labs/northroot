//! Custom deserialization for Receipt to handle kind/payload mapping.

use crate::{Payload, Receipt, ReceiptKind};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use uuid::Uuid;

/// Deserialize a Receipt from JSON, handling the kind/payload mapping.
///
/// This function extracts the `kind` field first, then deserializes the `payload`
/// field as the appropriate payload type based on the kind.
pub fn deserialize_receipt<'de, D>(deserializer: D) -> Result<Receipt, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let mut map = value
        .as_object()
        .ok_or_else(|| {
            serde::de::Error::invalid_type(
                serde::de::Unexpected::Other("expected object"),
                &"an object",
            )
        })?
        .clone();

    // Extract kind
    let kind_value = map
        .remove("kind")
        .ok_or_else(|| serde::de::Error::missing_field("kind"))?;
    let kind_str = kind_value.as_str().ok_or_else(|| {
        serde::de::Error::invalid_type(
            serde::de::Unexpected::Other("kind must be a string"),
            &"a string",
        )
    })?;
    let kind =
        match kind_str {
            "data_shape" => ReceiptKind::DataShape,
            "method_shape" => ReceiptKind::MethodShape,
            "reasoning_shape" => ReceiptKind::ReasoningShape,
            "execution" => ReceiptKind::Execution,
            "spend" => ReceiptKind::Spend,
            "settlement" => ReceiptKind::Settlement,
            _ => return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(kind_str),
                &"one of: data_shape, method_shape, reasoning_shape, execution, spend, settlement",
            )),
        };

    // Extract payload
    let payload_value = map
        .remove("payload")
        .ok_or_else(|| serde::de::Error::missing_field("payload"))?;
    let payload = match kind {
        ReceiptKind::DataShape => Payload::DataShape(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
        ReceiptKind::MethodShape => Payload::MethodShape(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
        ReceiptKind::ReasoningShape => Payload::ReasoningShape(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
        ReceiptKind::Execution => Payload::Execution(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
        ReceiptKind::Spend => Payload::Spend(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
        ReceiptKind::Settlement => Payload::Settlement(
            serde_json::from_value(payload_value.clone()).map_err(serde::de::Error::custom)?,
        ),
    };

    // Put payload back for deserializing other fields
    map.insert("payload".to_string(), payload_value);
    map.insert("kind".to_string(), kind_value);

    // Deserialize other fields manually
    let rid_str = map
        .get("rid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("rid"))?;
    let rid = Uuid::parse_str(rid_str)
        .map_err(|e| serde::de::Error::custom(format!("Invalid UUID: {}", e)))?;

    let version = map
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("version"))?
        .to_string();

    let dom = map
        .get("dom")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("dom"))?
        .to_string();

    let cod = map
        .get("cod")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("cod"))?
        .to_string();

    let links = map
        .get("links")
        .map(|v| {
            v.as_array()
                .ok_or_else(|| {
                    serde::de::Error::invalid_type(
                        serde::de::Unexpected::Other("expected array"),
                        &"an array",
                    )
                })
                .and_then(|arr| {
                    arr.iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| {
                                    serde::de::Error::invalid_type(
                                        serde::de::Unexpected::Other("expected string"),
                                        &"a string",
                                    )
                                })
                                .and_then(|s| {
                                    Uuid::parse_str(s).map_err(|e| {
                                        serde::de::Error::custom(format!("Invalid UUID: {}", e))
                                    })
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
        })
        .transpose()?
        .unwrap_or_default();

    let ctx = serde_json::from_value(
        map.get("ctx")
            .cloned()
            .ok_or_else(|| serde::de::Error::missing_field("ctx"))?,
    )
    .map_err(serde::de::Error::custom)?;

    let attest = map.get("attest").cloned();

    let sig = map.get("sig").and_then(|v| {
        if v.is_null() {
            None
        } else {
            serde_json::from_value(v.clone()).ok()
        }
    });

    let hash = map
        .get("hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::missing_field("hash"))?
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
