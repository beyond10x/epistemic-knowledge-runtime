//! The resolver against one canonical snapshot: the acceptance of `story:typed-reference-resolver`.
//!
//! Over one canonical snapshot, a typed reference resolves to the single node of exactly its type
//! that holds one of its aliases, to `ProposeNew` when no node does, and to `Ambiguous` listing
//! every candidate when more than one does, and two nodes sharing a name but differing in type
//! never resolve to each other.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Node, Space};
use ekr_integrate::{
    resolve, AmbiguousReference, ResolutionOutcome, ResolutionRefusal, ResolutionRefusalCode,
    ResolvedReference, TypedReference,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
use proptest::prelude::*;

fn id_text(tag: u16, n: u64) -> String {
    format!("00000000-{tag:04x}-7000-8000-{n:012x}")
}

fn node_id(n: u64) -> NodeId {
    id_text(1, n).parse().expect("a canonical uuid")
}

fn type_id(n: u64) -> TypeId {
    id_text(2, n).parse().expect("a canonical uuid")
}

fn root_id(n: u64) -> GraphRootId {
    id_text(3, n).parse().expect("a canonical uuid")
}

fn schema_id() -> SchemaVersionId {
    id_text(4, 0).parse().expect("a canonical uuid")
}

/// Two concrete leaf types, an abstract type, a parent with one declared child.
struct Types {
    person: TypeId,
    organisation: TypeId,
    abstract_agent: TypeId,
    parent: TypeId,
    child: TypeId,
}

fn types() -> Types {
    Types {
        person: type_id(1),
        organisation: type_id(2),
        abstract_agent: type_id(3),
        parent: type_id(4),
        child: type_id(5),
    }
}

fn ontology() -> Ontology {
    let t = types();
    let mut abstract_agent = NodeType::new(t.abstract_agent, "agent");
    abstract_agent.abstract_type = true;
    let mut child = NodeType::new(t.child, "child");
    child.parents = BTreeSet::from([t.parent]);
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema_id(), Timestamp::EPOCH),
        node_types: vec![
            NodeType::new(t.person, "person"),
            NodeType::new(t.organisation, "organisation"),
            abstract_agent,
            NodeType::new(t.parent, "parent"),
            child,
        ],
        edge_types: Vec::new(),
    })
    .expect("the fixture ontology loads")
}

fn node(n: u64, type_id: TypeId, name: &str, aliases: &[&str]) -> Node {
    let mut node = Node::new(node_id(n), root_id(1), type_id, name);
    node.aliases = aliases.iter().map(|alias| (*alias).to_owned()).collect();
    node
}

fn graph(nodes: impl IntoIterator<Item = Node>) -> CanonicalGraph {
    CanonicalGraph {
        root: GraphRoot {
            id: root_id(1),
            space: Space::Canonical,
            schema_version_id: schema_id(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(1),
        ontology: ontology(),
        nodes: nodes.into_iter().map(|node| (node.id, node)).collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

fn reference(type_id: TypeId, aliases: &[&str]) -> TypedReference {
    TypedReference {
        type_id,
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
    }
}

#[test]
fn one_node_of_exactly_the_type_holding_an_alias_resolves() {
    let t = types();
    let state = graph([
        node(1, t.person, "Ada", &["ada", "a.l."]),
        node(2, t.person, "Grace", &["grace"]),
    ]);
    let outcome = resolve(
        GraphSnapshot::of(&state),
        &reference(t.person, &["unknown", "a.l."]),
    );
    assert_eq!(
        outcome,
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(1)
        })
    );
}

#[test]
fn two_nodes_sharing_a_name_but_differing_in_type_never_resolve_to_each_other() {
    let t = types();
    let state = graph([
        node(1, t.person, "Meridian", &["meridian"]),
        node(2, t.organisation, "Meridian", &["meridian"]),
    ]);
    let snapshot = GraphSnapshot::of(&state);
    assert_eq!(
        resolve(snapshot, &reference(t.person, &["meridian"])),
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(1)
        })
    );
    assert_eq!(
        resolve(snapshot, &reference(t.organisation, &["meridian"])),
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(2)
        })
    );
}

