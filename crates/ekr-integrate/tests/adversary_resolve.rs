//! Adversary cases against `story:typed-reference-resolver`.

use std::collections::BTreeMap;

use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Node, Space};
use ekr_integrate::{
    resolve, AmbiguousReference, ResolutionOutcome, ResolutionRefusal, ResolutionRefusalCode,
    ResolvedReference, TypedReference,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};

fn id_text(tag: u16, n: u64) -> String {
    format!("00000000-{tag:04x}-7000-8000-{n:012x}")
}

fn node_id(n: u64) -> NodeId {
    id_text(1, n).parse().expect("a canonical uuid")
}

fn type_id(n: u64) -> TypeId {
    id_text(2, n).parse().expect("a canonical uuid")
}

fn root_id() -> GraphRootId {
    id_text(3, 1).parse().expect("a canonical uuid")
}

fn schema_id() -> SchemaVersionId {
    id_text(4, 0).parse().expect("a canonical uuid")
}

fn person() -> TypeId {
    type_id(1)
}

fn graph(nodes: impl IntoIterator<Item = Node>) -> CanonicalGraph {
    CanonicalGraph {
        root: GraphRoot {
            id: root_id(),
            space: Space::Canonical,
            schema_version_id: schema_id(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(1),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema_id(), Timestamp::EPOCH),
            node_types: vec![NodeType::new(person(), "person")],
            edge_types: Vec::new(),
        })
        .expect("the fixture ontology loads"),
        nodes: nodes.into_iter().map(|node| (node.id, node)).collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

fn node(n: u64, aliases: &[&str]) -> Node {
    let mut node = Node::new(node_id(n), root_id(), person(), "same name");
    node.aliases = aliases.iter().map(|alias| (*alias).to_owned()).collect();
    node
}

fn reference(type_id: TypeId, aliases: &[&str]) -> TypedReference {
    TypedReference {
        type_id,
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
    }
}

/// `reference-without-identity`: "the reference holds no alias, so nothing identifies the node it
/// means". An empty string identifies nothing either; accepting it as an identity makes every node
/// that carries an empty alias a candidate for every reference that carries one.
#[test]
fn a_reference_whose_only_alias_is_empty_is_refused_as_without_identity() {
    let state = graph([node(1, &[""]), node(2, &["ada"])]);
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(person(), &[""])),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceWithoutIdentity,
            reference: reference(person(), &[""]),
        })
    );
}

/// An empty alias beside a real one must not turn a unique match into an ambiguity.
#[test]
fn an_empty_alias_does_not_make_an_unrelated_node_a_candidate() {
    let state = graph([node(1, &[""]), node(2, &["", "ada"]), node(3, &[""])]);
    let outcome = resolve(
        GraphSnapshot::of(&state),
        &reference(person(), &["ada", ""]),
    );
    assert_ne!(
        outcome,
        ResolutionOutcome::Ambiguous(AmbiguousReference {
            candidates: vec![node_id(1), node_id(2), node_id(3)],
        }),
        "the empty alias matched every node that carries one"
    );
    assert_eq!(
        outcome,
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(2)
        })
    );
}

/// A type the ontology does not declare: `Ontology::conforms_to` holds that an undeclared type
/// conforms to nothing, so a node of it is not a node the kernel admits. Proposing one is
/// proposing a node that cannot be committed.
#[test]
fn a_reference_to_an_undeclared_type_is_not_proposed_as_a_new_node() {
    let state = graph([node(1, &["ada"])]);
    let undeclared = type_id(99);
    assert!(state.ontology.node_type(undeclared).is_none());
    let outcome = resolve(GraphSnapshot::of(&state), &reference(undeclared, &["ada"]));
    assert!(
        !matches!(outcome, ResolutionOutcome::ProposeNew(_)),
        "an undeclared type was proposed as a new node: {outcome:?}"
    );
}

/// The crate documentation promises the aliases of a proposal and a refusal are deduplicated;
/// no existing case holds a repeated alias, so dropping `dedup()` stays green without this.
#[test]
fn a_repeated_alias_is_carried_once_in_a_proposal_and_a_refusal() {
    let state = graph([node(1, &["ada"])]);
    assert_eq!(
        resolve(
            GraphSnapshot::of(&state),
            &reference(person(), &["b", "a", "b", "a"])
        ),
        ResolutionOutcome::ProposeNew(reference(person(), &["a", "b"]))
    );
}

/// Every outcome survives its own wire form.
#[test]
fn every_outcome_round_trips_through_its_wire_form() {
    let outcomes = [
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(1),
        }),
        ResolutionOutcome::ProposeNew(reference(person(), &["a"])),
        ResolutionOutcome::Ambiguous(AmbiguousReference {
            candidates: vec![node_id(1), node_id(2)],
        }),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceTypeHasSubtypes,
            reference: reference(person(), &["a"]),
        }),
    ];
    for outcome in outcomes {
        let wire = serde_json::to_string(&outcome).expect("an outcome serialises");
        let back: ResolutionOutcome =
            serde_json::from_str(&wire).unwrap_or_else(|error| panic!("{wire}: {error}"));
        assert_eq!(back, outcome, "{wire}");
    }
}
