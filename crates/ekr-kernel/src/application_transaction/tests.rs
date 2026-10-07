use super::*;
use crate::transaction::*;
use ekr_core::canonical::Canonical;
use ekr_core::*;
use ekr_graph::*;
use ekr_ontology::{
    Cardinality, EdgeType, Lifecycle, NodeType, OperationDefinition, PropertyDefinition,
    Transition, ValueType,
};
use serde_json::{json, Value};

fn transaction() -> GraphTransaction<CanonicalValue> {
    let proposer = AgentId::mint();
    let root = GraphRootId::mint();
    let node = NodeId::mint();
    let edge = EdgeId::mint();
    let type_id = TypeId::mint();
    let property_id = PropertyId::mint();
    let evidence_id = EvidenceId::mint();
    let assertion_id = AssertionId::mint();
    let values = vec![CanonicalValue::Record(
        [(
            "nested".into(),
            CanonicalValue::List(vec![
                CanonicalValue::String("retained\nbytes".into()),
                CanonicalValue::Boolean(true),
                CanonicalValue::Integer(i64::MIN),
                CanonicalValue::Decimal("1.250".into()),
                CanonicalValue::Timestamp(Timestamp::from_millis(-1234)),
                CanonicalValue::Duration(-17),
                CanonicalValue::NodeRef(CanonicalRef::new(node)),
                CanonicalValue::Enum("amber".into()),
            ]),
        )]
        .into(),
    )];
    let declaration = PropertyDefinition {
        id: property_id,
        name: "details".into(),
        value_type: ValueType::Record(
            [(
                "members".into(),
                ValueType::List(Box::new(ValueType::NodeRef {
                    allowed_types: [type_id].into(),
                })),
            )]
            .into(),
        ),
        cardinality: Cardinality::Many,
        required: true,
        constraints: vec!["opaque expression".into()],
    };
    let mut node_type = NodeType::new(type_id, "WorkItem");
    node_type.parents.insert(TypeId::mint());
    node_type.abstract_type = true;
    node_type
        .properties
        .insert(property_id, declaration.clone());
    node_type.lifecycle = Some(Lifecycle {
        initial: "Open".into(),
        states: ["Open".into(), "Closed".into()].into(),
        transitions: [Transition {
            from: "Open".into(),
            to: "Closed".into(),
        }]
        .into(),
    });
    node_type.operations.insert(
        "close".into(),
        OperationDefinition {
            name: "close".into(),
            arguments: [(
                "resolution".into(),
                ValueType::Enum {
                    variants: ["Done".into(), "Withdrawn".into()].into(),
                },
            )]
            .into(),
            preconditions: vec!["ready".into()],
            transition: Some(Transition {
                from: "Open".into(),
                to: "Closed".into(),
            }),
            emits: vec!["Closed".into()],
        },
    );
    let mut edge_type = EdgeType::new(TypeId::mint(), "RelatesTo");
    edge_type.source_types.insert(type_id);
    edge_type.target_types.insert(type_id);
    edge_type
        .properties
        .insert(property_id, declaration.clone());
    edge_type.cardinality = Cardinality::Many;
    edge_type.inverse = Some(TypeId::mint());
    edge_type.symmetric = true;
    edge_type.transitive = true;
    let claim = Assertion {
        id: assertion_id,
        root_id: root,
        subject: Subject::Node(CanonicalRef::new(node)),
        predicate: Predicate::Property(property_id),
        object: Object::Value(values[0].clone()),
        evidence: [CanonicalRef::new(evidence_id)].into(),
        proposed_by: proposer,
        assessment: Assessment::Disputed {
            competing_assertions: vec![CanonicalRef::new(AssertionId::mint())],
        },
        lifecycle: AssertionLifecycle::Superseded {
            by: CanonicalRef::new(AssertionId::mint()),
            at_revision: RevisionNumber::new(8),
            effective_from: Timestamp::from_millis(20),
        },
        valid_time: TemporalRange::new(
            Some(Timestamp::from_millis(-20)),
            Some(Timestamp::from_millis(20)),
        )
        .unwrap(),
        transaction_time: TransactionTime::new(
            Timestamp::from_millis(3),
            Some(Timestamp::from_millis(99)),
        )
        .unwrap(),
    };
    let payload = b"\x00retained evidence\xff".to_vec();
    GraphTransaction {
        id: TransactionId::mint(),
        proposer,
        evidence: [evidence_id].into(),
        schema_version: Some(SchemaVersionId::mint()),
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id: node,
                root_id: root,
                type_id,
                canonical_name: "Sample".into(),
                properties: [(property_id, values.clone())].into(),
                aliases: vec!["sample-1".into(), "sample-1".into()],
            }),
            GraphOperation::UpdateProperty(PropertyMutation {
                node,
                property: property_id,
                values: values.clone(),
            }),
            GraphOperation::CreateEdge(EdgeDraft {
                id: edge,
                root_id: root,
                type_id: edge_type.id,
                source: node,
                target: NodeId::mint(),
                properties: [(property_id, values.clone())].into(),
            }),
            GraphOperation::DeleteEdge(edge),
            GraphOperation::AddAssertion(Box::new(claim)),
            GraphOperation::RetractAssertion(Retraction {
                assertion: assertion_id,
                reason: RetractionReason::new("corrected"),
            }),
            GraphOperation::DefineNodeType(Box::new(node_type)),
            GraphOperation::DefineEdgeType(Box::new(edge_type)),
            GraphOperation::ModifyProperty(PropertyModification {
                owner: Some(type_id),
                property: declaration,
            }),
            GraphOperation::MergeEntity(EntityMerge {
                absorbed: node,
                into: NodeId::mint(),
            }),
            GraphOperation::Invoke {
                node,
                operation: "close".into(),
                arguments: [("reason".into(), values[0].clone())].into(),
            },
            GraphOperation::SupersedeAssertion(Supersession {
                assertion: assertion_id,
                by: AssertionId::mint(),
                effective_from: Timestamp::from_millis(20),
            }),
            GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                evidence: Evidence {
                    id: evidence_id,
                    source: EvidenceSource::Document {
                        document_id: "sample-doc".into(),
                        section: Some("summary".into()),
                    },
                    content_hash: ContentHash::of_bytes(&payload),
                    extracted_by: proposer,
                    observed_at: Timestamp::from_millis(-99),
                    confidence: Confidence::from_basis_points(7319).unwrap(),
                },
                payload,
            })),
            GraphOperation::WidenEdgeType(EdgeWidening {
                edge_type: TypeId::mint(),
                source_types: [type_id].into(),
                target_types: [type_id, TypeId::mint()].into(),
            }),
            GraphOperation::AddAlias(AliasAddition {
                node,
                alias: "also-sample".into(),
            }),
            GraphOperation::AttachEvidence(EvidenceAttachment {
                assertion: assertion_id,
                evidence: evidence_id,
            }),
        ],
    }
}

