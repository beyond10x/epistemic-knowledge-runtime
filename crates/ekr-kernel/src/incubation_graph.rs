//! Materialize an inspection-only transient graph; its candidates are never persisted as canonical.
use ekr_core::{AgentId, AssertionId, EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Edge, GraphRoot, Node, Object, Predicate, Subject,
    TemporalRange, TransactionTime, TransientGraph,
};
use ekr_integrate::{ExtractedFact, ExtractedReference, ExtractionDocument};
use ekr_ontology::Value;
use std::collections::BTreeMap;

pub(super) fn materialize(
    root: GraphRoot,
    document: &ExtractionDocument,
    proposer: AgentId,
) -> TransientGraph {
    let mut graph = TransientGraph {
        root,
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
    };
    let mut types = BTreeMap::new();
    let mut properties = BTreeMap::new();
    let mut references = BTreeMap::new();
    let mut node = |reference: &ExtractedReference, graph: &mut TransientGraph| {
        let mut aliases = reference.aliases.clone();
        aliases.sort();
        aliases.dedup();
        *references
            .entry((reference.node_type.clone(), aliases))
            .or_insert_with(|| {
                let type_id = *types
                    .entry(reference.node_type.clone())
                    .or_insert_with(TypeId::mint);
                let id = NodeId::mint();
                let mut node = Node::new(
                    id,
                    root.id,
                    type_id,
                    reference.aliases.first().cloned().unwrap_or_default(),
                );
                node.aliases = reference.aliases.clone();
                graph.nodes.insert(id, node);
                id
            })
    };
    for reference in &document.entities {
        node(reference, &mut graph);
    }
    let mut relations = BTreeMap::new();
    for fact in &document.facts {
        let (subject, predicate, object) = match fact {
            ExtractedFact::Property(fact) => {
                let subject = node(&fact.subject, &mut graph);
                let property = *properties
                    .entry((fact.subject.node_type.clone(), fact.property.clone()))
                    .or_insert_with(PropertyId::mint);
                graph
                    .nodes
                    .get_mut(&subject)
                    .expect("materialized subject")
                    .properties
                    .entry(property)
                    .or_default()
                    .push(fact.value.clone());
                (
                    Subject::Node(subject),
                    Predicate::Property(property),
                    Object::Value(fact.value.clone()),
                )
            }
            ExtractedFact::Relation(fact) => {
                let source = node(&fact.subject, &mut graph);
                let target = node(&fact.object, &mut graph);
                let type_id = *relations
                    .entry(fact.relation.clone())
                    .or_insert_with(TypeId::mint);
                let id = EdgeId::mint();
                graph
                    .edges
                    .insert(id, Edge::<Value>::new(id, root.id, type_id, source, target));
                (
                    Subject::Node(source),
                    Predicate::Relation(type_id),
                    Object::Node(target),
                )
            }
        };
        let id = AssertionId::mint();
        graph.assertions.insert(
            id,
            Assertion {
                id,
                root_id: root.id,
                subject,
                predicate,
                object,
                evidence: fact.evidence().iter().copied().collect(),
                proposed_by: proposer,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::new(None, None).expect("unbounded range"),
                transaction_time: TransactionTime::since(root.created_at),
            },
        );
    }
    graph
}
