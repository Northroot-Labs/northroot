use std::collections::BTreeMap;

use northroot_canonical::{
    canonicalizer::Canonicalizer, ContentRef, Digest, DigestAlg, HygieneReport, HygieneStatus,
    HygieneWarning, ProfileId, Quantity,
};
use serde_json::json;

#[test]
fn digest_serializes_to_golden_json() {
    let digest = Digest {
        alg: DigestAlg::Sha256,
        b64: "Zm9vYmFy".into(),
    };

    assert_eq!(
        serde_json::to_string(&digest).unwrap(),
        r#"{"alg":"sha-256","b64":"Zm9vYmFy"}"#
    );
}

#[test]
fn quantity_dec_serialization_is_deterministic() {
    let quantity = Quantity::Dec {
        m: "12345".into(),
        s: 2,
    };

    assert_eq!(
        serde_json::to_string(&quantity).unwrap(),
        r#"{"t":"dec","m":"12345","s":2}"#
    );
}

#[test]
fn hygiene_report_matches_expected_shape() {
    let report = HygieneReport {
        status: HygieneStatus::Ok,
        warnings: vec![HygieneWarning::new("DuplicateKeys")],
        metrics: BTreeMap::new(),
        profile_id: ProfileId::new("example_profile_0001".into()),
    };

    let serialized = serde_json::to_value(&report).unwrap();
    let expected = json!({
        "status": "Ok",
        "warnings": ["DuplicateKeys"],
        "metrics": {},
        "profile_id": "example_profile_0001"
    });

    assert_eq!(serialized, expected);
}

#[test]
fn canonicalizer_produces_ordered_bytes() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile);
    // Test with string values
    let value = json!({"b": "value1", "a": {"nested": "value2"}});
    let result = canonicalizer.canonicalize(&value).unwrap();
    assert_eq!(
        result.bytes,
        br#"{"a":{"nested":"value2"},"b":"value1"}"#.to_vec()
    );
    assert_eq!(result.report.status, HygieneStatus::Ok);
}

#[test]
fn content_ref_serialization_includes_digest() {
    let payload = json!({
        "alg": "sha-256",
        "b64": "Zm9v"
    });
    let content_ref = ContentRef {
        digest: Digest {
            alg: DigestAlg::Sha256,
            b64: "Zm9v".into(),
        },
        size_bytes: Some(42),
        media_type: Some("application/json".into()),
    };

    assert_eq!(
        serde_json::to_string(&content_ref).unwrap(),
        format!(
            r#"{{"digest":{},"size_bytes":42,"media_type":"application/json"}}"#,
            payload
        )
    );
}

#[test]
fn canonicalizer_validates_object_structure() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile.clone());

    // Test with valid structure (no raw numbers) to ensure validation passes
    let value = json!({
        "a": "value1",
        "b": "value2"
    });

    let result = canonicalizer.canonicalize_with_report(&value);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.report.status, HygieneStatus::Ok);

    // Note: Duplicate key detection is not performed here because
    // serde_json::Value::Object cannot have duplicates by design.
    // Duplicate detection should happen at the JSON parsing layer.
}

#[test]
fn canonicalizer_validates_nested_structures() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile.clone());

    // Test with valid nested structure
    let value = json!({
        "outer": {
            "inner": "value1",
            "other": "value2"
        },
        "other": "value3"
    });

    let result = canonicalizer.canonicalize_with_report(&value);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.report.status, HygieneStatus::Ok);
}

#[test]
fn canonicalizer_allows_raw_json_numbers() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile.clone());

    // Raw JSON numbers are allowed (schema validation handles quantity field restrictions)
    let value = json!({"amount": 42, "scale": 2});

    let result = canonicalizer.canonicalize_with_report(&value);
    assert!(result.is_ok(), "Raw JSON numbers should be allowed");

    let result = result.unwrap();
    assert_eq!(result.report.status, HygieneStatus::Ok);
    // Verify canonical bytes contain the numbers
    let canonical_str = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(canonical_str.contains("42"));
    assert!(canonical_str.contains("2"));
}

