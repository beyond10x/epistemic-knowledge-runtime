use super::*;
use ekr_core::contracts::{graph as g, kernel as k, ontology as o, primitives as p, store as s};
use serde_json::{json, Value};

fn id() -> String {
    ekr_core::NodeId::mint().to_string()
}
fn hash(label: &str) -> String {
    ekr_core::ContentHash::of_bytes(label.as_bytes()).to_string()
}
fn fixture() -> Value {
    let source = json!({"interpretation_id":id(),"version":17,"document_digest":hash("source")});
    let item = json!({"source":source,"item":"facts[3]","mapping_digest":hash("mapping")});
    let value =
        json!({"kind":"String","canonical_bytes":ekr_core::bytes::encode(b"\x00opaque\xff")});
    let property = json!({"id":id(),"name":"health","cardinality":"Many","required":false,"constraints":["keep"],"value_type":{
        "kind":"Record","fields":{"nested":{"kind":"List","element":{"kind":"Enum","variants":["a","b"]}}}
    }});
    let transaction = json!({"id":id(),"proposer":id(),"evidence":[id()],"schema_version":id(),"operations":[
        {"kind":"Invoke","value":{"node":id(),"operation":"inspect","arguments":{"value":value}}},
        {"kind":"DefineNodeType","value":{"id":id(),"name":"WorkItem","parents":[id()],"properties":{"health":property},"abstract_type":true,
            "lifecycle":{"initial":"Open","states":["Open","Closed"],"transitions":[{"from":"Open","to":"Closed"}]},
            "operations":{"close":{"name":"close","arguments":{"note":{"kind":"String"}},"preconditions":["ready"],"transition":{"from":"Open","to":"Closed"},"emits":["Closed"]}}}},
        {"kind":"AddEvidence","value":{"payload":ekr_core::bytes::encode(b"\x00source\xff"),"evidence":{"id":id(),"content_hash":hash("evidence"),"extracted_by":id(),"observed_at":"2026-10-03T04:05:06.123456789+02:00","confidence_bp":7123,
            "source":{"kind":"DatabaseRecord","database":"db","table":"table","key":"row","identity":"retained optional field"}}}},
        {"kind":"AddAssertion","value":{"id":id(),"root_id":id(),"subject":{"kind":"Edge","id":id()},"predicate":{"kind":"Relation","id":id()},"object":{"kind":"Value","value":value,"reference":id()},"evidence":[id()],"proposed_by":id(),
            "assessment":{"kind":"Disputed","completed":2,"required":3,"validators":[id()],"issues":[id()],"competing_assertions":[id()]},
            "lifecycle":{"kind":"Superseded","at_revision":13,"reason":"reviewed","by":id(),"effective_from":"2026-09-30T00:00:00Z"},
            "valid_time":{"from":"2026-09-01T00:00:00Z"},"transaction_time":{"recorded_from":"2026-10-01T00:00:00Z","recorded_to":"2026-10-02T00:00:00Z"}}}
    ]});
    let step = json!({"kind":"Mapping","item":item});
    let receipt = json!({"application_id":id(),"receipt_id":id(),"proposal_id":id(),"review_id":id(),"progress":"Partial","schema_transaction":id(),"schema_revision":91,
        "processing_receipts":[id()],"remaining_items":[item],"corrections_pending":true,"stop_reason":"review changed"});
    json!({
        "election":{"application_id":id(),"proposal_id":id(),"proposal_digest":hash("proposal"),"initial_review_id":id(),"initial_proof_digest":hash("proof"),"base_schema":id(),"elected_at":"2026-10-03T00:00:00Z","schema_transaction":transaction,"selected_items":[item]},
        "steps":[{"step_election_id":id(),"application_id":id(),"step":step,"transaction":transaction,"elected_at":"2026-10-03T01:00:00Z","replacements":[{"previous":id(),"replacement":id()}],
            "mappings":[{"mapping_id":id(),"proposal_digest":hash("proposal"),"mapping_digest":hash("mapping"),"source_document_digest":hash("source"),"evidence":[id()],"payload":ekr_core::bytes::encode(b"mapping payload"),
                "mapping":{"source":source,"source_item":"facts[3]","source_type":"WorkItem","target_type":"WorkItem","target_member":"health","value":{"kind":"CopyField","value":{"declaration":"WorkItem","field":"health"}}}}],
            "derivations":[{"derivation_id":id(),"assertion_id":id(),"mapping_id":id(),"observation_id":id(),"evidence_id":id()}]}],
        "attempts":[{"transaction_id":id(),"step_election_id":id(),"predecessor_transaction":id(),"predecessor_record_hash":hash("predecessor"),"transaction":transaction,"elected_at":"2026-10-03T02:00:00Z"}],
        "publications":[{"guard":{"application_id":id(),"step_election_id":id(),"attempt_transaction":id(),"proposal_id":id(),"proposal_digest":hash("proposal"),"review_id":id(),"human_proof_digest":hash("proof"),"review_stream_version":29,"step":step},"transaction_id":id(),"event_id":id(),"record_hash":hash("publication"),"command":"Commit"}],
        "receipts":[receipt],"remaining_items":[item],"corrections_pending":true
    })
}
fn wire(value: Value) -> w::EkrIntegrateApplicationRead {
    serde_json::from_value(value).unwrap()
}
fn id_at(value: &Value, path: &str) -> String {
    value.pointer(path).unwrap().as_str().unwrap().into()
}

