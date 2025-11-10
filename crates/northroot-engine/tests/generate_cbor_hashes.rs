//! Helper test to generate CBOR hashes for golden vectors.
//!
//! Run with: cargo test --package northroot-engine --test generate_cbor_hashes -- --ignored --nocapture

use northroot_engine::commitments::*;
use serde_json::json;

#[test]
#[ignore]
fn generate_cbor_hashes() {
    let test_cases = vec![
        ("simple_map", json!({"z": 3, "a": 1, "m": 2})),
        ("nested_map", json!({"outer": {"inner": {"value": 42}}})),
        (
            "array_with_mixed_types",
            json!([1, "string", true, null, 3.14]),
        ),
        (
            "empty_structures",
            json!({"empty_map": {}, "empty_array": []}),
        ),
        (
            "receipt_like_structure",
            json!({
                "rid": "00000000-0000-0000-0000-000000000001",
                "version": "0.3.0",
                "kind": "spend",
                "payload": {
                    "meter": {
                        "vcpu_sec": 0.1,
                        "gb_sec": 0.02
                    },
                    "currency": "USD"
                }
            }),
        ),
    ];

    println!("Generated CBOR hashes:");
    println!("{{");
    println!(
        "  \"description\": \"Golden test vectors for CBOR deterministic encoding (RFC 8949)\","
    );
    println!("  \"vectors\": [");

    for (i, (name, value)) in test_cases.iter().enumerate() {
        let hash = cbor_hash(value).unwrap();
        println!("    {{");
        println!("      \"name\": \"{}\",", name);
        println!("      \"description\": \"{}\",", name.replace("_", " "));
        println!("      \"json\": {},", serde_json::to_string(value).unwrap());
        println!("      \"expected_cbor_hash\": \"{}\"", hash);
        if i < test_cases.len() - 1 {
            println!("    }},");
        } else {
            println!("    }}");
        }
    }

    println!("  ]");
    println!("}}");
}
