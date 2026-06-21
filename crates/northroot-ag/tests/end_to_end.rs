use northroot_ag::{validate_ag_record, AG_SEED_EVAL_COMPLETED};
use northroot_exchange::{validate_exchange_record, HANDOFF_SUBMITTED};
use northroot_execution::{MethodDescriptor, MethodRegistry};
use northroot_governance::matching_policies;
use northroot_journal::WriteOptions;
use northroot_node::{
    NodeManifest, WorkspaceManifest, NODE_MANIFEST_SCHEMA_V0, WORKSPACE_MANIFEST_SCHEMA_V0,
};
use northroot_record::{
    compute_record_id, export_nrj_records_to_jsonl_segment, import_jsonl_segment_to_nrj_records,
    validate_record, verify_jsonl_segment, verify_nrj_record_stream, Context, Method, MethodKind,
    NrjRecordWriter, Record, RecordRefs, RecordRole, Scope, Statement,
};
use serde_json::json;

fn with_id(mut record: Record) -> Record {
    record.id = compute_record_id(&record).unwrap();
    record
}

#[test]
fn record_stack_supports_end_to_end_without_core_domain_semantics() {
    let node = NodeManifest {
        schema: NODE_MANIFEST_SCHEMA_V0.to_string(),
        node_id: "node:ag_demo_2026".to_string(),
        resource_namespace: "resource:".to_string(),
        entity_namespace: "entity:".to_string(),
        journal_path: "journal/".to_string(),
        vault_path: "vault/".to_string(),
        state_path: "state/".to_string(),
    };
    node.validate().unwrap();

    let workspace = WorkspaceManifest {
        schema: WORKSPACE_MANIFEST_SCHEMA_V0.to_string(),
        workspace_id: "workspace:ag-demo".to_string(),
        node_id: node.node_id.clone(),
        custody_classes: vec!["restricted".to_string()],
        journal_path: "journal/ag-demo".to_string(),
        vault_path: "vault/ag-demo".to_string(),
        state_path: "state/ag-demo".to_string(),
    };
    workspace.validate().unwrap();

    let command = with_id(Record::new(
        RecordRole::Command,
        Statement {
            subject: "entity:principal:operator".to_string(),
            predicate: HANDOFF_SUBMITTED.to_string(),
            object: "resource:handoff:agronomy-report".to_string(),
        },
        Context {
            node_id: Some(node.node_id),
            time: Some("2026-06-14T18:00:00Z".to_string()),
            intent: Some("prepare_field_review".to_string()),
            scope: Some(Scope {
                workspace_id: workspace.workspace_id,
                custody_class: "restricted".to_string(),
            }),
            method: Some(Method {
                kind: MethodKind::Tool,
                ref_: "tool:classify-document".to_string(),
            }),
            ..Context::default()
        },
        RecordRefs {
            inputs: vec!["resource:document:agronomy-report".to_string()],
            ..RecordRefs::default()
        },
        json!({}),
    ));
    validate_record(&command).unwrap();
    validate_exchange_record(&command).unwrap();

    let policy = with_id(Record::new(
        RecordRole::Policy,
        Statement {
            subject: "entity:governance:local".to_string(),
            predicate: "policy.applies".to_string(),
            object: "resource:policy:field-review".to_string(),
        },
        Context::default(),
        RecordRefs::default(),
        json!({
            "match": {
                "predicate": HANDOFF_SUBMITTED,
                "custody_class": "restricted",
                "method_kind": "tool"
            },
            "effect": "requires_review"
        }),
    ));
    let matches = matching_policies([&policy], &command).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].effect.as_deref(), Some("requires_review"));

    let mut registry = MethodRegistry::default();
    registry
        .register(MethodDescriptor {
            kind: MethodKind::Tool,
            ref_: "tool:classify-document".to_string(),
            provider: "ag-demo-local".to_string(),
        })
        .unwrap();
    registry.validate_record_method(&command).unwrap();

    let result = with_id(Record::new(
        RecordRole::Event,
        Statement {
            subject: "entity:principal:operator".to_string(),
            predicate: AG_SEED_EVAL_COMPLETED.to_string(),
            object: "resource:seed_eval:trial-2026".to_string(),
        },
        Context {
            node_id: Some("node:ag_demo_2026".to_string()),
            time: Some("2026-06-14T18:05:00Z".to_string()),
            scope: Some(Scope {
                workspace_id: "workspace:ag-demo".to_string(),
                custody_class: "restricted".to_string(),
            }),
            ..Context::default()
        },
        RecordRefs {
            inputs: vec!["resource:document:agronomy-report".to_string()],
            outputs: vec!["resource:seed_eval:trial-2026".to_string()],
            causes: vec![format!("event:{}", command.id)],
            ..RecordRefs::default()
        },
        json!({"domain_type": "seed_eval"}),
    ));
    validate_record(&result).unwrap();
    validate_ag_record(&result).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let nrj_path = dir.path().join("records.nrj");
    let segment_path = dir.path().join("records-export.jsonl");
    let mut writer = NrjRecordWriter::open(&nrj_path, WriteOptions::default()).unwrap();
    writer.append(command).unwrap();
    writer.append(policy).unwrap();
    writer.append(result).unwrap();
    writer.finish().unwrap();

    let nrj_summary = verify_nrj_record_stream(&nrj_path).unwrap();
    assert_eq!(nrj_summary.record_count, 3);
    assert_eq!(nrj_summary.first_seq, Some(1));
    assert_eq!(nrj_summary.last_seq, Some(3));

    let seal = export_nrj_records_to_jsonl_segment(&nrj_path, &segment_path).unwrap();
    assert_eq!(seal.first_seq, 1);
    assert_eq!(seal.last_seq, 3);
    assert_eq!(seal.record_count, 3);
    assert_eq!(
        seal.source_journal_ref.as_deref(),
        Some(nrj_path.to_str().unwrap())
    );

    let jsonl_verification = verify_jsonl_segment(&segment_path, true).unwrap();
    assert!(jsonl_verification.valid);
    assert_eq!(jsonl_verification.source.valid, Some(true));

    let imported_nrj_path = dir.path().join("records-imported.nrj");
    let import_summary = import_jsonl_segment_to_nrj_records(
        &segment_path,
        &imported_nrj_path,
        WriteOptions::default(),
    )
    .unwrap();
    assert_eq!(import_summary.imported_record_count, 3);
    assert_eq!(import_summary.input_first_seq, Some(1));
    assert_eq!(import_summary.input_last_seq, Some(3));
    assert_eq!(import_summary.output_first_seq, Some(1));
    assert_eq!(import_summary.output_last_seq, Some(3));
    assert_eq!(
        verify_nrj_record_stream(&imported_nrj_path)
            .unwrap()
            .record_count,
        3
    );
}