#[test]
fn no_node_holding_an_alias_proposes_a_new_node_carrying_the_type_and_aliases() {
    let t = types();
    let state = graph([node(1, t.person, "Ada", &["ada"])]);
    let outcome = resolve(
        GraphSnapshot::of(&state),
        &reference(t.person, &["babbage"]),
    );
    assert_eq!(
        outcome,
        ResolutionOutcome::ProposeNew(reference(t.person, &["babbage"]))
    );
    assert_eq!(state.nodes.len(), 1, "a proposal writes nothing");
}

#[test]
fn a_node_of_another_type_holding_the_alias_is_not_a_candidate() {
    let t = types();
    let state = graph([node(1, t.organisation, "Ada", &["ada"])]);
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.person, &["ada"])),
        ResolutionOutcome::ProposeNew(reference(t.person, &["ada"]))
    );
}

#[test]
fn more_than_one_candidate_is_ambiguous_listing_every_one_in_id_order() {
    let t = types();
    let state = graph([
        node(3, t.person, "Third", &["shared"]),
        node(1, t.person, "First", &["x", "shared"]),
        node(2, t.person, "Second", &["other", "also"]),
        node(4, t.organisation, "Fourth", &["shared"]),
    ]);
    let outcome = resolve(
        GraphSnapshot::of(&state),
        &reference(t.person, &["shared", "also"]),
    );
    assert_eq!(
        outcome,
        ResolutionOutcome::Ambiguous(AmbiguousReference {
            candidates: vec![node_id(1), node_id(2), node_id(3)]
        })
    );
}

#[test]
fn an_alias_differing_by_case_or_whitespace_does_not_match() {
    let t = types();
    let state = graph([node(1, t.person, "Ada", &["ada lovelace"])]);
    for spelling in [
        "Ada Lovelace",
        "ADA LOVELACE",
        " ada lovelace",
        "ada lovelace ",
        "ada  lovelace",
        "ada\tlovelace",
        "adalovelace",
    ] {
        assert_eq!(
            resolve(GraphSnapshot::of(&state), &reference(t.person, &[spelling])),
            ResolutionOutcome::ProposeNew(reference(t.person, &[spelling])),
            "{spelling:?} is not the alias byte for byte"
        );
    }
}

#[test]
fn the_canonical_name_is_never_compared() {
    let t = types();
    let state = graph([node(1, t.person, "Ada", &["countess"])]);
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.person, &["Ada"])),
        ResolutionOutcome::ProposeNew(reference(t.person, &["Ada"]))
    );
}

#[test]
fn a_node_outside_the_canonical_root_is_not_a_candidate() {
    let t = types();
    let mut foreign = node(1, t.person, "Ada", &["ada"]);
    foreign.root_id = root_id(2);
    let state = graph([foreign]);
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.person, &["ada"])),
        ResolutionOutcome::ProposeNew(reference(t.person, &["ada"]))
    );
}

#[test]
fn a_reference_with_no_alias_is_refused_as_without_identity() {
    let t = types();
    let state = graph([node(1, t.person, "Ada", &["ada"])]);
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.person, &[])),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceWithoutIdentity,
            reference: reference(t.person, &[]),
        })
    );
}

#[test]
fn a_reference_to_an_abstract_type_or_a_type_with_descendants_is_refused() {
    let t = types();
    let state = graph([
        node(1, t.parent, "Base", &["base"]),
        node(2, t.child, "Derived", &["base"]),
    ]);
    for refused in [t.abstract_agent, t.parent] {
        assert_eq!(
            resolve(GraphSnapshot::of(&state), &reference(refused, &["base"])),
            ResolutionOutcome::Refused(ResolutionRefusal {
                code: ResolutionRefusalCode::ReferenceTypeHasSubtypes,
                reference: reference(refused, &["base"]),
            })
        );
    }
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.child, &["base"])),
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: node_id(2)
        }),
        "a leaf type resolves, and only to its own nodes"
    );
}

#[test]
fn a_reference_to_a_type_the_ontology_does_not_declare_is_refused_as_undeclared() {
    let undeclared = type_id(99);
    let stray = node(1, undeclared, "Stray", &["stray"]);
    let state = graph([stray]);
    assert!(state.ontology.node_type(undeclared).is_none());
    assert_eq!(
        resolve(
            GraphSnapshot::of(&state),
            &reference(undeclared, &["stray"])
        ),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceTypeUndeclared,
            reference: reference(undeclared, &["stray"]),
        }),
        "even a node carrying the undeclared type is not a candidate"
    );
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(undeclared, &[])),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceWithoutIdentity,
            reference: reference(undeclared, &[]),
        }),
        "a reference with no identity is refused for that first"
    );
}

