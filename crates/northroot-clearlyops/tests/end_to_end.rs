use northroot_clearlyops::{validate_clearlyops_record, AG_SEED_EVAL_COMPLETED};
use northroot_exchange::{validate_exchange_record, HANDOFF_SUBMITTED};
use northroot_execution::{MethodDescriptor, MethodRegistry};
use northroot_governance::matching_policies;
use northroot_node::{
    NodeManifest, WorkspaceManifest, NODE_MANIFEST_SCHEMA_V0, WORKSPACE_MANIFEST_SCHEMA_V0,
};
use northroot_record::{
    compute_record_id, seal_segment, validate_record, verify_segment_seal, Context,
    JsonlSegmentReader, JsonlSegmentWriter, Method, MethodKind, Record, RecordRefs, RecordRole,
    Scope, Statement,
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
        node_id: "node:apd_croptrak_2026".to_string(),
        resource_namespace: "resource:".to_string(),
        entity_namespace: "entity:".to_string(),
        journal_path: "journal/".to_string(),
        vault_path: "vault/".to_string(),
        state_path: "state/".to_string(),
    };
    node.validate().unwrap();

    let workspace = WorkspaceManifest {
        schema: WORKSPACE_MANIFEST_SCHEMA_V0.to_string(),
        workspace_id: "workspace:clientops-local".to_string(),
        node_id: node.node_id.clone(),
        custody_classes: vec!["client_sensitive".to_string()],
        journal_path: "journal/clientops-local".to_string(),
        vault_path: "vault/clientops-local".to_string(),
        state_path: "state/clientops-local".to_string(),
    };
    workspace.validate().unwrap();

    let command = with_id(Record::new(
        RecordRole::Command,
        Statement {
            subject: "entity:principal:codex".to_string(),
            predicate: HANDOFF_SUBMITTED.to_string(),
            object: "resource:handoff:seed-report".to_string(),
        },
        Context {
            node_id: Some(node.node_id),
            time: Some("2026-06-14T18:00:00Z".to_string()),
            intent: Some("prepare_client_delivery".to_string()),
            scope: Some(Scope {
                workspace_id: workspace.workspace_id,
                custody_class: "client_sensitive".to_string(),
            }),
            method: Some(Method {
                kind: MethodKind::Tool,
                ref_: "tool:classify-document".to_string(),
            }),
            ..Context::default()
        },
        RecordRefs {
            inputs: vec!["resource:document:seed-report".to_string()],
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
            object: "resource:policy:client-export-review".to_string(),
        },
        Context::default(),
        RecordRefs::default(),
        json!({
            "match": {
                "predicate": HANDOFF_SUBMITTED,
                "custody_class": "client_sensitive",
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
            provider: "clientops-local".to_string(),
        })
        .unwrap();
    registry.validate_record_method(&command).unwrap();

    let result = with_id(Record::new(
        RecordRole::Event,
        Statement {
            subject: "entity:principal:codex".to_string(),
            predicate: AG_SEED_EVAL_COMPLETED.to_string(),
            object: "resource:seed_eval:apd-2026".to_string(),
        },
        Context {
            node_id: Some("node:apd_croptrak_2026".to_string()),
            time: Some("2026-06-14T18:05:00Z".to_string()),
            scope: Some(Scope {
                workspace_id: "workspace:clientops-local".to_string(),
                custody_class: "client_sensitive".to_string(),
            }),
            ..Context::default()
        },
        RecordRefs {
            inputs: vec!["resource:document:seed-report".to_string()],
            outputs: vec!["resource:seed_eval:apd-2026".to_string()],
            causes: vec![format!("event:{}", command.id)],
            ..RecordRefs::default()
        },
        json!({"domain_type": "seed_eval"}),
    ));
    validate_record(&result).unwrap();
    validate_clearlyops_record(&result).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let segment_path = dir.path().join("records.jsonl");
    let mut writer = JsonlSegmentWriter::create(&segment_path, 1).unwrap();
    writer.append(command).unwrap();
    writer.append(policy).unwrap();
    writer.append(result).unwrap();
    writer.flush().unwrap();

    let seal = seal_segment(&segment_path).unwrap();
    assert_eq!(seal.first_seq, 1);
    assert_eq!(seal.last_seq, 3);
    assert_eq!(seal.record_count, 3);
    verify_segment_seal(&segment_path).unwrap();

    let mut reader = JsonlSegmentReader::open(&segment_path).unwrap();
    assert_eq!(reader.read_next().unwrap().unwrap().seq, 1);
    assert_eq!(reader.read_next().unwrap().unwrap().seq, 2);
    assert_eq!(reader.read_next().unwrap().unwrap().seq, 3);
    assert!(reader.read_next().unwrap().is_none());
}
