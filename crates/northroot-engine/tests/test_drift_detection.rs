//! Drift detection test: ensures root computation algorithms haven't changed unexpectedly.
//!
//! This test compares computed roots against stored baselines to detect
//! any changes to root computation algorithms that would break compatibility.

use northroot_engine::{
    commit_seq_root, commit_set_root, compute_execution_roots, compute_tensor_root,
};
use serde_json::json;
use std::collections::HashMap;

// Helper to convert JSON to CBOR
fn json_to_cbor(json: &serde_json::Value) -> ciborium::value::Value {
    match json {
        serde_json::Value::Null => ciborium::value::Value::Null,
        serde_json::Value::Bool(b) => ciborium::value::Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ciborium::value::Value::Integer(i.into())
            } else if let Some(f) = n.as_f64() {
                ciborium::value::Value::Float(f)
            } else {
                ciborium::value::Value::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => ciborium::value::Value::Text(s.clone()),
        serde_json::Value::Array(a) => {
            ciborium::value::Value::Array(a.iter().map(json_to_cbor).collect())
        }
        serde_json::Value::Object(o) => {
            let mut map = Vec::new();
            for (k, v) in o {
                map.push((ciborium::value::Value::Text(k.clone()), json_to_cbor(v)));
            }
            ciborium::value::Value::Map(map)
        }
    }
}

/// Baseline roots for all root computation functions.
///
/// These roots are computed from deterministic inputs. If root computation
/// algorithms change, these roots will change, alerting us to potential
/// compatibility issues.
///
/// To update baselines after intentional algorithm changes:
/// 1. Run `cargo test --test test_drift_detection -- --nocapture` to see computed values
/// 2. Update this constant with the new root values
/// 3. Document the change in commit message
const BASELINE_ROOTS: &[(&str, &str)] = &[
    // commit_set_root baselines
    (
        "commit_set_root_empty",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "commit_set_root_single",
        "sha256:5d4cc820b37f3d1fd0c6e04ed50a56ead8d497ecfd3c25c749855ed9d852837d",
    ),
    (
        "commit_set_root_three_items",
        "sha256:b294e910ea566785d193092a0f165f09d7aa909a44d6e41c1ab19846a9092bdb",
    ),
    // commit_seq_root baselines
    (
        "commit_seq_root_empty",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "commit_seq_root_single",
        "sha256:5d4cc820b37f3d1fd0c6e04ed50a56ead8d497ecfd3c25c749855ed9d852837d",
    ),
    (
        "commit_seq_root_three_items",
        "sha256:b294e910ea566785d193092a0f165f09d7aa909a44d6e41c1ab19846a9092bdb",
    ),
    // compute_tensor_root baselines
    (
        "compute_tensor_root_empty",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "compute_tensor_root_single",
        "sha256:5d4cc820b37f3d1fd0c6e04ed50a56ead8d497ecfd3c25c749855ed9d852837d",
    ),
    (
        "compute_tensor_root_three_items",
        "sha256:b294e910ea566785d193092a0f165f09d7aa909a44d6e41c1ab19846a9092bdb",
    ),
    // MerkleRowMap baselines removed - rowmap module deleted as dead weight
    // compute_execution_roots baselines (trace_set_root values)
    (
        "compute_execution_roots_empty_trace_set",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "compute_execution_roots_single_trace_set",
        "sha256:5d4cc820b37f3d1fd0c6e04ed50a56ead8d497ecfd3c25c749855ed9d852837d",
    ),
    (
        "compute_execution_roots_multiple_trace_set",
        "sha256:1a478d6e1f4552286397eb9f2a86cb56e3f33f082a4613ea55537c819369f2ec",
    ),
    // compute_execution_roots baselines (trace_seq_root values)
    (
        "compute_execution_roots_empty_trace_seq",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "compute_execution_roots_single_trace_seq",
        "sha256:5d4cc820b37f3d1fd0c6e04ed50a56ead8d497ecfd3c25c749855ed9d852837d",
    ),
    (
        "compute_execution_roots_multiple_trace_seq",
        "sha256:1a478d6e1f4552286397eb9f2a86cb56e3f33f082a4613ea55537c819369f2ec",
    ),
];

