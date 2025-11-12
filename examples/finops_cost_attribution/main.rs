//! FinOps Cost Attribution - Real End-to-End Example
//!
//! This example demonstrates real data processing with:
//! 1. Reading actual CSV billing data
//! 2. Processing and aggregating data
//! 3. Generating manifests from input/output data
//! 4. Storing receipts and manifests in SQLite
//! 5. Computing reuse decisions from previous executions
//! 6. Generating complete execution receipts
//!
//! Run with: cargo run --example finops_cost_attribution

use northroot_engine::cas::build_manifest_from_data;
use northroot_engine::delta::{
    chunk_id_from_str, compute_data_shape_hash_from_bytes, compute_method_shape_hash_from_signature,
    compute_minhash_sketch, decide_reuse, economic_delta, jaccard_similarity,
    load_cost_model_from_policy,
};
use northroot_engine::shapes::ChunkScheme;
use northroot_receipts::{
    Context, DeterminismClass, ExecutionPayload, MethodRef, Receipt, ReceiptKind,
};
use northroot_storage::{ReceiptStore, SqliteStore};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

/// Resource tuple representing a billing graph node.
type ResourceTuple = (String, String, String, String);

/// Billing record from CSV
#[derive(Debug, Clone)]
struct BillingRecord {
    account_id: String,
    service: String,
    region: String,
    resource_type: String,
    cost_usd: f64,
    usage_units: f64,
}

/// Read billing data from CSV file
fn read_billing_csv(path: &Path) -> Result<Vec<BillingRecord>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut records = Vec::new();
    let mut lines = content.lines();
    
    // Skip header
    lines.next();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            records.push(BillingRecord {
                account_id: parts[0].to_string(),
                service: parts[1].to_string(),
                region: parts[2].to_string(),
                resource_type: parts[3].to_string(),
                cost_usd: parts[4].parse().unwrap_or(0.0),
                usage_units: parts[5].parse().unwrap_or(0.0),
            });
        }
    }
    
    Ok(records)
}

/// Process billing data: aggregate by account
fn process_billing_data(records: &[BillingRecord]) -> Vec<(String, f64, f64)> {
    use std::collections::HashMap;
    let mut by_account: HashMap<String, (f64, f64)> = HashMap::new();
    
    for record in records {
        let entry = by_account.entry(record.account_id.clone()).or_insert((0.0, 0.0));
        entry.0 += record.cost_usd;
        entry.1 += record.usage_units;
    }
    
    by_account
        .into_iter()
        .map(|(account, (cost, usage))| (account, cost, usage))
        .collect()
}

/// Write aggregated results to CSV
fn write_output_csv(
    results: &[(String, f64, f64)],
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "account_id,total_cost_usd,total_usage_units")?;
    for (account, cost, usage) in results {
        writeln!(file, "{},{:.2},{}", account, cost, usage)?;
    }
    Ok(())
}

/// Extract resource tuples from billing records
fn extract_resource_tuples(records: &[BillingRecord]) -> Vec<ResourceTuple> {
    records
        .iter()
        .map(|r| {
            (
                r.account_id.clone(),
                r.service.clone(),
                r.region.clone(),
                r.resource_type.clone(),
            )
        })
        .collect()
}

/// Convert resource tuples to chunk IDs
fn tuples_to_chunk_ids(tuples: &[ResourceTuple]) -> HashSet<String> {
    tuples
        .iter()
        .map(|(acct, svc, region, rtype)| {
            let tuple_str = format!("{}:{}:{}:{}", acct, svc, region, rtype);
            chunk_id_from_str(&tuple_str)
        })
        .collect()
}