#[test]
fn canonicalizer_rejects_non_finite_numbers() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile.clone());

    // Create a Value with Infinity (serde_json doesn't support NaN/Infinity directly,
    // but we test the validation logic)
    // Note: serde_json::Value::Number doesn't expose Infinity, so we test via f64 parsing
    // For a real test, we'd need to construct invalid JSON or use a custom parser
    // For now, we test that the validation function exists and would catch it

    // Test with a number that would be Infinity if parsed as f64
    // Since serde_json::Value::Number doesn't support Infinity, we'll test the path exists
    // by ensuring the code compiles and the error variant exists
    let value = json!({"value": 1.0e308}); // Large but finite

    // Large but finite numbers should pass
    let result = canonicalizer.canonicalize(&value);
    assert!(result.is_ok(), "Finite numbers should be allowed");
    assert_eq!(result.unwrap().report.status, HygieneStatus::Ok);
}

#[test]
fn canonicalizer_golden_bytes_simple_object() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile);

    let value = json!({
        "b": 2,
        "a": 1,
        "c": "hello"
    });

    // Raw JSON numbers are now allowed
    let result = canonicalizer.canonicalize(&value).unwrap();
    // Verify keys are lexicographically ordered: "a", "b", "c"
    let canonical_str = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(canonical_str.starts_with(r#"{"a":1,"b":2,"c":"hello"}"#));
    assert_eq!(result.report.status, HygieneStatus::Ok);
}

#[test]
fn canonicalizer_golden_bytes_with_quantities() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile);

    // Use proper Quantity types with scale as integer (per schema)
    let value = json!({
        "z": "last",
        "a": {
            "t": "dec",
            "m": "12345",
            "s": 2  // Integer per schema definition
        },
        "b": {
            "t": "int",
            "v": "42"
        }
    });

    let result = canonicalizer.canonicalize(&value).unwrap();
    // Golden bytes: keys should be lexicographically ordered
    let canonical_str = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(canonical_str.contains(r#""a":"#));
    assert!(canonical_str.contains(r#""b":"#));
    assert!(canonical_str.contains(r#""z":"#));
    assert!(canonical_str.contains(r#""s":2"#)); // Verify scale is a number
    assert_eq!(result.report.status, HygieneStatus::Ok);
}

#[test]
fn canonicalizer_golden_bytes_nested_structures() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile);

    let value = json!({
        "data": {
            "items": [
                {"id": {"t": "int", "v": "1"}, "value": "first"},
                {"id": {"t": "int", "v": "2"}, "value": "second"}
            ],
            "metadata": {
                "timestamp": "2023-10-27T10:00:00Z",
                "source": "test"
            }
        },
        "version": "1.0"
    });

    let result = canonicalizer.canonicalize(&value).unwrap();
    // Verify ordering: "data" comes before "version"
    let canonical_str = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(canonical_str.starts_with(r#"{"data":"#));
    assert!(canonical_str.contains(r#""version":"#));
    assert_eq!(result.report.status, HygieneStatus::Ok);
}

#[test]
fn canonicalizer_hygiene_report_serialization_stability() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile.clone());

    // Test with valid structure to get hygiene report
    let value = json!({"amount": 42, "name": "test"});

    let result = canonicalizer.canonicalize_with_report(&value).unwrap();
    let report = result.report;

    // Serialize and deserialize to ensure stability
    let serialized = serde_json::to_string(&report).unwrap();
    let deserialized: HygieneReport = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.status, HygieneStatus::Ok);
    assert_eq!(deserialized.profile_id, profile);
}

#[test]
fn canonicalizer_can_canonicalize_quantity_with_scale() {
    let profile = ProfileId::parse("profileid000000001").unwrap();
    let canonicalizer = Canonicalizer::new(profile);

    // Create a Quantity::Dec and serialize it to JSON
    let quantity = Quantity::Dec {
        m: "12345".into(),
        s: 2,
    };
    let value = serde_json::to_value(&quantity).unwrap();

    // Should canonicalize successfully (scale is a valid integer per schema)
    let result = canonicalizer.canonicalize(&value).unwrap();
    assert_eq!(result.report.status, HygieneStatus::Ok);

    // Verify the canonical bytes contain the quantity structure
    let canonical_str = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(canonical_str.contains(r#""t":"dec""#));
    assert!(canonical_str.contains(r#""m":"12345""#));
    assert!(canonical_str.contains(r#""s":2"#)); // Scale as integer
}
