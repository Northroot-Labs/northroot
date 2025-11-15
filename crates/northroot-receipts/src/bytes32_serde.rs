//! Serialization for fixed-size byte arrays ([u8; 32]) that adapts to format (JSON vs CBOR).
//!
//! This module provides serialization for 32-byte arrays that uses:
//! - JSON (human-readable): base64url string format
//! - CBOR (binary): 32-byte byte string
//!
//! This ensures clean round-trips and prevents byte arrays from being converted
//! to arrays of integers when converting between formats.

use serde::{de, Deserialize, Deserializer, Serializer};

/// Serialize Option<[u8; 32]> with format-aware encoding.
///
/// - JSON (human-readable): base64url string or null
/// - CBOR (binary): 32-byte byte string or null
pub fn serialize<S: Serializer>(bytes: &Option<[u8; 32]>, serializer: S) -> Result<S::Ok, S::Error> {
    match bytes {
        Some(b) => {
            if serializer.is_human_readable() {
                // JSON path → base64url string
                use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
                let encoded = URL_SAFE_NO_PAD.encode(b);
                serializer.serialize_some(&encoded)
            } else {
                // CBOR path → 32-byte bstr
                serializer.serialize_some(b)
            }
        }
        None => serializer.serialize_none(),
    }
}

/// Deserialize Option<[u8; 32]> with format-aware decoding.
///
/// - JSON (human-readable): parse from base64url string, array of numbers (legacy), or null
/// - CBOR (binary): expect exactly 32 bytes or null
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<[u8; 32]>, D::Error> {
    if deserializer.is_human_readable() {
        // JSON path → handle multiple formats for backward compatibility
        use serde::de::Visitor;
        
        struct Bytes32Visitor;
        
        impl<'de> Visitor<'de> for Bytes32Visitor {
            type Value = Option<[u8; 32]>;
            
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64url string, array of 32 numbers, or null")
            }
            
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
            
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(Bytes32ValueVisitor)
            }
            
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
        }
        
        struct Bytes32ValueVisitor;
        
        impl<'de> Visitor<'de> for Bytes32ValueVisitor {
            type Value = Option<[u8; 32]>;
            
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64url string or array of 32 numbers")
            }
            
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // New format: base64url string
                use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
                let bytes = URL_SAFE_NO_PAD
                    .decode(v)
                    .map_err(|e| de::Error::custom(format!("Invalid base64url string: {}", e)))?;
                if bytes.len() != 32 {
                    return Err(de::Error::custom(format!(
                        "Expected 32 bytes, got {} bytes",
                        bytes.len()
                    )));
                }
                let mut result = [0u8; 32];
                result.copy_from_slice(&bytes);
                Ok(Some(result))
            }
            
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                // Legacy format: array of numbers
                let mut result = [0u8; 32];
                let mut idx = 0;
                while let Some(val) = seq.next_element::<u8>()? {
                    if idx >= 32 {
                        return Err(de::Error::custom("Array too long, expected 32 bytes"));
                    }
                    result[idx] = val;
                    idx += 1;
                }
                if idx != 32 {
                    return Err(de::Error::custom(format!(
                        "Array too short, expected 32 bytes, got {}",
                        idx
                    )));
                }
                Ok(Some(result))
            }
        }
        
        deserializer.deserialize_option(Bytes32Visitor)
    } else {
        // CBOR path → expect exactly 32 bytes or null
        let opt_bytes: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        match opt_bytes {
            Some(b) => {
                if b.len() != 32 {
                    return Err(de::Error::custom(format!(
                        "Expected 32 bytes in CBOR, got {} bytes",
                        b.len()
                    )));
                }
                let mut result = [0u8; 32];
                result.copy_from_slice(&b);
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}

