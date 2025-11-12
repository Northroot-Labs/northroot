//! Compute CBOR-based hashes for all test vectors.
//!
//! This example computes the new CBOR canonicalization hashes for all test vectors
//! and outputs them in a format suitable for updating baseline hashes.

use northroot_receipts::adapters::json;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the workspace root (go up from crates/northroot-receipts/examples)
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

    println!("// Updated baseline hashes (CBOR canonicalization)");
    println!("const BASELINE_HASHES: &[(&str, &str)] = &[");

    for path in &vectors {
        let full_path = workspace_root.join(path);
        let json_str = fs::read_to_string(&full_path).unwrap();
        let receipt = json::receipt_from_json(&json_str).unwrap();
        let hash = receipt.compute_hash().unwrap();
        let filename = path.split('/').last().unwrap();
        println!("    (");
        println!("        \"{}\",", filename);
        println!("        \"{}\",", hash);
        println!("    ),");
    }

    println!("];");
}