#[test]
fn an_empty_alias_identifies_nothing_and_is_not_carried_into_a_proposal() {
    let t = types();
    let state = graph([node(1, t.person, "Blank", &[""])]);
    assert_eq!(
        resolve(
            GraphSnapshot::of(&state),
            &reference(t.person, &["", "zed", ""])
        ),
        ResolutionOutcome::ProposeNew(reference(t.person, &["zed"]))
    );
    assert_eq!(
        resolve(GraphSnapshot::of(&state), &reference(t.person, &["", ""])),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceWithoutIdentity,
            reference: reference(t.person, &[""]),
        }),
        "a refusal echoes the reference, sorted and deduplicated"
    );
}

const POOL: [&str; 4] = ["p", "q", "r", "s"];

/// Six nodes, each of one of two types, each holding a subset of a four-alias pool.
fn universe() -> impl Strategy<Value = Vec<(bool, Vec<&'static str>)>> {
    proptest::collection::vec(
        (
            any::<bool>(),
            proptest::sample::subsequence(POOL.to_vec(), 0..=POOL.len()),
        ),
        6,
    )
}

fn build(
    shape: &[(bool, Vec<&'static str>)],
    order: &[usize],
    alias_orders: &[Vec<usize>],
) -> CanonicalGraph {
    let t = types();
    graph(order.iter().map(|&index| {
        let (is_person, aliases) = &shape[index];
        let kind = if *is_person { t.person } else { t.organisation };
        let permuted: Vec<&str> = alias_orders[index]
            .iter()
            .filter_map(|&at| aliases.get(at).copied())
            .collect();
        node(index as u64, kind, "same name", &permuted)
    }))
}

proptest! {
    #[test]
    fn permuting_node_order_or_alias_order_gives_byte_identical_outcomes(
        shape in universe(),
        order in Just((0..6usize).collect::<Vec<_>>()).prop_shuffle(),
        alias_orders in proptest::collection::vec(
            Just((0..POOL.len()).collect::<Vec<_>>()).prop_shuffle(), 6),
        person in any::<bool>(),
        wanted in proptest::sample::subsequence(POOL.to_vec(), 0..=POOL.len()),
        wanted_order in Just((0..POOL.len()).collect::<Vec<_>>()).prop_shuffle(),
    ) {
        let t = types();
        let kind = if person { t.person } else { t.organisation };
        let identity: Vec<Vec<usize>> = vec![(0..POOL.len()).collect(); 6];

        let base_state = build(&shape, &(0..6).collect::<Vec<_>>(), &identity);
        let base = resolve(GraphSnapshot::of(&base_state), &reference(kind, &wanted));

        let permuted_state = build(&shape, &order, &alias_orders);
        let permuted_wanted: Vec<&str> = wanted_order
            .iter()
            .filter_map(|&at| wanted.get(at).copied())
            .collect();
        let permuted = resolve(
            GraphSnapshot::of(&permuted_state),
            &reference(kind, &permuted_wanted),
        );

        prop_assert_eq!(
            serde_json::to_vec(&base).expect("an outcome serialises"),
            serde_json::to_vec(&permuted).expect("an outcome serialises")
        );

        let matched = |outcome: &ResolutionOutcome| -> Vec<NodeId> {
            match outcome {
                ResolutionOutcome::Resolved(resolved) => vec![resolved.node_id],
                ResolutionOutcome::Ambiguous(ambiguous) => ambiguous.candidates.clone(),
                ResolutionOutcome::ProposeNew(_) | ResolutionOutcome::Refused(_) => Vec::new(),
            }
        };
        let expected: Vec<NodeId> = base_state
            .nodes
            .values()
            .filter(|node| node.type_id == kind)
            .filter(|node| node.aliases.iter().any(|alias| wanted.contains(&alias.as_str())))
            .map(|node| node.id)
            .collect();
        prop_assert_eq!(matched(&base), expected);
    }
}