#[test]
fn mixed_application_history_keeps_nested_payloads_and_coordinates() {
    let fixture = fixture();
    let actual = read(&wire(fixture.clone())).unwrap();
    assert_eq!(
        actual.election.application_id.0 .0,
        id_at(&fixture, "/election/application_id")
    );
    assert_eq!(
        actual.election.initial_review_id.0 .0,
        id_at(&fixture, "/election/initial_review_id")
    );
    assert_eq!(actual.election.proposal_digest.0, hash("proposal"));
    assert_eq!(actual.election.initial_proof_digest.0, hash("proof"));
    assert_eq!(
        actual.election.base_schema.0 .0,
        id_at(&fixture, "/election/base_schema")
    );
    assert_eq!(actual.election.elected_at.0, "2026-10-03T00:00:00Z");
    let item = m::ApplicationItemKey {
        source: m::InterpretationVersion {
            interpretation_id: m::InterpretationId(p::Uuid(id_at(
                &fixture,
                "/remaining_items/0/source/interpretation_id",
            ))),
            version: 17,
            document_digest: k::ContentHash(hash("source")),
        },
        item: "facts[3]".into(),
        mapping_digest: k::ContentHash(hash("mapping")),
    };
    assert_eq!(actual.election.selected_items, vec![item.clone()]);
    assert_eq!(actual.remaining_items, vec![item.clone()]);
    assert!(actual.corrections_pending);
    assert_eq!(actual.steps.len(), 1);
    let step = &actual.steps[0];
    assert_eq!(
        step.step,
        m::ApplicationStep {
            kind: m::ApplicationStepKind::Mapping,
            item: Some(item.clone())
        }
    );
    assert_eq!(step.transaction, actual.election.schema_transaction);
    assert_eq!(
        step.replacements[0].previous.0 .0,
        id_at(&fixture, "/steps/0/replacements/0/previous")
    );
    assert_eq!(
        step.replacements[0].replacement.0 .0,
        id_at(&fixture, "/steps/0/replacements/0/replacement")
    );
    assert_eq!(step.mappings[0].payload, b"mapping payload");
    assert_eq!(
        step.mappings[0].mapping.value,
        m::MappingValue::CopyField(m::DeclaredSourceField {
            declaration: "WorkItem".into(),
            field: "health".into()
        })
    );
    assert_eq!(step.mappings[0].mapping.source, item.source);
    assert_eq!(
        step.derivations[0].observation_id.as_ref().unwrap().0 .0,
        id_at(&fixture, "/steps/0/derivations/0/observation_id")
    );
    assert_eq!(actual.attempts[0].transaction, step.transaction);
    assert_eq!(
        actual.attempts[0].predecessor_record_hash,
        Some(k::ContentHash(hash("predecessor")))
    );
    assert_eq!(
        actual.attempts[0]
            .predecessor_transaction
            .as_ref()
            .unwrap()
            .0
             .0,
        id_at(&fixture, "/attempts/0/predecessor_transaction")
    );
    let publication = &actual.publications[0];
    assert_eq!(publication.command, s::PublicationCommandKind::Commit);
    assert_eq!(publication.guard.review_stream_version, 29);
    assert_eq!(publication.guard.step, step.step);
    assert_eq!(
        publication.guard.attempt_transaction.0 .0,
        id_at(&fixture, "/publications/0/guard/attempt_transaction")
    );
    assert_eq!(publication.record_hash.0, hash("publication"));
    assert_eq!(actual.receipts[0].progress, m::ApplicationProgress::Partial);
    assert_eq!(actual.receipts[0].schema_revision, k::RevisionNumber(91));
    assert_eq!(actual.receipts[0].remaining_items, vec![item]);
    assert_eq!(
        actual.receipts[0].stop_reason,
        Some("review changed".into())
    );
    assert!(actual.receipts[0].corrections_pending);
    let operations = &step.transaction.operations;
    let k::CanonicalOperationProjection::Invoke(invoke) = &operations[0] else {
        panic!("invoke tag changed")
    };
    assert_eq!(invoke.arguments["value"].canonical_bytes, b"\x00opaque\xff");
    let k::CanonicalOperationProjection::DefineNodeType(node) = &operations[1] else {
        panic!("node tag changed")
    };
    assert!(node.abstract_type);
    assert_eq!(node.operations["close"].emits, vec!["Closed"]);
    let value_type = &node.properties["health"].value_type;
    assert_eq!(value_type.kind, o::ValueKind::Record);
    assert_eq!(
        value_type.fields.as_ref().unwrap()["nested"]
            .element
            .as_ref()
            .unwrap()
            .variants,
        Some(vec!["a".into(), "b".into()])
    );
    let k::CanonicalOperationProjection::AddEvidence(evidence) = &operations[2] else {
        panic!("evidence tag changed")
    };
    assert_eq!(evidence.payload, b"\x00source\xff");
    assert_eq!(evidence.evidence.confidence_bp, 7123);
    assert_eq!(
        evidence.evidence.observed_at.0,
        "2026-10-03T04:05:06.123456789+02:00"
    );
    assert_eq!(
        evidence.evidence.source.identity,
        Some("retained optional field".into())
    );
    let k::CanonicalOperationProjection::AddAssertion(assertion) = &operations[3] else {
        panic!("assertion tag changed")
    };
    assert_eq!(assertion.assessment.kind, g::AssessmentKind::Disputed);
    assert_eq!(assertion.assessment.completed, Some(2));
    assert_eq!(assertion.assessment.required, Some(3));
    assert_eq!(assertion.assessment.validators.as_ref().unwrap().len(), 1);
    assert_eq!(assertion.assessment.issues.as_ref().unwrap().len(), 1);
    assert_eq!(
        assertion.object.reference.as_ref().unwrap().0,
        id_at(
            &fixture,
            "/steps/0/transaction/operations/3/value/object/reference"
        )
    );
    assert_eq!(assertion.lifecycle.reason, Some("reviewed".into()));
    assert_eq!(assertion.valid_time.to, None);
}

