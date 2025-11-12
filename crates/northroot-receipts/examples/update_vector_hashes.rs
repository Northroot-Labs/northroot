//! Update test vector JSON files with new CBOR-based hashes.
//!
//! This example reads all test vectors, recomputes their hashes using CBOR canonicalization,
//! and updates the hash field in each JSON file.

use northroot_receipts::adapters::json;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    let vectors = [
        "vectors/data_shape.json",
        "vectors/method_shape.json",
        "vectors/reasoning_shape.json",
        "vectors/execution.json",
        "vectors/spend.json",
        "vectors/settlement.json",
    ];

    for path in &vectors {
        let full_path = workspace_root.join(path);
        println!("Updating hash in: {}", path);

        // Read JSON file
        let json_str = fs::read_to_string(&full_path).unwrap();
        let mut json_value: Value = serde_json::from_str(&json_str).unwrap();

        // Load receipt and compute new hash
        let mut receipt = json::receipt_from_json(&json_str).unwrap();
        let new_hash = receipt.compute_hash().unwrap();

        // Update hash in JSON
        json_value["hash"] = Value::String(new_hash.clone());

        // Write back to file
        let updated_json = serde_json::to_string_pretty(&json_value).unwrap();
        fs::write(&full_path, updated_json).unwrap();

        println!("  Updated hash: {}", new_hash);
    }

    println!("\nAll test vectors updated with CBOR-based hashes.");
}
