//! Update engine test vector JSON files with new CBOR-based hashes.
//!
//! This example reads engine test vectors (arrays of receipts), recomputes their hashes,
//! and updates the hash field in each receipt.

use northroot_receipts::adapters::json;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    let vectors = [
        "vectors/engine/composition_chain_valid.json",
        "vectors/engine/composition_chain_invalid.json",
    ];

    for path in &vectors {
        let full_path = workspace_root.join(path);
        println!("Updating hashes in: {}", path);

        // Read JSON file
        let json_str = fs::read_to_string(&full_path).unwrap();
        let mut json_value: Value = serde_json::from_str(&json_str).unwrap();

        // Process array of receipts
        if let Value::Array(receipts) = &mut json_value {
            for (idx, receipt_json) in receipts.iter_mut().enumerate() {
                let receipt_str = serde_json::to_string(receipt_json).unwrap();
                let receipt = json::receipt_from_json(&receipt_str).unwrap();
                let new_hash = receipt.compute_hash().unwrap();

                // Update hash in JSON
                receipt_json["hash"] = Value::String(new_hash.clone());
                println!("  Receipt {}: {}", idx, new_hash);
            }
        }

        // Write back to file
        let updated_json = serde_json::to_string_pretty(&json_value).unwrap();
        fs::write(&full_path, updated_json).unwrap();
    }

    println!("\nAll engine test vectors updated with CBOR-based hashes.");
}
