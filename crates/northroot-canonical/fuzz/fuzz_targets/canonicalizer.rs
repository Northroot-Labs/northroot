#![no_main]
use libfuzzer_sys::fuzz_target;
use northroot_canonical::{Canonicalizer, ProfileId};
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    // Parse input as JSON
    let Ok(value): Result<Value, _> = serde_json::from_slice(data) else {
        return;
    };

    // Test with valid profile (pattern: [A-Za-z0-9_-]{16,128})
    let profile = ProfileId::new("test_profile_12345".to_string());
    let canonicalizer = Canonicalizer::new(profile);

    // Fuzz canonicalize - should handle any valid JSON
    let _ = canonicalizer.canonicalize(&value);
    
    // Fuzz canonicalize_with_report - should always return report
    let _ = canonicalizer.canonicalize_with_report(&value);
});

