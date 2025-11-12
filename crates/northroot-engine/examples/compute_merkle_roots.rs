//! Compute new CBOR-based Merkle roots for test vectors.
//!
//! Run with: cargo run --package northroot-engine --example compute_merkle_roots

use ciborium::value::Value as CborValue;
use northroot_engine::execution::MerkleRowMap;
use serde_json::json;

// Helper to convert JSON Value to CBOR Value
fn json_to_cbor_value(json: &serde_json::Value) -> CborValue {
    match json {
        serde_json::Value::Null => CborValue::Null,
        serde_json::Value::Bool(b) => CborValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CborValue::Integer(i.into())
            } else if let Some(f) = n.as_f64() {
                CborValue::Float(f)
            } else {
                CborValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => CborValue::Text(s.clone()),
        serde_json::Value::Array(a) => CborValue::Array(a.iter().map(json_to_cbor_value).collect()),
        serde_json::Value::Object(o) => {
            let mut map = Vec::new();
            for (k, v) in o {
                map.push((CborValue::Text(k.clone()), json_to_cbor_value(v)));
            }
            CborValue::Map(map)
        }
    }
}

fn main() {
    println!("{{");
    println!("  \"description\": \"Merkle Row-Map examples with CBOR canonicalization\",");
    println!("  \"empty_map\": {{");
    let empty_map = MerkleRowMap::new();
    let empty_root = empty_map.compute_root();
    println!("    \"root\": \"{}\"", empty_root);
    println!("  }},");

    println!("  \"single_entry\": {{");
    let mut single_map = MerkleRowMap::new();
    single_map.insert("key1".to_string(), json_to_cbor_value(&json!(42)));
    let single_root = single_map.compute_root();
    println!("    \"entries\": {{");
    println!("      \"key1\": 42");
    println!("    }},");
    println!("    \"root\": \"{}\"", single_root);
    println!("  }},");

    println!("  \"multiple_entries\": {{");
    let mut multi_map = MerkleRowMap::new();
    multi_map.insert("key1".to_string(), json_to_cbor_value(&json!(42)));
    multi_map.insert("key2".to_string(), json_to_cbor_value(&json!("value")));
    multi_map.insert("key3".to_string(), json_to_cbor_value(&json!(true)));
    let multi_root = multi_map.compute_root();
    println!("    \"entries\": {{");
    println!("      \"key1\": 42,");
    println!("      \"key2\": \"value\",");
    println!("      \"key3\": true");
    println!("    }},");
    println!("    \"root\": \"{}\"", multi_root);
    println!("  }},");

    println!("  \"order_independence\": {{");
    let mut map1 = MerkleRowMap::new();
    map1.insert("a".to_string(), json_to_cbor_value(&json!(1)));
    map1.insert("b".to_string(), json_to_cbor_value(&json!(2)));
    let root1 = map1.compute_root();

    let mut map2 = MerkleRowMap::new();
    map2.insert("b".to_string(), json_to_cbor_value(&json!(2)));
    map2.insert("a".to_string(), json_to_cbor_value(&json!(1)));
    let root2 = map2.compute_root();

    assert_eq!(
        root1, root2,
        "Roots should be same regardless of insertion order"
    );

    println!("    \"map1\": {{");
    println!("      \"entries\": {{");
    println!("        \"a\": 1,");
    println!("        \"b\": 2");
    println!("      }},");
    println!("      \"root\": \"{}\"", root1);
    println!("    }},");
    println!("    \"map2\": {{");
    println!("      \"entries\": {{");
    println!("        \"b\": 2,");
    println!("        \"a\": 1");
    println!("      }},");
    println!("      \"root\": \"{}\"", root2);
    println!("    }}");
    println!("  }}");
    println!("}}");
}