#[test]
fn all_operations_preserve_native_fields_and_canonical_bytes() {
    let original = transaction();
    let encoded = encode(&original).expect("all operations have a generated transport");
    let json = serde_json::to_value(&encoded).unwrap();
    let tags: Vec<_> = json["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        tags,
        [
            "CreateNode",
            "UpdateProperty",
            "CreateEdge",
            "DeleteEdge",
            "AddAssertion",
            "RetractAssertion",
            "DefineNodeType",
            "DefineEdgeType",
            "ModifyProperty",
            "MergeEntity",
            "Invoke",
            "SupersedeAssertion",
            "AddEvidence",
            "WidenEdgeType",
            "AddAlias",
            "AttachEvidence"
        ]
    );
    assert_eq!(
        json["operations"][8]["value"]["owner"],
        json!(match &original.operations[8] {
            GraphOperation::ModifyProperty(v) => v.owner.unwrap().to_string(),
            _ => unreachable!(),
        })
    );
    let held = serde_json::from_slice(&serde_json::to_vec(&encoded).unwrap()).unwrap();
    let restored = decode(&held).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.canonical_bytes(), original.canonical_bytes());
}

#[test]
fn malformed_identity_and_duplicate_manifest_are_refused() {
    let encoded = serde_json::to_value(encode(&transaction()).unwrap()).unwrap();
    for path in [
        "/id",
        "/proposer",
        "/operations/0/value/id",
        "/operations/4/value/subject/id",
        "/operations/8/value/owner",
        "/schema_version",
    ] {
        let mut damaged = encoded.clone();
        *damaged.pointer_mut(path).unwrap() = json!("invalid-identity");
        let held = serde_json::from_value(damaged).unwrap();
        assert!(decode(&held).is_err(), "accepted {path}");
    }
    let mut damaged = encoded;
    let ids = damaged["evidence"].as_array_mut().unwrap();
    ids.push(ids[0].clone());
    assert!(decode(&serde_json::from_value(damaged).unwrap()).is_err());
}

