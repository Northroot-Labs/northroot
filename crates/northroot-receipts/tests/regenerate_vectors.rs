//! Regenerate vectors with correct hashes.
//!
//! Run with: cargo test --test regenerate_vectors -- --ignored --nocapture
//! This will update the vector files with correctly computed hashes.
//!
//! Uses shared generation functions from `northroot_receipts::test_utils`.

use northroot_receipts::{
    generate_data_shape_receipt, generate_execution_receipt, generate_method_shape_receipt,
    generate_reasoning_shape_receipt, generate_settlement_receipt, generate_spend_receipt,
    validate_composition,
};
use std::fs;
use std::io::Write;

#[test]
#[ignore] // Only run when explicitly requested
fn regenerate_all_vectors() {
    // Use re-exported functions from test_utils module
    let data_shape = generate_data_shape_receipt();
    let method_shape = generate_method_shape_receipt(&data_shape.cod);
    let reasoning_shape = generate_reasoning_shape_receipt(&method_shape.cod);
    let execution = generate_execution_receipt(&reasoning_shape.cod);
    let spend = generate_spend_receipt(&execution.cod);
    let settlement = generate_settlement_receipt(&spend.cod);

    let chain = vec![
        data_shape,
        method_shape,
        reasoning_shape,
        execution,
        spend,
        settlement,
    ];

    let vectors_dir = "../../vectors";

    // Write each receipt to its file
    let files = [
        "data_shape.json",
        "method_shape.json",
        "reasoning_shape.json",
        "execution.json",
        "spend.json",
        "settlement.json",
    ];

    for (i, file) in files.iter().enumerate() {
        let receipt = &chain[i];
        let json = serde_json::to_string_pretty(receipt).unwrap();
        let path = format!("{}/{}", vectors_dir, file);

        let mut f =
            fs::File::create(&path).unwrap_or_else(|e| panic!("Failed to create {}: {}", path, e));
        f.write_all(json.as_bytes()).unwrap();

        println!("Regenerated: {}", path);
    }

    // Verify all regenerated vectors
    for receipt in &chain {
        receipt.validate().unwrap();
    }

    // Verify composition
    validate_composition(&chain).unwrap();

    println!("All vectors regenerated and validated successfully!");
}
