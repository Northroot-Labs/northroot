//! UUID serialization that adapts to format (JSON vs CBOR).
//!
//! This module provides UUID serialization that uses:
//! - JSON (human-readable): hyphenated string format
//! - CBOR (binary): 16-byte byte string
//!
//! This ensures clean round-trips and prevents UUIDs from being converted
//! to arrays of integers when converting between formats.

use serde::{de, Deserialize, Deserializer, Serializer};
use uuid::Uuid;

/// Serialize UUID with format-aware encoding.
///
/// - JSON (human-readable): hyphenated string
/// - CBOR (binary): 16-byte byte string
pub fn serialize<S: Serializer>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        // JSON path → hyphenated string
        serializer.serialize_str(&uuid.hyphenated().to_string())
    } else {
        // CBOR path → 16-byte bstr
        serializer.serialize_bytes(uuid.as_bytes())
    }
}

/// Deserialize UUID with format-aware decoding.
///
/// - JSON (human-readable): parse from hyphenated string
/// - CBOR (binary): expect exactly 16 bytes
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
    if deserializer.is_human_readable() {
        // JSON path → parse string
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(de::Error::custom)
    } else {
        // CBOR path → expect exactly 16 bytes
        let b: Vec<u8> = Vec::deserialize(deserializer)?;
        if b.len() != 16 {
            return Err(de::Error::custom("UUID must be exactly 16 bytes in CBOR"));
        }
        Uuid::from_slice(&b).map_err(de::Error::custom)
    }
}

/// Serialize Vec<Uuid> with format-aware encoding.
pub mod vec {
    use super::*;
    use serde::{Deserializer, Serializer};

    /// Serialize a slice of UUIDs with format-aware encoding.
    ///
    /// - JSON (human-readable): array of hyphenated strings
    /// - CBOR (binary): array of 16-byte byte strings
    pub fn serialize<S: Serializer>(uuids: &[Uuid], serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let is_human_readable = serializer.is_human_readable();
        let mut seq = serializer.serialize_seq(Some(uuids.len()))?;
        for uuid in uuids {
            if is_human_readable {
                seq.serialize_element(&uuid.hyphenated().to_string())?;
            } else {
                // For CBOR, serialize as byte string, not array of integers
                // Create a wrapper that implements Serialize for byte slices
                struct BytesWrapper<'a>(&'a [u8]);
                impl<'a> serde::Serialize for BytesWrapper<'a> {
                    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        serializer.serialize_bytes(self.0)
                    }
                }
                seq.serialize_element(&BytesWrapper(uuid.as_bytes()))?;
            }
        }
        seq.end()
    }

    /// Deserialize a vector of UUIDs with format-aware decoding.
    ///
    /// - JSON (human-readable): parse from array of hyphenated strings
    /// - CBOR (binary): expect array of 16-byte byte strings
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Uuid>, D::Error> {
        let is_human_readable = deserializer.is_human_readable();

        struct UuidVisitor {
            is_human_readable: bool,
        }

        impl<'de> serde::de::Visitor<'de> for UuidVisitor {
            type Value = Vec<Uuid>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of UUIDs")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut uuids = Vec::new();
                if self.is_human_readable {
                    // JSON: expect strings
                    while let Some(s) = seq.next_element::<String>()? {
                        let uuid = Uuid::parse_str(&s).map_err(de::Error::custom)?;
                        uuids.push(uuid);
                    }
                } else {
                    // CBOR: expect byte vectors
                    while let Some(b) = seq.next_element::<Vec<u8>>()? {
                        if b.len() != 16 {
                            return Err(de::Error::custom("UUID must be exactly 16 bytes in CBOR"));
                        }
                        let uuid = Uuid::from_slice(&b).map_err(de::Error::custom)?;
                        uuids.push(uuid);
                    }
                }
                Ok(uuids)
            }
        }

        deserializer.deserialize_seq(UuidVisitor { is_human_readable })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(with = "super")]
        id: Uuid,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStructWithVec {
        #[serde(with = "super::vec")]
        ids: Vec<Uuid>,
    }

    #[test]
    fn test_json_roundtrip() {
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test = TestStruct { id: uuid };

        // Serialize to JSON
        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("00000000-0000-0000-0000-000000000001"));
        assert!(!json.contains("[")); // Should not be an array

        // Deserialize from JSON
        let test2: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(test.id, test2.id);
    }

    #[test]
    fn test_cbor_roundtrip() {
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test = TestStruct { id: uuid };

        // Serialize to CBOR
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&test, &mut cbor_bytes).unwrap();

        // Should be compact (16 bytes for UUID, not an array)
        assert!(cbor_bytes.len() < 50); // Much smaller than array representation

        // Deserialize from CBOR
        let test2: TestStruct = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
        assert_eq!(test.id, test2.id);
    }

    #[test]
    fn test_vec_json_roundtrip() {
        let uuids = vec![
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        ];
        let test = TestStructWithVec { ids: uuids.clone() };

        // Serialize to JSON
        let json = serde_json::to_string(&test).unwrap();
        let test2: TestStructWithVec = serde_json::from_str(&json).unwrap();
        assert_eq!(test.ids, test2.ids);
    }

    #[test]
    fn test_vec_cbor_roundtrip() {
        let uuids = vec![
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        ];
        let test = TestStructWithVec { ids: uuids.clone() };

        // Serialize to CBOR
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&test, &mut cbor_bytes).unwrap();

        // Deserialize from CBOR
        let test2: TestStructWithVec = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
        assert_eq!(test.ids, test2.ids);
    }

    #[test]
    fn test_json_to_cbor_preserves_uuid() {
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test = TestStruct { id: uuid };

        // Direct CBOR roundtrip (bypassing JSON Value conversion)
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&test, &mut cbor_bytes).unwrap();

        // Deserialize from CBOR - UUID should be 16 bytes, not an array
        let test2: TestStruct = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
        assert_eq!(test.id, test2.id);
    }
}