#[allow(dead_code)]
fn test_commit_set_root_baselines() {
    use northroot_engine::commit_set_root;

    // Empty set
    let empty_root = commit_set_root(&[]);

    // Single item
    let single = commit_set_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);

    // Multiple items (order-independent)
    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let h3 = "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    let root1 = commit_set_root(&[h1.clone(), h2.clone(), h3.clone()]);
    let root2 = commit_set_root(&[h3.clone(), h1.clone(), h2.clone()]); // Different order

    // Should be order-independent
    assert_eq!(root1, root2, "commit_set_root should be order-independent");

    // Different hashes produce different root
    let h4 = "sha256:4444444444444444444444444444444444444444444444444444444444444444".to_string();
    let root3 = commit_set_root(&[h1.clone(), h2.clone(), h4]);
    assert_ne!(
        root1, root3,
        "Different hashes should produce different root"
    );

    // Print for baseline population
    println!("commit_set_root empty: {}", empty_root);
    println!("commit_set_root single: {}", single);
    println!("commit_set_root three_items: {}", root1);
}

#[allow(dead_code)]
fn test_commit_seq_root_baselines() {
    use northroot_engine::commit_seq_root;

    // Empty sequence
    let empty_root = commit_seq_root(&[]);

    // Single item
    let single = commit_seq_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);

    // Multiple items (order-dependent)
    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let h3 = "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    let root1 = commit_seq_root(&[h1.clone(), h2.clone(), h3.clone()]);
    let root2 = commit_seq_root(&[h3.clone(), h1.clone(), h2.clone()]); // Different order

    // Should be order-dependent
    assert_ne!(root1, root2, "commit_seq_root should be order-dependent");

    // Print for baseline population
    println!("commit_seq_root empty: {}", empty_root);
    println!("commit_seq_root single: {}", single);
    println!("commit_seq_root three_items: {}", root1);
}

#[allow(dead_code)]
fn test_compute_tensor_root_baselines() {
    use northroot_engine::compute_tensor_root;

    // Empty
    let empty_root = compute_tensor_root(&[]);

    // Single hash
    let single = compute_tensor_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);

    // Multiple hashes (order-independent)
    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let h3 = "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    let root1 = compute_tensor_root(&[h1.clone(), h2.clone(), h3.clone()]);
    let root2 = compute_tensor_root(&[h3.clone(), h1.clone(), h2.clone()]);

    // Should be order-independent
    assert_eq!(
        root1, root2,
        "compute_tensor_root should be order-independent"
    );

    // Print for baseline population
    println!("compute_tensor_root empty: {}", empty_root);
    println!("compute_tensor_root single: {}", single);
    println!("compute_tensor_root three_items: {}", root1);
}

// MerkleRowMap baseline test removed - rowmap module deleted as dead weight

