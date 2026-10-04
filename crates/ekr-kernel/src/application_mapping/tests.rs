use super::*;
use crate::{
    Agent, AuthorityStateV1, BootstrapContext, Runtime, SeedDocument, ValidationProfileV1,
};
use ekr_core::{canonical::Canonical, AgentId, NodeId, PropertyId, Timestamp, TypeId};
use ekr_graph::{CanonicalRef, Node};
use ekr_integrate::{
    ExtractedFact, ExtractedReference, ExtractionDocument, PropertyFact, RelationFact,
};
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;

struct Fixture {
    read: VerifiedRead,
    mapping: w::EkrIntegrateKnowledgeMapping,
    sources: BTreeMap<String, Source>,
    subject: NodeId,
    object: NodeId,
    property: PropertyId,
    relation: TypeId,
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
                            name: "mapping fixture".into(),
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
        let mut edge_type = EdgeType::new(TypeId::mint(), "DependsOn");
        edge_type.source_types.insert(node_type.id);
        edge_type.target_types.insert(node_type.id);
        let relation = edge_type.id;
        let mut subject = Node::new(
            NodeId::mint(),
            document.graph.root.id,
            node_type.id,
            "canonical project",
        );
        subject.aliases.push("source project".into());
        let mut object = Node::new(
            NodeId::mint(),
            document.graph.root.id,
            node_type.id,
            "dependency",
        );
        object.aliases.push("dependency".into());
        document.graph.nodes.insert(subject.id, subject.clone());
        document.graph.nodes.insert(object.id, object.clone());
        document.ontology.node_types.push(node_type);
        document.ontology.edge_types.push(edge_type);
        let dir = tempfile::tempdir().unwrap();
        let runtime = Runtime::file(
            &dir.path().join("store"),
            "mapping-fixture",
            context,
            anchor,
        )
        .unwrap();
        runtime.seed(document, || Timestamp::EPOCH).unwrap();
        let read = runtime.read(None).unwrap();
        let interpretation = NodeId::mint();
        let version = json!({"interpretation_id":interpretation,"version":2,"document_digest":ContentHash::of_bytes(b"retained source")});
        let mapping:w::EkrIntegrateKnowledgeMapping=serde_json::from_value(json!({"source":version,"source_item":"facts[0]","source_type":"Project","target_type":"Project","target_member":"health","value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}})).unwrap();
        let source=Source {retained_document:serde_json::from_value(json!({"version":{"interpretation_id":interpretation,"version":2},"root_id":NodeId::mint(),"observations":[],"local_schema":{"node_types":[],"edge_types":[]},"entities":[],"facts":[],"evidence":[]})).unwrap(),document:ExtractionDocument {format:ekr_integrate::extraction::ExtractionFormat::V1,ontology:Default::default(),entities:vec![],facts:vec![ExtractedFact::Property(PropertyFact {subject:ExtractedReference{node_type:"Project".into(),aliases:vec!["source project".into()]},property:"health".into(),value:Value::String("amber".into()),evidence:vec![EvidenceId::mint(),EvidenceId::mint()]})],evidence:vec![]},selected:["facts[0]".into()].into()};
        let mut sources = BTreeMap::new();
        sources.insert(
            crate::schema_proposals::coordinate(&mapping.source).unwrap(),
            source,
        );
        Self {
            read,
            mapping,
            sources,
            subject: subject.id,
            object: object.id,
            property,
            relation,
        }
    }
    fn source(&mut self) -> &mut Source {
        self.sources.values_mut().next().unwrap()
    }
    fn preview(&self) -> Vec<String> {
        let proposal: w::EkrIntegrateSchemaProposalDocument=serde_json::from_value(json!({"proposal_id":NodeId::mint(),"base_schema":self.read.graph.ontology.version().id,"observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[self.mapping],"corrections":[],"explanation":"fixture"})).unwrap();
        crate::schema_proposal_mapping::preview(
            &self.read,
            &proposal,
            &self.read.graph.ontology,
            &self.sources,
            false,
        )
        .unwrap()
        .0
        .remove(0)
        .blockers
    }
    fn outcome(&self) -> MappingOutcome {
        resolve(
            &self.read,
            &self.mapping,
            &self.read.graph.ontology,
            &self.sources,
        )
        .unwrap()
    }
    fn type_of_property(&mut self, ty: ValueType) {
        let mut doc = self.read.graph.ontology.to_document();
        doc.node_types[0]
            .properties
            .get_mut(&self.property)
            .unwrap()
            .value_type = ty;
        Arc::make_mut(&mut self.read.graph).ontology = Ontology::load(doc).unwrap();
    }
}
fn constant(value: CanonicalValue) -> Box<w::EkrIntegrateMappingValue> {
    let plain: Value = value.clone().into();
    Box::new(serde_json::from_value(json!({"kind":"Constant","value":{"kind":plain.kind(),"canonical_bytes":ekr_core::bytes::encode(&value.canonical_bytes())}})).unwrap())
}
#[test]
fn resolved_property_uses_alias_and_exact_selected_fact_evidence() {
    let fixture = Fixture::new();
    assert!(fixture.preview().is_empty());
    assert_eq!(
        fixture.outcome(),
        MappingOutcome::Ready(MappedClaim {
            subject: Subject::Node(CanonicalRef::new(fixture.subject)),
            predicate: Predicate::Property(fixture.property),
            object: Object::Value(CanonicalValue::String("amber".into())),
            evidence: fixture.sources.values().next().unwrap().document.facts[0]
                .evidence()
                .to_vec()
        })
    );
}
#[test]
fn unresolved_ambiguous_and_invalid_selectors_match_preview_blockers() {
    for case in 0..6 {
        let mut fixture = Fixture::new();
        match case {
            0 => {
                Arc::make_mut(&mut fixture.read.graph)
                    .nodes
                    .remove(&fixture.subject);
            }
            1 => {
                let graph = Arc::make_mut(&mut fixture.read.graph);
                let mut duplicate = graph.nodes[&fixture.subject].clone();
                duplicate.id = NodeId::mint();
                graph.nodes.insert(duplicate.id, duplicate);
            }
            2 => {
                *fixture.mapping.value = serde_json::from_value(
                    json!({"kind":"CopyField","value":{"declaration":"Project","field":"wrong"}}),
                )
                .unwrap()
            }
            3 => fixture.mapping.target_member = "unknown".into(),
            4 => {
                fixture.source().selected.clear();
            }
            _ => fixture.type_of_property(ValueType::Boolean),
        }
        let blockers = fixture.preview();
        assert!(!blockers.is_empty(), "case {case}");
        assert_eq!(
            fixture.outcome(),
            MappingOutcome::Blocked(blockers),
            "case {case}"
        );
    }
}
#[test]
fn all_canonical_value_kinds_work_for_constant_and_copy_field() {
    let mut fixture = Fixture::new();
    let allowed = [fixture.read.graph.nodes[&fixture.object].type_id].into();
    let pairs = vec![
        (CanonicalValue::String("text".into()), ValueType::String),
        (CanonicalValue::Boolean(true), ValueType::Boolean),
        (CanonicalValue::Integer(-17), ValueType::Integer),
        (CanonicalValue::Decimal("1.25".into()), ValueType::Decimal),
        (
            CanonicalValue::Timestamp(Timestamp::from_millis(-1)),
            ValueType::Timestamp,
        ),
        (CanonicalValue::Duration(20), ValueType::Duration),
        (
            CanonicalValue::NodeRef(CanonicalRef::new(fixture.object)),
            ValueType::NodeRef {
                allowed_types: allowed,
            },
        ),
        (
            CanonicalValue::Enum("amber".into()),
            ValueType::Enum {
                variants: ["amber".into()].into(),
            },
        ),
        (
            CanonicalValue::List(vec![CanonicalValue::Integer(1)]),
            ValueType::List(Box::new(ValueType::Integer)),
        ),
        (
            CanonicalValue::Record([("ok".into(), CanonicalValue::Boolean(true))].into()),
            ValueType::Record([("ok".into(), ValueType::Boolean)].into()),
        ),
    ];
    for (value, ty) in pairs {
        fixture.type_of_property(ty);
        for copied in [false, true] {
            if copied {
                let ExtractedFact::Property(fact) = &mut fixture.source().document.facts[0] else {
                    unreachable!()
                };
                fact.value = value.clone().into();
                *fixture.mapping.value = serde_json::from_value(
                    json!({"kind":"CopyField","value":{"declaration":"Project","field":"health"}}),
                )
                .unwrap();
            } else {
                fixture.mapping.value = constant(value.clone());
            }
            assert!(fixture.preview().is_empty());
            let MappingOutcome::Ready(claim) = fixture.outcome() else {
                panic!("blocked {value:?}")
            };
            assert_eq!(claim.object, Object::Value(value.clone()));
        }
    }
}
#[test]
fn relation_copy_and_noderef_constant_check_both_endpoints() {
    let mut fixture = Fixture::new();
    let evidence = fixture.source().document.facts[0].evidence().to_vec();
    fixture.source().document.facts[0] = ExtractedFact::Relation(RelationFact {
        subject: ExtractedReference {
            node_type: "Project".into(),
            aliases: vec!["source project".into()],
        },
        relation: "DependsOn".into(),
        object: ExtractedReference {
            node_type: "Project".into(),
            aliases: vec!["dependency".into()],
        },
        evidence,
    });
    fixture.mapping.target_member = "DependsOn".into();
    for copied in [true, false] {
        fixture.mapping.value = if copied {
            Box::new(serde_json::from_value(json!({"kind":"CopyRelation","value":{"declaration":"Project","relation":"DependsOn"}})).unwrap())
        } else {
            constant(CanonicalValue::NodeRef(CanonicalRef::new(fixture.object)))
        };
        assert!(fixture.preview().is_empty());
        let MappingOutcome::Ready(claim) = fixture.outcome() else {
            panic!("relation blocked")
        };
        assert_eq!(claim.predicate, Predicate::Relation(fixture.relation));
        assert_eq!(
            claim.object,
            Object::Node(CanonicalRef::new(fixture.object))
        );
    }
    fixture.mapping.value = constant(CanonicalValue::String("not a node".into()));
    assert_eq!(
        fixture.outcome(),
        MappingOutcome::Blocked(fixture.preview())
    );
    fixture.mapping.value = constant(CanonicalValue::NodeRef(CanonicalRef::new(NodeId::mint())));
    assert_eq!(
        fixture.outcome(),
        MappingOutcome::Blocked(fixture.preview())
    );
}
#[test]
fn mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source() {
    let fixture = Fixture::new();
    let mapping = &fixture.mapping;
    let bytes = payload(mapping).unwrap();
    assert_eq!(bytes, serde_json::to_vec(mapping).unwrap());
    assert_ne!(bytes.last(), Some(&b'\n'));
    assert_eq!(digest(mapping).unwrap(), ContentHash::of_bytes(&bytes));
    let first = item(mapping).unwrap();
    assert_eq!(first.source, mapping.source);
    assert_eq!(first.item, "facts[0]");
    assert_eq!(first.mapping_digest.0, digest(mapping).unwrap().to_string());
    for changed in 0..4 {
        let mut other = mapping.clone();
        match changed {
            0 => other.target_member = "different".into(),
            1 => other.source.version = 3.into(),
            2 => other.source.document_digest.0 = ContentHash::of_bytes(b"other").to_string(),
            _ => other.source.interpretation_id.0 = NodeId::mint().to_string(),
        };
        assert_ne!(item(&other).unwrap(), first);
    }
    let mut invalid = mapping.clone();
    invalid.source_item = "facts[00]".into();
    assert!(item(&invalid).is_err());
}

#[test]
fn relation_missing_ambiguous_and_outside_endpoints_stay_blocked() {
    for case in 0..5 {
        let mut fixture = Fixture::new();
        let evidence = fixture.source().document.facts[0].evidence().to_vec();
        fixture.source().document.facts[0] = ExtractedFact::Relation(RelationFact {
            subject: ExtractedReference {
                node_type: "Project".into(),
                aliases: vec!["source project".into()],
            },
            relation: "DependsOn".into(),
            object: ExtractedReference {
                node_type: "Project".into(),
                aliases: vec!["dependency".into()],
            },
            evidence,
        });
        fixture.mapping.target_member = "DependsOn".into();
        *fixture.mapping.value = serde_json::from_value(
            json!({"kind":"CopyRelation","value":{"declaration":"Project","relation":"DependsOn"}}),
        )
        .unwrap();
        let graph = Arc::make_mut(&mut fixture.read.graph);
        match case {
            0 => {
                graph.nodes.remove(&fixture.object);
            }
            1 => {
                let mut duplicate = graph.nodes[&fixture.object].clone();
                duplicate.id = NodeId::mint();
                graph.nodes.insert(duplicate.id, duplicate);
            }
            2 | 3 => {
                let mut ontology = graph.ontology.to_document();
                let other = NodeType::new(TypeId::mint(), "Other");
                if case == 2 {
                    ontology.edge_types[0].source_types = [other.id].into();
                } else {
                    ontology.edge_types[0].target_types = [other.id].into();
                }
                ontology.node_types.push(other);
                graph.ontology = Ontology::load(ontology).unwrap();
            }
            _ => graph
                .nodes
                .get_mut(&fixture.object)
                .unwrap()
                .aliases
                .clear(),
        }
        let before = fixture.read.graph.nodes.len();
        let blockers = fixture.preview();
        assert!(!blockers.is_empty(), "case {case}");
        assert_eq!(
            fixture.outcome(),
            MappingOutcome::Blocked(blockers),
            "case {case}"
        );
        assert_eq!(
            fixture.read.graph.nodes.len(),
            before,
            "mapping must allocate no entity"
        );
    }
}