/// Process a billing run with real data processing
fn process_billing_run(
    input_file: &Path,
    output_file: &Path,
    run_number: usize,
    store: &SqliteStore,
) -> Result<Receipt, Box<dyn std::error::Error>> {
    println!("\n=== RUN {}: Processing Billing Data ===", run_number);
    println!("Input: {}", input_file.display());
    
    // 1. Read input data
    let records = read_billing_csv(input_file)?;
    println!("  Read {} billing records", records.len());
    
    // 2. Generate input manifest
    let input_data = fs::read(input_file)?;
    let input_manifest = build_manifest_from_data(&input_data, ChunkScheme::Fixed { size: 4096 })?;
    println!("  Input manifest: {} chunks, {} bytes", input_manifest.chunks.len(), input_manifest.manifest_len);
    
    // 3. Compute input data shape hash
    let input_shape_hash = compute_data_shape_hash_from_bytes(&input_data, Some(ChunkScheme::Fixed { size: 4096 }))?;
    println!("  Input data shape hash: {}", input_shape_hash);
    
    // 4. Process data (real computation)
    let results = process_billing_data(&records);
    println!("  Aggregated to {} accounts", results.len());
    
    // 5. Write output
    write_output_csv(&results, output_file)?;
    let output_data = fs::read(output_file)?;
    println!("  Output written: {} bytes", output_data.len());
    
    // 6. Generate output manifest
    let output_manifest = build_manifest_from_data(&output_data, ChunkScheme::Fixed { size: 4096 })?;
    println!("  Output manifest: {} chunks, {} bytes", output_manifest.chunks.len(), output_manifest.manifest_len);
    
    // 7. Compute output data shape hash
    let output_shape_hash = compute_data_shape_hash_from_bytes(&output_data, Some(ChunkScheme::Fixed { size: 4096 }))?;
    println!("  Output data shape hash: {}", output_shape_hash);
    
    // 8. Extract resource tuples and compute chunk sets
    let resource_tuples = extract_resource_tuples(&records);
    let current_chunks = tuples_to_chunk_ids(&resource_tuples);
    
    // 9. Compute MinHash sketch
    let tuple_strings: Vec<String> = resource_tuples
        .iter()
        .map(|(acct, svc, region, rtype)| format!("{}:{}:{}:{}", acct, svc, region, rtype))
        .collect();
    let sketch_hash = compute_minhash_sketch(tuple_strings.iter());
    
    // 10. Compute method shape and PAC
    let method_shape_hash = compute_method_shape_hash_from_signature(
        "aggregate_billing_by_account",
        &["Vec<BillingRecord>"],
        "Vec<(String, f64, f64)>",
    )?;
    
    // Extract PAC from method shape hash (first 32 bytes of hex)
    let pac_hex = method_shape_hash.strip_prefix("sha256:").unwrap_or(&method_shape_hash);
    let pac_bytes = hex::decode(&pac_hex[..64])?;
    let mut pac = [0u8; 32];
    pac.copy_from_slice(&pac_bytes[..32]);
    
    // Query for previous execution with same method shape
    // Use a consistent base trace_id for the method
    let base_trace_id = "tr_finops_aggregate_billing";
    // Query for any previous execution with same PAC (method shape)
    // The trace_id parameter is not used in the current implementation
    let prev_receipt = store.get_previous_execution(&pac, base_trace_id)?;
    
    // 11. Compute overlap with previous execution
    let (overlap_j, prev_chunks) = if let Some(prev) = &prev_receipt {
        println!("  Found previous execution: {}", prev.rid);
        
        // For demo, we'll compute overlap from the previous run's data if available
        // In production, chunk sets would be stored in execution receipts and retrieved from storage
        if run_number > 1 {
            let prev_input_file = input_file.parent().unwrap().join(format!("billing_run{}.csv", run_number - 1));
            if prev_input_file.exists() {
                let prev_records = read_billing_csv(&prev_input_file)?;
                let prev_tuples = extract_resource_tuples(&prev_records);
                let prev_chunks = tuples_to_chunk_ids(&prev_tuples);
                let j = jaccard_similarity(&current_chunks, &prev_chunks);
                (j, prev_chunks)
            } else {
                println!("  Previous input file not found, assuming no overlap");
                (0.0, HashSet::new())
            }
        } else {
            (0.0, HashSet::new())
        }
    } else {
        println!("  No previous execution found (first run)");
        (0.0, HashSet::new())
    };
    
    // 12. Make reuse decision
    let policy_ref = "pol:finops/cost-attribution@1";
    let row_count = Some(records.len());
    let cost_model = load_cost_model_from_policy(policy_ref, row_count)
        .map_err(|e| format!("Failed to load cost model: {}", e))?;
    let (decision, _justification) = decide_reuse(overlap_j, &cost_model, row_count);
    let delta_c = economic_delta(overlap_j, &cost_model, row_count);
    
    println!("\n  REUSE DECISION:");
    println!("    Previous chunks: {}", prev_chunks.len());
    println!("    Current chunks: {}", current_chunks.len());
    println!("    Intersection: {}", current_chunks.intersection(&prev_chunks).count());
    println!("    Union: {}", current_chunks.union(&prev_chunks).count());
    println!("    Jaccard similarity (J): {:.4}", overlap_j);
    println!("    Economic delta (ΔC): ${:.4}", delta_c);
    println!("    Decision: {:?}", decision);
    
    // 13. Store manifests (serialize as JSON manually)
    let input_manifest_json = serde_json::json!({
        "manifest_root": input_manifest.manifest_root,
        "manifest_len": input_manifest.manifest_len,
        "chunks": input_manifest.chunks.iter().map(|c| serde_json::json!({
            "id": c.id,
            "offset": c.offset,
            "len": c.len,
            "hash": hex::encode(c.hash),
        })).collect::<Vec<_>>(),
        "chunk_scheme": match input_manifest.chunk_scheme {
            ChunkScheme::CDC { avg_size } => serde_json::json!({"type": "cdc", "avg_size": avg_size}),
            ChunkScheme::Fixed { size } => serde_json::json!({"type": "fixed", "size": size}),
        },
    });
    let input_manifest_bytes = serde_json::to_vec(&input_manifest_json)?;
    let input_manifest_hash_bytes = hex::decode(input_manifest.manifest_root.strip_prefix("sha256:").unwrap())?;
    let mut input_manifest_hash = [0u8; 32];
    input_manifest_hash.copy_from_slice(&input_manifest_hash_bytes[..32]);
    
    store.put_manifest(
        &input_manifest_hash,
        &input_manifest_bytes,
        &northroot_storage::ManifestMeta {
            pac,
            change_epoch_id: None,
            encoding: "json".to_string(),
            size_uncompressed: input_manifest_bytes.len(),
            expires_at: None,
        },
    )?;
    
    let output_manifest_json = serde_json::json!({
        "manifest_root": output_manifest.manifest_root,
        "manifest_len": output_manifest.manifest_len,
        "chunks": output_manifest.chunks.iter().map(|c| serde_json::json!({
            "id": c.id,
            "offset": c.offset,
            "len": c.len,
            "hash": hex::encode(c.hash),
        })).collect::<Vec<_>>(),
        "chunk_scheme": match output_manifest.chunk_scheme {
            ChunkScheme::CDC { avg_size } => serde_json::json!({"type": "cdc", "avg_size": avg_size}),
            ChunkScheme::Fixed { size } => serde_json::json!({"type": "fixed", "size": size}),
        },
    });
    let output_manifest_bytes = serde_json::to_vec(&output_manifest_json)?;
    let output_manifest_hash_bytes = hex::decode(output_manifest.manifest_root.strip_prefix("sha256:").unwrap())?;
    let mut output_manifest_hash = [0u8; 32];
    output_manifest_hash.copy_from_slice(&output_manifest_hash_bytes[..32]);
    
    store.put_manifest(
        &output_manifest_hash,
        &output_manifest_bytes,
        &northroot_storage::ManifestMeta {
            pac,
            change_epoch_id: None,
            encoding: "json".to_string(),
            size_uncompressed: output_manifest_bytes.len(),
            expires_at: None,
        },
    )?;
    
    // 14. Generate execution receipt
    let rid = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
    let prev_rid = prev_receipt.as_ref().map(|r| r.rid);
    let method_ref = MethodRef {
        method_id: "aggregate_billing_by_account".to_string(),
        version: "1.0.0".to_string(),
        method_shape_root: method_shape_hash.clone(),
    };
    
    let receipt = Receipt {
        rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: input_shape_hash.clone(),
        cod: output_shape_hash.clone(),
        links: prev_rid.into_iter().collect(),
        ctx: Context {
            policy_ref: Some(policy_ref.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zFinOps".to_string()),
        },
        payload: northroot_receipts::Payload::Execution(ExecutionPayload {
            trace_id: format!("{}_{}", base_trace_id, run_number),
            method_ref,
            data_shape_hash: input_shape_hash,
            span_commitments: vec![],
            roots: northroot_receipts::ExecutionRoots {
                trace_set_root: output_shape_hash.clone(),
                identity_root: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                trace_seq_root: None,
            },
            cdf_metadata: None,
            pac: Some(pac),
            change_epoch: None,
            minhash_signature: Some(hex::decode(sketch_hash.strip_prefix("sha256:").unwrap_or(&sketch_hash))?),
            hll_cardinality: Some(records.len() as u64),
            chunk_manifest_hash: Some(input_manifest_hash),
            chunk_manifest_size_bytes: Some(input_manifest_bytes.len() as u64),
            merkle_root: Some(output_manifest_hash),
            prev_execution_rid: prev_rid,
            output_digest: Some(output_shape_hash.clone()),
            manifest_root: Some(output_manifest_hash),
            output_mime_type: Some("text/csv".to_string()),
            output_size_bytes: Some(output_data.len() as u64),
            input_locator_refs: None,
            output_locator_ref: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    
    let hash = receipt.compute_hash()?;
    let receipt = Receipt { hash, ..receipt };
    
    // 15. Store receipt
    store.store_receipt(&receipt)?;
    println!("\n  Receipt stored: {}", receipt.rid);
    println!("  Receipt hash: {}", receipt.hash);
    
    // Return receipt (prev_receipt was only borrowed)
    Ok(receipt)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("FinOps Cost Attribution - Real End-to-End Example");
    println!("==================================================\n");
    
    // Initialize storage
    let db_path = "examples/finops_cost_attribution/northroot.db";
    let store = SqliteStore::new(db_path)?;
    println!("Storage initialized: {}", db_path);
    
    // Process Run 1
    let input1 = Path::new("examples/finops_cost_attribution/data/billing_run1.csv");
    let output1 = Path::new("examples/finops_cost_attribution/output/aggregated_run1.csv");
    let receipt1 = process_billing_run(input1, output1, 1, &store)?;
    
    println!("\n=== RUN 1 SUMMARY ===");
    println!("Receipt RID: {}", receipt1.rid);
    println!("Receipt Hash: {}", receipt1.hash);
    if let northroot_receipts::Payload::Execution(exec) = &receipt1.payload {
        println!("Trace ID: {}", exec.trace_id);
        println!("Method: {}", exec.method_ref.method_id);
        println!("Input Shape: {}", exec.data_shape_hash);
        println!("Output Shape: {}", exec.roots.trace_set_root);
    }
    
    // Process Run 2 (with overlap)
    let input2 = Path::new("examples/finops_cost_attribution/data/billing_run2.csv");
    let output2 = Path::new("examples/finops_cost_attribution/output/aggregated_run2.csv");
    let receipt2 = process_billing_run(input2, output2, 2, &store)?;
    
    println!("\n=== RUN 2 SUMMARY ===");
    println!("Receipt RID: {}", receipt2.rid);
    println!("Receipt Hash: {}", receipt2.hash);
    println!("Links to: {:?}", receipt2.links);
    if let northroot_receipts::Payload::Execution(exec) = &receipt2.payload {
        println!("Trace ID: {}", exec.trace_id);
        println!("Previous Execution RID: {:?}", exec.prev_execution_rid);
            if let Some(mh) = &exec.minhash_signature {
                println!("MinHash Sketch: {}", hex::encode(mh));
            }
    }
    
    // Verify receipts are linked
    println!("\n=== VERIFICATION ===");
    println!("✓ Run 2 receipt links to Run 1: {}", receipt2.links.contains(&receipt1.rid));
    println!("✓ Both receipts stored in database");
    println!("✓ Manifests stored for both runs");
    println!("✓ Reuse decisions computed from real data");
    
    Ok(())
}