#[test]
fn numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing() {
    for (path, value) in [
        (
            "/publications/0/guard/review_stream_version",
            json!(u64::MAX),
        ),
        ("/receipts/0/schema_revision", json!(u64::MAX)),
        ("/remaining_items/0/source/version", json!(1.5)),
        (
            "/steps/0/transaction/operations/2/value/evidence/confidence_bp",
            json!(u64::MAX),
        ),
        ("/steps/0/mappings/0/payload", json!("invalid-base64")),
        (
            "/steps/0/transaction/operations/0/value/arguments/value/canonical_bytes",
            json!("invalid-base64"),
        ),
    ] {
        let mut fixture = fixture();
        *fixture.pointer_mut(path).unwrap() = value;
        assert!(read(&wire(fixture)).is_err(), "accepted {path}");
    }
}

#[test]
fn absent_optionals_and_empty_collections_remain_distinct() {
    let mut fixture = fixture();
    fixture["steps"][0]["derivations"][0]
        .as_object_mut()
        .unwrap()
        .remove("observation_id");
    fixture["attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("predecessor_transaction");
    fixture["attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("predecessor_record_hash");
    fixture["receipts"][0]
        .as_object_mut()
        .unwrap()
        .remove("stop_reason");
    fixture["remaining_items"] = json!([]);
    fixture["corrections_pending"] = json!(false);
    let actual = read(&wire(fixture)).unwrap();
    assert_eq!(actual.steps[0].derivations[0].observation_id, None);
    assert_eq!(actual.attempts[0].predecessor_transaction, None);
    assert_eq!(actual.attempts[0].predecessor_record_hash, None);
    assert_eq!(actual.receipts[0].stop_reason, None);
    assert!(actual.remaining_items.is_empty());
    assert!(!actual.corrections_pending);
}

#[test]
fn proposal_show_exposes_receipts_and_application_history() {
    let application = fixture();
    let input: w::EkrIntegrateSchemaProposalRead = serde_json::from_value(json!({
        "proposal":{"proposal_id":id(),"base_schema":id(),"observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[],"corrections":[],"explanation":"retained proposal"},
        "proposal_digest":hash("proposal"),"preview":[],"reviews":[],"receipts":application["receipts"],
        "basis":{"observed_revision":93,"evidence_digest":hash("evidence"),"options_digest":hash("options"),"effects_digest":hash("effects")},
        "expected_previous_decision":hash("previous"),"application":application
    })).unwrap();
    let shown = crate::schema_proposal_projection::shown(&input)
        .expect("application history remains inspectable");
    assert_eq!(shown.receipts.len(), 1);
    assert_eq!(
        shown.receipts[0].stop_reason.as_deref(),
        Some("review changed")
    );
    assert_eq!(shown.application, Some(read(&wire(application)).unwrap()));
    assert_eq!(shown.basis.observed_revision, k::RevisionNumber(93));
    assert_eq!(
        shown.expected_previous_decision,
        Some(k::ContentHash(hash("previous")))
    );
}

#[test]
fn every_operation_union_alternative_keeps_its_tag() {
    let mut fixture = fixture();
    let original = fixture["election"]["schema_transaction"]["operations"]
        .as_array()
        .unwrap()
        .clone();
    let property = json!({"id":id(),"name":"note","value_type":{"kind":"String"},"cardinality":"One","required":false,"constraints":[]});
    let mut operations = original;
    operations.extend([
        json!({"kind":"AddAlias","value":{"node":id(),"alias":"alias"}}),
        json!({"kind":"AttachEvidence","value":{"assertion":id(),"evidence":id()}}),
        json!({"kind":"CreateEdge","value":{"id":id(),"root_id":id(),"type_id":id(),"source":id(),"target":id(),"properties":{}}}),
        json!({"kind":"CreateNode","value":{"id":id(),"root_id":id(),"type_id":id(),"canonical_name":"name","properties":{},"aliases":[]}}),
        json!({"kind":"DefineEdgeType","value":{"id":id(),"name":"RelatesTo","source_types":[id()],"target_types":[id()],"cardinality":"Many","properties":{"note":property},"inverse":id(),"symmetric":true,"transitive":false}}),
        json!({"kind":"DeleteEdge","value":id()}),
        json!({"kind":"MergeEntity","value":{"absorbed":id(),"into":id()}}),
        json!({"kind":"ModifyProperty","value":{"owner":id(),"property":property}}),
        json!({"kind":"RetractAssertion","value":{"assertion":id(),"reason":"reviewed"}}),
        json!({"kind":"SupersedeAssertion","value":{"assertion":id(),"by":id(),"effective_from":"2026-10-04T00:00:00Z"}}),
        json!({"kind":"UpdateProperty","value":{"node":id(),"property":id(),"values":[]}}),
        json!({"kind":"WidenEdgeType","value":{"edge_type":id(),"source_types":[id()],"target_types":[id()]}}),
    ]);
    fixture["election"]["schema_transaction"]["operations"] = json!(operations);
    let actual = read(&wire(fixture)).unwrap();
    let kinds: Vec<_> = actual
        .election
        .schema_transaction
        .operations
        .iter()
        .map(|op| match op {
            k::CanonicalOperationProjection::AddAlias(_) => "AddAlias",
            k::CanonicalOperationProjection::AddAssertion(_) => "AddAssertion",
            k::CanonicalOperationProjection::AddEvidence(_) => "AddEvidence",
            k::CanonicalOperationProjection::AttachEvidence(_) => "AttachEvidence",
            k::CanonicalOperationProjection::CreateEdge(_) => "CreateEdge",
            k::CanonicalOperationProjection::CreateNode(_) => "CreateNode",
            k::CanonicalOperationProjection::DefineEdgeType(_) => "DefineEdgeType",
            k::CanonicalOperationProjection::DefineNodeType(_) => "DefineNodeType",
            k::CanonicalOperationProjection::DeleteEdge(_) => "DeleteEdge",
            k::CanonicalOperationProjection::Invoke(_) => "Invoke",
            k::CanonicalOperationProjection::MergeEntity(_) => "MergeEntity",
            k::CanonicalOperationProjection::ModifyProperty(_) => "ModifyProperty",
            k::CanonicalOperationProjection::RetractAssertion(_) => "RetractAssertion",
            k::CanonicalOperationProjection::SupersedeAssertion(_) => "SupersedeAssertion",
            k::CanonicalOperationProjection::UpdateProperty(_) => "UpdateProperty",
            k::CanonicalOperationProjection::WidenEdgeType(_) => "WidenEdgeType",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "Invoke",
            "DefineNodeType",
            "AddEvidence",
            "AddAssertion",
            "AddAlias",
            "AttachEvidence",
            "CreateEdge",
            "CreateNode",
            "DefineEdgeType",
            "DeleteEdge",
            "MergeEntity",
            "ModifyProperty",
            "RetractAssertion",
            "SupersedeAssertion",
            "UpdateProperty",
            "WidenEdgeType"
        ]
    );
    let k::CanonicalOperationProjection::CreateNode(node) =
        &actual.election.schema_transaction.operations[7]
    else {
        unreachable!()
    };
    assert_eq!(node.aliases, Some(vec![]));
}
