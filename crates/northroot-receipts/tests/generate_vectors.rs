//! Generate canonical receipt vectors with correct hashes.
//!
//! This module provides functions to generate test receipts for all six kinds
//! with properly computed hashes and valid composition chains.
//!
//! Uses shared generation functions from `northroot_receipts::test_utils`.

use northroot_receipts::{
    generate_data_shape_receipt, generate_execution_receipt, generate_method_shape_receipt,
    generate_reasoning_shape_receipt, generate_settlement_receipt, generate_spend_receipt, Receipt,
};

/// Generate a sequential chain of receipts with matching dom/cod.
pub fn generate_sequential_chain() -> Vec<Receipt> {
    // Use re-exported functions from test_utils module
    let data_shape = generate_data_shape_receipt();
    let method_shape = generate_method_shape_receipt(&data_shape.cod);
    let reasoning_shape = generate_reasoning_shape_receipt(&method_shape.cod);
    let execution = generate_execution_receipt(&reasoning_shape.cod);
    let spend = generate_spend_receipt(&execution.cod);
    let settlement = generate_settlement_receipt(&spend.cod);

    vec![
        data_shape,
        method_shape,
        reasoning_shape,
        execution,
        spend,
        settlement,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_chain() {
        let chain = generate_sequential_chain();
        assert_eq!(chain.len(), 6);

        // Verify composition
        for i in 0..chain.len() - 1 {
            assert_eq!(chain[i].cod, chain[i + 1].dom, "Mismatch at index {}", i);
        }

        // Verify all receipts validate
        for receipt in &chain {
            receipt.validate().unwrap();
        }
    }
}
