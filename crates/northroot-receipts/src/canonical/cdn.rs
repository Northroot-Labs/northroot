//! CBOR Diagnostic Notation (CDN) pretty-printer.
//!
//! This module converts CBOR values to human-readable CDN format for debugging
//! and observability. CDN is the standard text representation of CBOR data.

use ciborium::value::Value as CborValue;

/// Convert a CBOR value to CBOR Diagnostic Notation (CDN) string.
///
/// CDN is a human-readable text representation of CBOR data, similar to JSON
/// but with explicit type information (e.g., `h'hex'` for bytes).
///
/// # Arguments
///
/// * `value` - CBOR value to format
///
/// # Returns
///
/// CDN-formatted string
pub fn to_cdn(value: &CborValue) -> String {
    match value {
        CborValue::Null => "null".into(),
        CborValue::Bool(b) => b.to_string(),
        CborValue::Integer(i) => {
            // Format as integer - ciborium::value::Integer doesn't implement Display
            // Use Debug formatting and extract the number from the string
            // Format is typically "Integer(42)" or similar
            let debug_str = format!("{:?}", i);
            // Try to extract number from debug string, or use the whole thing
            // For now, just use Debug format and clean it up
            debug_str
                .trim_start_matches("Integer(")
                .trim_end_matches(")")
                .to_string()
        }
        CborValue::Float(f) => match f {
            f if f.is_nan() => "NaN".into(),
            f if *f == f64::INFINITY => "Infinity".into(),
            f if *f == f64::NEG_INFINITY => "-Infinity".into(),
            f => format!("{}", f),
        },
        CborValue::Text(s) => {
            // Escape special characters and quote
            format!("\"{}\"", s.escape_default())
        }
        CborValue::Bytes(b) => {
            // Format as hex string with h'...' notation
            format!("h'{}'", hex::encode(b))
        }
        CborValue::Array(a) => {
            let inner: Vec<String> = a.iter().map(to_cdn).collect();
            format!("[{}]", inner.join(", "))
        }
        CborValue::Map(m) => {
            let inner: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}: {}", to_cdn(k), to_cdn(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        CborValue::Tag(tag, inner) => {
            format!("{}({})", tag, to_cdn(inner))
        }
        _ => {
            // Handle any other CBOR value types (shouldn't happen in practice)
            format!("{:?}", value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdn_basic_types() {
        assert_eq!(to_cdn(&CborValue::Null), "null");
        assert_eq!(to_cdn(&CborValue::Bool(true)), "true");
        assert_eq!(to_cdn(&CborValue::Bool(false)), "false");
        assert_eq!(to_cdn(&CborValue::Integer(42.into())), "42");
        assert_eq!(to_cdn(&CborValue::Text("hello".into())), "\"hello\"");
    }

    #[test]
    fn test_cdn_bytes() {
        let bytes = CborValue::Bytes(vec![0x01, 0x02, 0xff]);
        let cdn = to_cdn(&bytes);
        assert!(cdn.starts_with("h'"));
        assert!(cdn.ends_with("'"));
        assert!(cdn.contains("0102ff") || cdn.contains("0102FF"));
    }

    #[test]
    fn test_cdn_array() {
        let arr = CborValue::Array(vec![
            CborValue::Integer(1.into()),
            CborValue::Integer(2.into()),
            CborValue::Text("three".into()),
        ]);
        let cdn = to_cdn(&arr);
        assert!(cdn.starts_with("["));
        assert!(cdn.ends_with("]"));
        assert!(cdn.contains("1"));
        assert!(cdn.contains("2"));
        assert!(cdn.contains("\"three\""));
    }

    #[test]
    fn test_cdn_map() {
        // Create a map directly as a Vec of (key, value) pairs (CBOR Map representation)
        let map: Vec<(CborValue, CborValue)> = vec![(
            CborValue::Text("key".into()),
            CborValue::Text("value".into()),
        )];
        let cbor_map = CborValue::Map(map);
        let cdn = to_cdn(&cbor_map);
        assert!(cdn.starts_with("{"));
        assert!(cdn.ends_with("}"));
        assert!(cdn.contains("key"));
        assert!(cdn.contains("value"));
    }
}
