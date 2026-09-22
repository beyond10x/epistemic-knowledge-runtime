//! `Ontology::to_document` writes back every declaration a loaded ontology holds, in stable
//! identity order, with the version data and the declarations nothing references.

use std::collections::BTreeSet;

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OperationDefinition,
    PropertyDefinition, SchemaVersion, Transition, ValueType,
};

fn id<T: std::str::FromStr>(n: u8) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

/// Declarations written in descending identity order, so a writer that returned declaration order
/// and one that returned identity order give different documents.
fn declared() -> OntologyDocument {
    let (actor, unused, decision) = (id::<TypeId>(1), id::<TypeId>(2), id::<TypeId>(3));
    let (unused_edge, decided_by) = (id::<TypeId>(4), id::<TypeId>(5));
    let title = id::<PropertyId>(6);

    let mut base = NodeType::new(actor, "Actor");
    base.abstract_type = true;

    let mut subject = NodeType::new(decision, "Decision");
    subject.parents.insert(actor);
    let mut property = PropertyDefinition::new(title, "title", ValueType::String);
    property.required = true;
    subject.properties.insert(title, property);
    subject.lifecycle = Some(Lifecycle {
        initial: "open".into(),
        states: BTreeSet::from(["open".to_owned(), "closed".to_owned()]),
        transitions: BTreeSet::from([Transition::new("open", "closed")]),
    });
    let mut close = OperationDefinition::new("close");
    close.transition = Some(Transition::new("open", "closed"));
    subject.operations.insert("close".into(), close);

    let mut relation = EdgeType::new(decided_by, "decided_by");
    relation.source_types.insert(decision);
    relation.target_types.insert(actor);
    let mut loop_edge = EdgeType::new(unused_edge, "unreferenced");
    loop_edge.source_types.insert(unused);
    loop_edge.target_types.insert(unused);
    loop_edge.symmetric = true;

    OntologyDocument {
        version: SchemaVersion {
            id: id::<SchemaVersionId>(7),
            number: 3,
            parent: Some(id::<SchemaVersionId>(8)),
            created_at: Timestamp::from_millis(1_790_035_200_000),
        },
        node_types: vec![subject, NodeType::new(unused, "Unreferenced"), base],
        edge_types: vec![relation, loop_edge],
    }
}

#[test]
fn a_loaded_ontology_writes_back_every_declaration_in_identity_order() {
    let document = declared();
    let ontology = Ontology::load(document.clone()).unwrap();
    let written = ontology.to_document();

    assert_eq!(written.version, document.version);
    assert_eq!(
        written.node_types.iter().map(|t| t.id).collect::<Vec<_>>(),
        [id::<TypeId>(1), id(2), id(3)]
    );
    assert_eq!(
        written.edge_types.iter().map(|t| t.id).collect::<Vec<_>>(),
        [id::<TypeId>(4), id(5)]
    );

    let mut node_types = document.node_types.clone();
    node_types.sort_by_key(|t| t.id);
    let mut edge_types = document.edge_types.clone();
    edge_types.sort_by_key(|t| t.id);
    assert_eq!(written.node_types, node_types);
    assert_eq!(written.edge_types, edge_types);
    assert_ne!(
        written, document,
        "declaration order came back instead of identity order"
    );
}

#[test]
fn the_written_document_reloads_to_the_same_ontology_and_is_a_fixed_point() {
    let ontology = Ontology::load(declared()).unwrap();
    let written = ontology.to_document();

    let reloaded = Ontology::load(written.clone()).unwrap();
    assert_eq!(reloaded, ontology);
    assert_eq!(reloaded.to_document(), written);

    let yaml = serde_yaml_ng::to_string(&written).unwrap();
    let from_text = Ontology::from_yaml(&yaml).unwrap();
    assert_eq!(from_text, ontology);
    assert_eq!(from_text.to_document(), written);
}

#[test]
fn an_empty_seed_writes_back_only_its_version() {
    let version = SchemaVersion::seed(id(9), Timestamp::EPOCH);
    let ontology = Ontology::load(OntologyDocument {
        version: version.clone(),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap();
    assert_eq!(
        ontology.to_document(),
        OntologyDocument {
            version,
            node_types: Vec::new(),
            edge_types: Vec::new(),
        }
    );
}
