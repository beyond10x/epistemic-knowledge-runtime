//! Adversary pass, wave perf-02 unit E (`story:edge-type-endpoints-widen`): the compatibility rule
//! lets an edge type whose ends only gained types replace the prior one over its edges. Nothing
//! else may ride along with an end gaining a type and be passed as compatible.

use std::collections::BTreeSet;
/// One change to an edge type's declaration beside a widening, by name.
type Change = fn(&mut EdgeType, TypeId);

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    incompatibilities, Cardinality, EdgeType, Incompatibility, InstanceState, NodeType, Ontology,
    OntologyDocument, SchemaVersion, ValueKind,
};

struct Edges(TypeId, u64);

impl InstanceState for Edges {
    fn node_count(&self, _: TypeId) -> u64 {
        1
    }
    fn edge_count(&self, edge_type: TypeId) -> u64 {
        if edge_type == self.0 {
            self.1
        } else {
            0
        }
    }
    fn max_values(&self, _: TypeId, _: PropertyId) -> u64 {
        0
    }
    fn min_values(&self, _: TypeId, _: PropertyId) -> u64 {
        0
    }
    fn value_kinds(&self, _: TypeId, _: PropertyId) -> BTreeSet<ValueKind> {
        BTreeSet::new()
    }
}

#[test]
fn an_end_gained_beside_any_other_declaration_change_is_still_a_declaration_change() {
    let (note, topic, cites, other) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
    );
    let mut declared = EdgeType::new(cites, "cites");
    declared.source_types.insert(note);
    declared.target_types.insert(note);
    let mut inverse = EdgeType::new(other, "cited_by");
    inverse.source_types.insert(note);
    inverse.target_types.insert(note);
    let prior = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![NodeType::new(note, "Note"), NodeType::new(topic, "Topic")],
        edge_types: vec![declared.clone(), inverse],
    })
    .unwrap();
    let changes: [(&str, Change); 5] = [
        ("transitive", |at, _| at.transitive = !at.transitive),
        ("symmetric", |at, _| at.symmetric = !at.symmetric),
        ("inverse", |at, other| at.inverse = Some(other)),
        ("cardinality", |at, _| at.cardinality = Cardinality::Many),
        ("name", |at, _| at.name = "references".into()),
    ];
    for (what, change) in changes {
        let mut document = prior.to_document();
        document.version = SchemaVersion {
            id: SchemaVersionId::mint(),
            number: 1,
            parent: Some(prior.version().id),
            created_at: Timestamp::from_millis(1),
        };
        for at in &mut document.edge_types {
            if at.id == cites {
                at.target_types.insert(topic);
                change(at, other);
            }
        }
        let next = Ontology::load(document).unwrap();
        assert_eq!(
            incompatibilities(&prior, &next, &Edges(cites, 2)),
            vec![Incompatibility::DeclarationChanged {
                type_id: cites,
                instances: 2
            }],
            "{what} changed with a gained end"
        );
    }
}

/// Either end replaced, not widened, by a route other than `WidenEdgeType` is a declaration
/// change under edges. The unit's own case replaces only `target_types`, so the `source_types`
/// half of the subset rule can be dropped with its suite green.
#[test]
fn a_source_end_replaced_rather_than_widened_is_a_declaration_change_under_edges() {
    let (note, topic, cites) = (TypeId::mint(), TypeId::mint(), TypeId::mint());
    let mut declared = EdgeType::new(cites, "cites");
    declared.source_types.insert(note);
    declared.target_types.insert(note);
    let prior = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![NodeType::new(note, "Note"), NodeType::new(topic, "Topic")],
        edge_types: vec![declared],
    })
    .unwrap();
    for end in ["source_types", "target_types"] {
        let mut document = prior.to_document();
        document.version = SchemaVersion {
            id: SchemaVersionId::mint(),
            number: 1,
            parent: Some(prior.version().id),
            created_at: Timestamp::from_millis(1),
        };
        let replaced = &mut document.edge_types[0];
        if end == "source_types" {
            replaced.source_types = BTreeSet::from([topic]);
        } else {
            replaced.target_types = BTreeSet::from([topic]);
        }
        let next = Ontology::load(document).unwrap();
        assert_eq!(
            incompatibilities(&prior, &next, &Edges(cites, 3)),
            vec![Incompatibility::DeclarationChanged {
                type_id: cites,
                instances: 3
            }],
            "{end} replaced under 3 edges"
        );
    }
}