#[test]
fn canonical_value_bytes_and_kind_are_both_checked() {
    let original = serde_json::to_value(encode(&transaction()).unwrap()).unwrap();
    for replacement in [
        json!({"kind":"Integer", "canonical_bytes":"AA=="}),
        json!({"kind":"Integer", "canonical_bytes":ekr_core::bytes::encode(&CanonicalValue::Boolean(true).canonical_bytes())}),
    ] {
        let mut damaged = original.clone();
        damaged["operations"][1]["value"]["values"][0] = replacement;
        assert!(decode(&serde_json::from_value(damaged).unwrap()).is_err());
    }
    let mut damaged = original;
    damaged["operations"][1]["value"]["values"][0]["kind"] = json!("Float");
    assert!(serde_json::from_value::<w::EkrKernelCanonicalTransactionProjection>(damaged).is_err());
}

#[test]
fn optional_owner_schema_and_empty_aliases_preserve_historical_meaning() {
    let mut original = transaction();
    original.schema_version = None;
    if let GraphOperation::ModifyProperty(value) = &mut original.operations[8] {
        value.owner = None;
    }
    if let GraphOperation::CreateNode(value) = &mut original.operations[0] {
        value.aliases.clear();
    }
    let encoded = encode(&original).unwrap();
    let json = serde_json::to_value(&encoded).unwrap();
    assert!(json.get("schema_version").is_none());
    assert!(json["operations"][8]["value"].get("owner").is_none());
    let restored = decode(&encoded).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.canonical_bytes(), original.canonical_bytes());
}

#[test]
fn inconsistent_projection_payloads_are_not_silently_dropped() {
    let original = serde_json::to_value(encode(&transaction()).unwrap()).unwrap();
    let mut wrong_object = original.clone();
    wrong_object["operations"][4]["value"]["object"]["reference"] =
        json!(NodeId::mint().to_string());
    assert!(decode(&serde_json::from_value(wrong_object).unwrap()).is_err());
    let mut wrong_type = original.clone();
    wrong_type["operations"][8]["value"]["property"]["value_type"]["variants"] =
        json!(["unrelated"]);
    assert!(decode(&serde_json::from_value(wrong_type).unwrap()).is_err());
    let mut wrong_source = original;
    wrong_source["operations"][12]["value"]["evidence"]["source"]["url"] =
        Value::String("https://example.invalid/unrelated".into());
    assert!(decode(&serde_json::from_value(wrong_source).unwrap()).is_err());
}

