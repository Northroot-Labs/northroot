//! Strict JSON parsing for canonical evidence boundaries.
//!
//! The trust kernel remains untyped: parsed JSON is still returned as
//! [`serde_json::Value`]. This module only rejects structural ambiguity that
//! would otherwise be lost when object keys collapse during ordinary parsing.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

/// Error returned by [`parse_json_strict`].
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[error("strict JSON parse error: {message}")]
pub struct StrictJsonError {
    message: String,
}

impl StrictJsonError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the deterministic structural error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Parses JSON into [`serde_json::Value`] while rejecting duplicate object keys.
///
/// Duplicate keys are rejected at every object depth before `serde_json::Value`
/// can collapse them. No domain schema, policy, workflow, or event-type
/// semantics are checked here.
///
/// # Errors
///
/// Returns [`StrictJsonError`] for invalid JSON or duplicate object keys.
///
/// # Example
///
/// ```rust
/// use northroot_canonical::parse_json_strict;
///
/// let value = parse_json_strict(r#"{"b":2,"a":1}"#)?;
/// assert_eq!(value["a"], 1);
/// assert!(parse_json_strict(r#"{"a":1,"a":2}"#).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_json_strict(input: &str) -> Result<Value, StrictJsonError> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = StrictValueSeed {
        path: JsonPath::root(),
    }
    .deserialize(&mut deserializer)
    .map_err(|err| StrictJsonError::new(err.to_string()))?;
    deserializer
        .end()
        .map_err(|err| StrictJsonError::new(err.to_string()))?;
    Ok(value)
}

#[derive(Debug, Clone)]
struct JsonPath {
    segments: Vec<String>,
}

impl JsonPath {
    fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    fn push_field(&self, field: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(format!(".{}", field));
        Self { segments }
    }

    fn push_index(&self, index: usize) -> Self {
        let mut segments = self.segments.clone();
        segments.push(format!("[{}]", index));
        Self { segments }
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            write!(f, "$")
        } else {
            write!(f, "${}", self.segments.join(""))
        }
    }
}

struct StrictValueSeed {
    path: JsonPath,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor { path: self.path })
    }
}

struct StrictValueVisitor {
    path: JsonPath,
}

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom(format!("non-finite number at {}", self.path)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_string()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        let mut index = 0;
        while let Some(value) = seq.next_element_seed(StrictValueSeed {
            path: self.path.push_index(index),
        })? {
            values.push(value);
            index += 1;
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate key '{}' at {}",
                    key, self.path
                )));
            }
            let child_path = self.path.push_field(&key);
            let value = map.next_value_seed(StrictValueSeed { path: child_path })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_json_strict;
    use serde_json::json;

    #[test]
    fn rejects_duplicate_top_level_keys() {
        let err = parse_json_strict(r#"{"a":1,"a":2}"#).unwrap_err();
        assert!(err.message().contains("duplicate key 'a' at $"));
    }

    #[test]
    fn rejects_nested_duplicate_keys() {
        let err = parse_json_strict(r#"{"outer":{"a":1,"a":2}}"#).unwrap_err();
        assert!(err.message().contains("duplicate key 'a' at $.outer"));
    }

    #[test]
    fn accepts_same_key_name_in_sibling_objects() {
        let value = parse_json_strict(r#"{"left":{"id":1},"right":{"id":2}}"#).unwrap();
        assert_eq!(value["left"]["id"], 1);
        assert_eq!(value["right"]["id"], 2);
    }

    #[test]
    fn rejects_duplicate_keys_in_arrays_of_objects() {
        let err = parse_json_strict(r#"[{"id":1,"id":2}]"#).unwrap_err();
        assert!(err.message().contains("duplicate key 'id' at $[0]"));
    }

    #[test]
    fn accepts_valid_existing_fixture_shape_unchanged() {
        let value = parse_json_strict(
            r#"{"data":{"items":[{"id":{"t":"int","v":"1"},"value":"first"}]},"version":"1.0"}"#,
        )
        .unwrap();
        assert_eq!(
            value,
            json!({
                "data": {
                    "items": [
                        {"id": {"t": "int", "v": "1"}, "value": "first"}
                    ]
                },
                "version": "1.0"
            })
        );
    }
}
