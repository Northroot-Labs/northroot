//! Event ID command implementation.

use northroot_canonical::{compute_event_id, parse_json_strict, Canonicalizer, ProfileId};
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

    let event_id = compute_event_id(&value, &canonicalizer)
        .map_err(|e| format!("Event ID computation failed: {}", e))?;

    println!("{}", event_id.b64);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn rejects_duplicate_key_input() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("event.json");
        fs::write(&input, r#"{"a":1,"a":2}"#).unwrap();

        let result = run(Some(input.to_str().unwrap().to_string()));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate key"));
    }
}