#[allow(dead_code)]
fn test_compute_execution_roots_baselines() {
    use northroot_engine::compute_execution_roots;

    // Empty span commitments
    let empty_roots = compute_execution_roots(
        &[],
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
    );

    // Single span commitment
    let single_roots = compute_execution_roots(
        &["sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string()],
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
    );

    // Multiple span commitments
    let multi_roots = compute_execution_roots(
        &[
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        ],
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
    );

    // Print for baseline population
    println!(
        "compute_execution_roots empty: trace_set_root={}, trace_seq_root={:?}",
        empty_roots.trace_set_root, empty_roots.trace_seq_root
    );
    println!(
        "compute_execution_roots single: trace_set_root={}, trace_seq_root={:?}",
        single_roots.trace_set_root, single_roots.trace_seq_root
    );
    println!(
        "compute_execution_roots multiple: trace_set_root={}, trace_seq_root={:?}",
        multi_roots.trace_set_root, multi_roots.trace_seq_root
    );
}

#[test]
fn test_root_computation_baselines() {
    // Build baseline map for easy lookup
    let baseline_map: HashMap<&str, &str> = BASELINE_ROOTS.iter().cloned().collect();
    let mut mismatches: Vec<(&str, &str, String)> = Vec::new();

    // Test commit_set_root
    let empty_set = commit_set_root(&[]);
    if let Some(expected) = baseline_map.get("commit_set_root_empty") {
        if empty_set != *expected {
            mismatches.push(("commit_set_root_empty", expected, empty_set.clone()));
        }
    }

    let single_set = commit_set_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);
    if let Some(expected) = baseline_map.get("commit_set_root_single") {
        if single_set != *expected {
            mismatches.push(("commit_set_root_single", expected, single_set.clone()));
        }
    }

    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
    let h3 = "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();
    let three_set = commit_set_root(&[h1.clone(), h2.clone(), h3.clone()]);
    if let Some(expected) = baseline_map.get("commit_set_root_three_items") {
        if three_set != *expected {
            mismatches.push(("commit_set_root_three_items", expected, three_set.clone()));
        }
    }

    // Test commit_seq_root
    let empty_seq = commit_seq_root(&[]);
    if let Some(expected) = baseline_map.get("commit_seq_root_empty") {
        if empty_seq != *expected {
            mismatches.push(("commit_seq_root_empty", expected, empty_seq.clone()));
        }
    }

    let single_seq = commit_seq_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);
    if let Some(expected) = baseline_map.get("commit_seq_root_single") {
        if single_seq != *expected {
            mismatches.push(("commit_seq_root_single", expected, single_seq.clone()));
        }
    }

    let three_seq = commit_seq_root(&[h1.clone(), h2.clone(), h3.clone()]);
    if let Some(expected) = baseline_map.get("commit_seq_root_three_items") {
        if three_seq != *expected {
            mismatches.push(("commit_seq_root_three_items", expected, three_seq.clone()));
        }
    }

    // Test compute_tensor_root
    let empty_tensor = compute_tensor_root(&[]);
    if let Some(expected) = baseline_map.get("compute_tensor_root_empty") {
        if empty_tensor != *expected {
            mismatches.push(("compute_tensor_root_empty", expected, empty_tensor.clone()));
        }
    }

    let single_tensor = compute_tensor_root(&[
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    ]);
    if let Some(expected) = baseline_map.get("compute_tensor_root_single") {
        if single_tensor != *expected {
            mismatches.push((
                "compute_tensor_root_single",
                expected,
                single_tensor.clone(),
            ));
        }
    }

    let three_tensor = compute_tensor_root(&[h1.clone(), h2.clone(), h3.clone()]);
    if let Some(expected) = baseline_map.get("compute_tensor_root_three_items") {
        if three_tensor != *expected {
            mismatches.push((
                "compute_tensor_root_three_items",
                expected,
                three_tensor.clone(),
            ));
        }
    }

    // MerkleRowMap tests removed - rowmap module deleted as dead weight

    // Test compute_execution_roots
    let identity_root =
        "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

    let empty_exec = compute_execution_roots(&[], identity_root.clone());
    if let Some(expected) = baseline_map.get("compute_execution_roots_empty_trace_set") {
        if empty_exec.trace_set_root != *expected {
            mismatches.push((
                "compute_execution_roots_empty_trace_set",
                expected,
                empty_exec.trace_set_root.clone(),
            ));
        }
    }
    if let Some(expected) = baseline_map.get("compute_execution_roots_empty_trace_seq") {
        if let Some(ref seq_root) = empty_exec.trace_seq_root {
            if seq_root != *expected {
                mismatches.push((
                    "compute_execution_roots_empty_trace_seq",
                    expected,
                    seq_root.clone(),
                ));
            }
        }
    }

    let single_exec = compute_execution_roots(&[h1.clone()], identity_root.clone());
    if let Some(expected) = baseline_map.get("compute_execution_roots_single_trace_set") {
        if single_exec.trace_set_root != *expected {
            mismatches.push((
                "compute_execution_roots_single_trace_set",
                expected,
                single_exec.trace_set_root.clone(),
            ));
        }
    }
    if let Some(expected) = baseline_map.get("compute_execution_roots_single_trace_seq") {
        if let Some(ref seq_root) = single_exec.trace_seq_root {
            if seq_root != *expected {
                mismatches.push((
                    "compute_execution_roots_single_trace_seq",
                    expected,
                    seq_root.clone(),
                ));
            }
        }
    }

    let multi_exec = compute_execution_roots(&[h1.clone(), h2.clone()], identity_root);
    if let Some(expected) = baseline_map.get("compute_execution_roots_multiple_trace_set") {
        if multi_exec.trace_set_root != *expected {
            mismatches.push((
                "compute_execution_roots_multiple_trace_set",
                expected,
                multi_exec.trace_set_root.clone(),
            ));
        }
    }
    if let Some(expected) = baseline_map.get("compute_execution_roots_multiple_trace_seq") {
        if let Some(ref seq_root) = multi_exec.trace_seq_root {
            if seq_root != *expected {
                mismatches.push((
                    "compute_execution_roots_multiple_trace_seq",
                    expected,
                    seq_root.clone(),
                ));
            }
        }
    }

    if !mismatches.is_empty() {
        eprintln!("\n⚠️  Root computation drift detected!\n");
        eprintln!("This could indicate:");
        eprintln!("  1. Intentional algorithm changes (update BASELINE_ROOTS)");
        eprintln!("  2. Unintended changes to root computation logic");
        eprintln!("  3. Changes to underlying hash functions\n");

        for (test_name, expected, computed) in &mismatches {
            eprintln!("  {}:", test_name);
            eprintln!("    Expected: {}", expected);
            eprintln!("    Computed: {}", computed);
        }

        eprintln!("\nTo update baselines after intentional changes:");
        eprintln!("  1. Run: cargo test --test test_drift_detection -- --nocapture");
        eprintln!("  2. Update BASELINE_ROOTS in test_drift_detection.rs with new values\n");

        panic!(
            "Root computation drift detected in {} test(s)",
            mismatches.len()
        );
    }
}

