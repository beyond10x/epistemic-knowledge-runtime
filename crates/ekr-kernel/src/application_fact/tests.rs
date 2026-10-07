use super::*;
use crate::{
    Agent, AuthorityStateV1, BootstrapContext, Runtime, SeedDocument, ValidationProfileV1,
};
use ekr_core::{
    generated_identity::{Identity, SchemaApplicationId},
    AssertionId, ContentHash, NodeId, ObservationId, PropertyId, TypeId,
};
use ekr_graph::{
    AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    TemporalRange, TransactionTime,
};
use ekr_integrate::{ExtractedFact, ExtractedReference, ExtractionDocument, PropertyFact};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde_json::json;
use std::{collections::BTreeSet, sync::Arc};

struct Fixture {
    read: VerifiedRead,
    proposal: w::EkrIntegrateRetainedSchemaProposal,
    mapping: w::EkrIntegrateKnowledgeMapping,
    sources: BTreeMap<String, Source>,
    evidence_map: BTreeMap<EvidenceId, EvidenceId>,
    application_id: w::EkrIntegrateSchemaApplicationId,
    proposer: AgentId,
    subject: NodeId,
}
impl Fixture {
    fn new() -> Self {
        let context = BootstrapContext {
            operator: AgentId::mint(),
            validator: AgentId::mint(),
        };
        let anchor = AuthorityStateV1 {
            format: "ekr.authority-state/1".into(),
            agents: [context.operator, context.validator]
                .into_iter()
                .map(|id| {
                    (
                        id,
                        Agent {
                            id,
                            name: "fact fixture".into(),
                            capabilities: BTreeSet::new(),
                        },
                    )
                })
                .collect(),
            validation_profile: ValidationProfileV1::deterministic(context.validator),
        };
        let mut document =
            SeedDocument::from_yaml(include_str!("../../tests/fixtures/seed-minimal-v2.yaml"))
                .unwrap();
        let mut node_type = NodeType::new(TypeId::mint(), "Project");
        let property = PropertyId::mint();
        node_type.properties.insert(
            property,
            PropertyDefinition::new(property, "health", ValueType::String),
        );
        let mut node = Node::new(
            NodeId::mint(),
            document.graph.root.id,
            node_type.id,
            "Fixture project",
        );
        node.aliases.push("project-alias".into());
        document.graph.nodes.insert(node.id, node.clone());
        document.ontology.node_types.push(node_type);
        let dir = tempfile::tempdir().unwrap();
        let runtime =
            Runtime::file(&dir.path().join("store"), "fact-fixture", context, anchor).unwrap();
        runtime.seed(document, || Timestamp::EPOCH).unwrap();
        let mut read = runtime.read(None).unwrap();
        let originals = [EvidenceId::mint(), EvidenceId::mint()];
        let wrappers = [EvidenceId::mint(), EvidenceId::mint()];
        for (index, id) in wrappers.iter().enumerate() {
            Arc::make_mut(&mut read.graph).evidence.insert(
                *id,
                Evidence {
                    id: *id,
                    source: if index == 0 {
                        EvidenceSource::Observation(ObservationId::mint())
                    } else {
                        EvidenceSource::HumanStatement {
                            identity: Some("retained reviewer".into()),
                        }
                    },
                    content_hash: ContentHash::of_bytes(&[index as u8]),
                    extracted_by: context.operator,
                    observed_at: Timestamp::EPOCH,
                    confidence: Confidence::CERTAIN,
                },
            );
        }
        let interpretation = NodeId::mint();
        let source = json!({"interpretation_id":interpretation,"version":1,"document_digest":ContentHash::of_bytes(b"source")});
        let mapping:w::EkrIntegrateKnowledgeMapping=serde_json::from_value(json!({"source":source,"source_item":"facts[0]","source_type":"Project","target_type":"Project","target_member":"health","value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}})).unwrap();
        let retained_source=Source{retained_document:serde_json::from_value(json!({"version":{"interpretation_id":interpretation,"version":1},"root_id":NodeId::mint(),"observations":[],"local_schema":{"node_types":[],"edge_types":[]},"entities":[],"facts":[],"evidence":[]})).unwrap(),document:ExtractionDocument{format:ekr_integrate::extraction::ExtractionFormat::V1,ontology:Default::default(),entities:vec![],facts:vec![ExtractedFact::Property(PropertyFact{subject:ExtractedReference{node_type:"Project".into(),aliases:vec!["project-alias".into()]},property:"health".into(),value:Value::String("amber".into()),evidence:vec![originals[0],originals[1],originals[0]]})],evidence:vec![]},selected:["facts[0]".into()].into()};
        let proposal:w::EkrIntegrateSchemaProposalDocument=serde_json::from_value(json!({"proposal_id":NodeId::mint(),"base_schema":read.graph.ontology.version().id,"sources":[{"version":source,"items":["facts[0]"]}],"observations":[],"evidence":[],"additions":[],"mappings":[mapping],"corrections":[],"explanation":"approved mapping fixture"})).unwrap();
        let bytes = serde_json::to_vec_pretty(&proposal).unwrap();
        let proposal = w::EkrIntegrateRetainedSchemaProposal {
            proposal: Box::new(proposal),
            proposal_digest: Box::new(w::EkrKernelContentHash(
                ContentHash::of_bytes(&bytes).to_string(),
            )),
            payload: ekr_core::bytes::encode(&bytes),
        };
        Self {
            read,
            proposal,
            mapping: mapping.clone(),
            sources: [(
                crate::schema_proposals::coordinate(&mapping.source).unwrap(),
                retained_source,
            )]
            .into(),
            evidence_map: originals.into_iter().zip(wrappers).collect(),
            application_id: SchemaApplicationId::mint(),
            proposer: context.operator,
            subject: node.id,
        }
    }
    fn inputs(&self) -> BuildInputs<'_> {
        BuildInputs {
            read: &self.read,
            proposal: &self.proposal,
            mapping: &self.mapping,
            sources: &self.sources,
            evidence_map: &self.evidence_map,
            application_id: &self.application_id,
            proposer: self.proposer,
            elected_at: Timestamp::from_millis(1234),
        }
    }
    fn step(&self) -> w::EkrIntegrateRetainedApplicationStep {
        let FactStepOutcome::Ready(step) = build(&self.inputs()).unwrap() else {
            panic!("fixture blocked")
        };
        *step
    }
}

#[test]
fn fact_step_contains_only_supported_assertion_and_exact_mapping_provenance() {
    let fixture = Fixture::new();
    let step = fixture.step();
    let tx = crate::application_transaction::decode(&step.transaction).unwrap();
    assert_eq!(step.application_id.as_ref(), &fixture.application_id);
    assert_eq!(
        step.step.item,
        w::EssPresence::Present(Box::new(
            crate::application_mapping::item(&fixture.mapping).unwrap()
        ))
    );
    assert!(step.replacements.is_empty());
    assert_eq!(tx.proposer, fixture.proposer);
    assert_eq!(tx.schema_version, None);
    let [crate::GraphOperation::AddAssertion(assertion)] = tx.operations.as_slice() else {
        panic!("not one assertion")
    };
    assert_eq!(assertion.root_id, fixture.read.graph.root.id);
    assert_eq!(assertion.assessment, Assessment::Proposed);
    assert_eq!(assertion.lifecycle, AssertionLifecycle::Active);
    assert_eq!(assertion.valid_time, TemporalRange::UNBOUNDED);
    assert_eq!(
        assertion.transaction_time,
        TransactionTime::since(Timestamp::from_millis(1234))
    );
    assert_eq!(
        assertion.object,
        Object::Value(ekr_graph::CanonicalValue::String("amber".into()))
    );
    let supports: BTreeSet<_> = fixture.evidence_map.values().copied().collect();
    assert_eq!(tx.evidence, supports);
    assert_eq!(
        assertion
            .evidence
            .iter()
            .map(|v| v.id())
            .collect::<BTreeSet<_>>(),
        supports
    );
    assert_eq!(step.mappings.len(), 1);
    let mapping = &step.mappings[0];
    assert_eq!(
        mapping.payload,
        ekr_core::bytes::encode(&crate::application_mapping::payload(&fixture.mapping).unwrap())
    );
    assert_eq!(mapping.proposal_digest, fixture.proposal.proposal_digest);
    assert_eq!(
        mapping.source_document_digest,
        fixture.mapping.source.document_digest
    );
    assert_eq!(mapping.mapping.as_ref(), &fixture.mapping);
    assert_eq!(step.derivations.len(), 2);
    let mut ids = BTreeSet::new();
    for derivation in &step.derivations {
        let _: NodeId = derivation.derivation_id.parse().unwrap();
        assert!(ids.insert(&derivation.derivation_id));
        assert_eq!(derivation.assertion_id.0, assertion.id.to_string());
        assert_eq!(derivation.mapping_id, mapping.mapping_id);
        let id: EvidenceId = derivation.evidence_id.0.parse().unwrap();
        match fixture.read.graph.evidence[&id].source {
            EvidenceSource::Observation(observation) => assert_eq!(
                derivation.observation_id,
                w::EssPresence::Present(Box::new(w::EkrGraphObservationId(
                    observation.to_string()
                )))
            ),
            _ => assert_eq!(derivation.observation_id, w::EssPresence::Absent),
        }
    }
    verify_semantic(&fixture.inputs(), &step).unwrap();
    verify_structure(&fixture.inputs(), &step).unwrap();
    assert_ne!(fixture.step().step_election_id, step.step_election_id);
}
#[test]
fn blocked_mapping_returns_blockers_without_a_step() {
    let mut fixture = Fixture::new();
    Arc::make_mut(&mut fixture.read.graph)
        .nodes
        .remove(&fixture.subject);
    let outcome = build(&fixture.inputs()).unwrap();
    assert!(matches!(outcome,FactStepOutcome::Blocked(ref reasons) if !reasons.is_empty()));
}
#[test]
fn semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision() {
    let mut fixture = Fixture::new();
    let step = fixture.step();
    Arc::make_mut(&mut fixture.read.graph).revision = ekr_core::RevisionNumber::new(77);
    verify_semantic(&fixture.inputs(), &step).unwrap();
    Arc::make_mut(&mut fixture.read.graph)
        .nodes
        .get_mut(&fixture.subject)
        .unwrap()
        .aliases
        .clear();
    verify_structure(&fixture.inputs(), &step).unwrap();
    assert!(verify_semantic(&fixture.inputs(), &step).is_err());
}
#[test]
fn frozen_template_tampering_is_rejected() {
    let fixture = Fixture::new();
    let original = serde_json::to_value(fixture.step()).unwrap();
    for (path, value) in [
        ("/application_id", json!(NodeId::mint())),
        ("/mappings/0/payload", json!("AA==")),
        (
            "/mappings/0/source_document_digest",
            json!(ContentHash::of_bytes(b"wrong")),
        ),
        ("/transaction/proposer", json!(AgentId::mint())),
        (
            "/transaction/operations/0/value/assessment/kind",
            json!("Accepted"),
        ),
        ("/derivations/0/assertion_id", json!(AssertionId::mint())),
        ("/derivations/0/derivation_id", json!("not-a-uuid")),
        (
            "/derivations/0/observation_id",
            json!(ObservationId::mint()),
        ),
        ("/step/item/item", json!("facts[1]")),
        ("/elected_at", json!("2020-01-01T00:00:00Z")),
    ] {
        let mut changed = original.clone();
        if path == "/derivations/0/observation_id" {
            // HumanStatement support has no observation field; ordering UUIDs must not
            // make this test depend on which source kind happens to sort first.
            changed["derivations"][0]["observation_id"] = value;
        } else {
            *changed
                .pointer_mut(path)
                .unwrap_or_else(|| panic!("missing {path}")) = value;
        }
        let step = serde_json::from_value(changed).unwrap();
        assert!(
            verify_structure(&fixture.inputs(), &step).is_err(),
            "accepted {path}"
        );
        assert!(
            verify_semantic(&fixture.inputs(), &step).is_err(),
            "semantic accepted {path}"
        );
    }
}
#[test]
fn no_support_or_incomplete_correspondence_never_creates_fresh_evidence() {
    let mut fixture = Fixture::new();
    fixture.evidence_map.clear();
    assert!(build(&fixture.inputs()).is_err());
    let mut fixture = Fixture::new();
    let ExtractedFact::Property(fact) =
        &mut fixture.sources.values_mut().next().unwrap().document.facts[0]
    else {
        unreachable!()
    };
    fact.evidence.clear();
    assert!(build(&fixture.inputs()).is_err());
}

#[test]
fn blocked_resolution_does_not_even_invoke_the_identity_allocator() {
    let mut fixture = Fixture::new();
    Arc::make_mut(&mut fixture.read.graph)
        .nodes
        .remove(&fixture.subject);
    let result = build_using(&fixture.inputs(), |_| {
        panic!("blocked mapping allocated identities")
    })
    .unwrap();
    assert!(matches!(result, FactStepOutcome::Blocked(_)));
}

#[test]
fn frozen_template_rejects_duplicate_identities_and_incomplete_support() {
    let fixture = Fixture::new();
    let original = fixture.step();
    for case in 0..7 {
        let mut changed = original.clone();
        match case {
            0 => {
                changed.derivations[1].derivation_id = changed.derivations[0].derivation_id.clone()
            }
            1 => {
                changed.derivations.pop();
            }
            2 => changed.derivations[0].derivation_id = changed.step_election_id.0.clone(),
            3 => {
                changed.mappings[0].evidence.pop();
            }
            4 => {
                let mut tx = crate::application_transaction::decode(&changed.transaction).unwrap();
                tx.operations.push(tx.operations[0].clone());
                changed.transaction =
                    Box::new(crate::application_transaction::encode(&tx).unwrap());
            }
            5 => {
                let mut tx = crate::application_transaction::decode(&changed.transaction).unwrap();
                tx.evidence.clear();
                changed.transaction =
                    Box::new(crate::application_transaction::encode(&tx).unwrap());
            }
            6 => {
                let mut tx = crate::application_transaction::decode(&changed.transaction).unwrap();
                let GraphOperation::AddAssertion(assertion) = &mut tx.operations[0] else {
                    unreachable!()
                };
                assertion.evidence.clear();
                changed.transaction =
                    Box::new(crate::application_transaction::encode(&tx).unwrap());
            }
            _ => unreachable!(),
        }
        assert!(
            verify_structure(&fixture.inputs(), &changed).is_err(),
            "accepted case {case}"
        );
        assert!(
            verify_semantic(&fixture.inputs(), &changed).is_err(),
            "semantic accepted case {case}"
        );
    }
}

#[test]
fn semantic_verification_checks_the_claim_value_beyond_structural_shape() {
    let fixture = Fixture::new();
    let mut step = fixture.step();
    let mut tx = crate::application_transaction::decode(&step.transaction).unwrap();
    let GraphOperation::AddAssertion(assertion) = &mut tx.operations[0] else {
        unreachable!()
    };
    assertion.object = Object::Value(ekr_graph::CanonicalValue::String("unreviewed value".into()));
    step.transaction = Box::new(crate::application_transaction::encode(&tx).unwrap());
    verify_structure(&fixture.inputs(), &step).unwrap();
    assert!(verify_semantic(&fixture.inputs(), &step).is_err());
}

#[test]
fn unselected_mapping_and_missing_correspondence_fail_before_allocating() {
    let mut fixture = Fixture::new();
    fixture.mapping.target_member = "other".into();
    assert!(build_using(&fixture.inputs(), |_| panic!(
        "unselected mapping allocated"
    ))
    .is_err());
    let mut fixture = Fixture::new();
    fixture.evidence_map.clear();
    assert!(build_using(&fixture.inputs(), |_| panic!("missing evidence allocated")).is_err());
}