#[test]
fn every_assertion_and_evidence_payload_variant_round_trips() {
    let mut original = transaction();
    let GraphOperation::AddAssertion(template) = original.operations[4].clone() else {
        unreachable!()
    };
    let assessments = [
        Assessment::Proposed,
        Assessment::Validating {
            completed: 1,
            required: 3,
        },
        Assessment::Accepted {
            validators: [AgentId::mint(), AgentId::mint()].into(),
        },
        Assessment::Rejected {
            issues: vec![IssueId::mint()],
        },
        Assessment::Disputed {
            competing_assertions: vec![CanonicalRef::new(AssertionId::mint())],
        },
    ];
    for (index, assessment) in assessments.into_iter().enumerate() {
        let mut claim = template.clone();
        claim.assessment = assessment;
        claim.subject = if index % 2 == 0 {
            Subject::Edge(CanonicalRef::new(EdgeId::mint()))
        } else {
            Subject::Type(TypeId::mint())
        };
        claim.predicate = Predicate::Relation(TypeId::mint());
        claim.object = if index % 2 == 0 {
            Object::Node(CanonicalRef::new(NodeId::mint()))
        } else {
            Object::Type(TypeId::mint())
        };
        claim.lifecycle = if index % 2 == 0 {
            AssertionLifecycle::Active
        } else {
            AssertionLifecycle::Retracted {
                at_revision: RevisionNumber::new(12),
                reason: RetractionReason::new("withdrawn"),
            }
        };
        claim.valid_time = TemporalRange::UNBOUNDED;
        claim.transaction_time = TransactionTime::new(Timestamp::EPOCH, None).unwrap();
        original
            .operations
            .push(GraphOperation::AddAssertion(claim));
    }
    let GraphOperation::AddEvidence(template) = original.operations[12].clone() else {
        unreachable!()
    };
    for source in [
        EvidenceSource::Url("https://example.invalid/evidence".into()),
        EvidenceSource::Document {
            document_id: "document".into(),
            section: None,
        },
        EvidenceSource::DatabaseRecord {
            database: "database".into(),
            table: "table".into(),
            key: "key".into(),
        },
        EvidenceSource::GraphAssertion(CanonicalRef::new(AssertionId::mint())),
        EvidenceSource::Observation(ObservationId::mint()),
        EvidenceSource::HumanStatement {
            identity: Some("reviewer".into()),
        },
        EvidenceSource::HumanStatement { identity: None },
    ] {
        let mut addition = template.clone();
        addition.evidence.source = source;
        original
            .operations
            .push(GraphOperation::AddEvidence(addition));
    }
    let restored = decode(&encode(&original).unwrap()).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.canonical_bytes(), original.canonical_bytes());
}

#[test]
fn normalization_is_limited_to_empty_aliases_and_equivalent_instants() {
    let mut original = transaction();
    if let GraphOperation::CreateNode(value) = &mut original.operations[0] {
        value.aliases.clear();
    }
    let mut wire = serde_json::to_value(encode(&original).unwrap()).unwrap();
    wire["operations"][0]["value"]["aliases"] = json!([]);
    wire["operations"][11]["value"]["effective_from"] = json!("1970-01-01T01:00:00.020+01:00");
    let restored = decode(&serde_json::from_value(wire.clone()).unwrap()).unwrap();
    assert_eq!(restored.canonical_bytes(), original.canonical_bytes());
    wire["operations"][11]["value"]["effective_from"] = json!("1970-01-01T00:00:00.020001Z");
    assert!(decode(&serde_json::from_value(wire).unwrap())
        .unwrap_err()
        .to_string()
        .contains("submillisecond"));
}

#[test]
fn unrepresentable_timestamps_and_nested_values_fail_closed() {
    let mut original = transaction();
    if let GraphOperation::SupersedeAssertion(value) = &mut original.operations[11] {
        value.effective_from = Timestamp::from_millis(i64::MAX);
    }
    assert!(encode(&original)
        .unwrap_err()
        .to_string()
        .contains("calendar range"));
    let mut original = transaction();
    let mut nested = CanonicalValue::Boolean(true);
    for _ in 0..34 {
        nested = CanonicalValue::List(vec![nested]);
    }
    if let GraphOperation::UpdateProperty(value) = &mut original.operations[1] {
        value.values = vec![nested];
    }
    assert!(encode(&original)
        .unwrap_err()
        .to_string()
        .contains("nesting limit"));
}

#[test]
fn invalid_numeric_ranges_and_interval_order_are_refused() {
    let original = serde_json::to_value(encode(&transaction()).unwrap()).unwrap();
    for (path, replacement) in [
        ("/operations/12/value/evidence/confidence_bp", json!(10001)),
        ("/operations/12/value/evidence/confidence_bp", json!(-1)),
        ("/operations/4/value/lifecycle/at_revision", json!(-1)),
        (
            "/operations/4/value/valid_time/from",
            json!("2000-01-01T00:00:00Z"),
        ),
        (
            "/operations/4/value/transaction_time/recorded_to",
            json!("1969-01-01T00:00:00Z"),
        ),
        ("/operations/12/value/payload", json!("not-base64")),
    ] {
        let mut wire = original.clone();
        *wire.pointer_mut(path).unwrap() = replacement;
        assert!(
            decode(&serde_json::from_value(wire).unwrap()).is_err(),
            "accepted {path}"
        );
    }
}
