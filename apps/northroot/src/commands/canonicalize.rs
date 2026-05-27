//! Canonicalize command implementation.

use northroot_canonical::{parse_json_strict, Canonicalizer, ProfileId};
use std::io::{self, Read};

pub fn run(input: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let profile = ProfileId::parse("northroot-canonical-v1")
        .map_err(|e| format!("Invalid profile ID: {}", e))?;
    let canonicalizer = Canonicalizer::new(profile);

    // Read JSON from file or stdin
    let json_str = if let Some(path) = input {
        std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file {}: {}", path, e))?
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    let value = parse_json_strict(&json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    let result = canonicalizer
        .canonicalize(&value)
        .map_err(|e| format!("Canonicalization failed: {}", e))?;

    println!("{}", String::from_utf8_lossy(&result.bytes));
    Ok(())
}