#[test]
fn test_root_computations_are_deterministic() {
    // Test that root computations are deterministic (same input → same output)

    use northroot_engine::{
        commit_seq_root, commit_set_root, compute_execution_roots, compute_tensor_root,
    };

    let h1 = "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let h2 = "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();

    // commit_set_root
    let root1 = commit_set_root(&[h1.clone(), h2.clone()]);
    let root2 = commit_set_root(&[h1.clone(), h2.clone()]);
    assert_eq!(root1, root2, "commit_set_root should be deterministic");

    // commit_seq_root
    let root3 = commit_seq_root(&[h1.clone(), h2.clone()]);
    let root4 = commit_seq_root(&[h1.clone(), h2.clone()]);
    assert_eq!(root3, root4, "commit_seq_root should be deterministic");

    // compute_tensor_root
    let root5 = compute_tensor_root(&[h1.clone(), h2.clone()]);
    let root6 = compute_tensor_root(&[h1.clone(), h2.clone()]);
    assert_eq!(root5, root6, "compute_tensor_root should be deterministic");

    // MerkleRowMap tests removed - rowmap module deleted as dead weight

    // compute_execution_roots
    let roots1 = compute_execution_roots(&[h1.clone()], h2.clone());
    let roots2 = compute_execution_roots(&[h1.clone()], h2.clone());
    assert_eq!(
        roots1.trace_set_root, roots2.trace_set_root,
        "compute_execution_roots should be deterministic"
    );
    assert_eq!(
        roots1.trace_seq_root, roots2.trace_seq_root,
        "compute_execution_roots should be deterministic"
    );
}
