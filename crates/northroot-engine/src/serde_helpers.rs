//! Serde helpers for serialization.

use serde::{de, Deserialize, Deserializer, Serializer};

/// Serialize a byte array as a hex string.
///
/// For JSON: outputs hex string (e.g., "a1b2c3...")
/// For CBOR: outputs byte string directly (more efficient)
pub fn serialize<S: Serializer>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        // JSON path → hex string
        let hex_str = hex::encode(bytes);
        serializer.serialize_str(&hex_str)
    } else {
        // CBOR path → byte string
        serializer.serialize_bytes(bytes)
    }
}

/// Deserialize a hex string or byte string to a byte array.
///
/// Accepts:
/// - JSON: hex string (e.g., "a1b2c3...")
/// - CBOR: byte string (32 bytes)
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 32], D::Error> {
    if deserializer.is_human_readable() {
        // JSON path → parse hex string
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_str)
            .map_err(|e| de::Error::custom(format!("Invalid hex string: {}", e)))?;
        if bytes.len() != 32 {
            return Err(de::Error::custom(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes);
        Ok(result)
    } else {
        // CBOR path → expect byte string
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(de::Error::custom(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(&bytes);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(with = "super")]
        hash: [u8; 32],
    }

    #[test]
    fn test_json_roundtrip() {
        let hash = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
                    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20];
        let test = TestStruct { hash };

        // Serialize to JSON
        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"));

        // Deserialize from JSON
        let test2: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(test.hash, test2.hash);
    }

    #[test]
    fn test_cbor_roundtrip() {
        let hash = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
                    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
                    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20];
        let test = TestStruct { hash };

        // Serialize to CBOR
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&test, &mut cbor_bytes).unwrap();

        // Should be compact (32 bytes for hash, not a hex string)
        assert!(cbor_bytes.len() < 50);

        // Deserialize from CBOR
        let test2: TestStruct = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
        assert_eq!(test.hash, test2.hash);
    }
}



