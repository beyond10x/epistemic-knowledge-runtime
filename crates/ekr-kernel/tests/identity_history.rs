//! An identity names one record for the life of a store: `task:deleted-edge-id-is-reusable`.
//!
//! `DeleteEdge` removes an edge from canonical state and keeps no record of its id, so a rule that
//! reads only the current state lets a later `CreateEdge` take the id again for a different
//! relationship — one `EdgeId`, two meanings, at two revisions. Node and edge ids are both UUIDs in
//! one space, so the same rule covers a node minted over an edge's id and the other way round,
//! whether the other record still exists or was deleted.
//!
//! The rule belongs to the pipeline that is handed the lineage's identities
//! ([`Pipeline::identity_keeping`]). Profiles v1 and v2 keep answering as they did, because replay
//! re-runs a store's own profile over every retained decision, and a store that already holds such
//! a reuse must still reopen; the cases below say so for both.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, EdgeId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{CanonicalGraph, CanonicalRef, Edge, GraphRoot, GraphSnapshot, Node, Space};
use ekr_kernel::validate::HeldIdentities;
use ekr_kernel::{
    EdgeDraft, GraphOperation, GraphTransaction, NodeDraft, Pipeline, ValidationIssue,
    ValidatorName,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, SchemaVersion};

/// Two revisions of one store: at `before`, edge `deleted` joins `a` to `b` and edge `live` joins
/// `b` to `a`; at `now`, `deleted` is gone and `live` remains.
struct World {
    before: CanonicalGraph,
    now: CanonicalGraph,
    root_id: GraphRootId,
    decision: TypeId,
    depends_on: TypeId,
    a: NodeId,
    b: NodeId,
    c: NodeId,
    deleted: EdgeId,
    live: EdgeId,
    proposer: AgentId,
    reviewer: AgentId,
}

impl World {
    fn new() -> Self {
        let root_id = GraphRootId::mint();
        let (decision, depends_on) = (TypeId::mint(), TypeId::mint());
        let (a, b, c) = (NodeId::mint(), NodeId::mint(), NodeId::mint());
        let (deleted, live) = (EdgeId::mint(), EdgeId::mint());

        let mut edge_type = EdgeType::new(depends_on, "depends_on");
        edge_type.source_types = [decision].into_iter().collect();
        edge_type.target_types = [decision].into_iter().collect();
        edge_type.cardinality = Cardinality::Many;
        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![NodeType::new(decision, "Decision")],
            edge_types: vec![edge_type],
        })
        .expect("the fixture ontology coheres");

        let node = |id: NodeId, name: &str| (id, Node::new(id, root_id, decision, name));
        let edge = |id: EdgeId, source: NodeId, target: NodeId| {
            (
                id,
                Edge::new(
                    id,
                    root_id,
                    depends_on,
                    CanonicalRef::new(source),
                    CanonicalRef::new(target),
                ),
            )
        };
        let before = CanonicalGraph {
            attachments: Default::default(),
            root: GraphRoot {
                id: root_id,
                space: Space::Canonical,
                schema_version_id: ontology.version().id,
                parent: None,
                created_at: Timestamp::EPOCH,
            },
            revision: RevisionNumber::new(1),
            ontology,
            nodes: [node(a, "a"), node(b, "b"), node(c, "c")]
                .into_iter()
                .collect(),
            edges: [edge(deleted, a, b), edge(live, b, a)]
                .into_iter()
                .collect(),
            assertions: BTreeMap::new(),
            evidence: BTreeMap::new(),
        };
        let mut now = before.clone();
        now.revision = RevisionNumber::new(2);
        now.edges.remove(&deleted);

        Self {
            before,
            now,
            root_id,
            decision,
            depends_on,
            a,
            b,
            c,
            deleted,
            live,
            proposer: AgentId::mint(),
            reviewer: AgentId::mint(),
        }
    }

    fn proposal(&self, operations: Vec<GraphOperation>) -> GraphTransaction {
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: self.proposer,
            operations,
            evidence: BTreeSet::new(),
            schema_version: None,
        }
    }

    fn create_edge(&self, id: EdgeId, source: NodeId, target: NodeId) -> GraphOperation {
        GraphOperation::CreateEdge(EdgeDraft {
            id,
            root_id: self.root_id,
            type_id: self.depends_on,
            source,
            target,
            properties: BTreeMap::new(),
        })
    }

    fn create_node(&self, id: NodeId) -> GraphOperation {
        GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: self.root_id,
            type_id: self.decision,
            canonical_name: "a decision".to_owned(),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        })
    }

    /// The pipeline handed both revisions' identities.
    fn keeping(&self) -> Pipeline {
        Pipeline::identity_keeping(
            self.reviewer,
            BTreeSet::from([self.now.ontology.version().id]),
            HeldIdentities::of([&self.before, &self.now]),
        )
    }

    /// Profiles v1 and v2, which read the current state only.
    fn current_state_profiles(&self) -> [(&'static str, Pipeline); 2] {
        [
            ("v1", Pipeline::deterministic(self.reviewer)),
            (
                "v2",
                Pipeline::schema_evolving(
                    self.reviewer,
                    BTreeSet::from([self.now.ontology.version().id]),
                ),
            ),
        ]
    }

    fn validate(
        &self,
        pipeline: &Pipeline,
        operations: Vec<GraphOperation>,
    ) -> Result<(), Vec<ValidationIssue>> {
        pipeline
            .validate(&GraphSnapshot::of(&self.now), &self.proposal(operations))
            .map(|_| ())
    }
}

fn node_id(edge: EdgeId) -> NodeId {
    NodeId::from_uuid(edge.to_uuid())
}

fn edge_id(node: NodeId) -> EdgeId {
    EdgeId::from_uuid(node.to_uuid())
}

/// The refusal a proposal gets from `pipeline`: which validators, and which codes.
fn refused(result: Result<(), Vec<ValidationIssue>>) -> (Vec<ValidatorName>, Vec<String>) {
    let issues = result.expect_err("expected a refusal");
    let mut validators: Vec<ValidatorName> = issues.iter().map(|issue| issue.validator).collect();
    validators.dedup();
    (
        validators,
        issues.into_iter().map(|issue| issue.code).collect(),
    )
}

/// The review's reproduction: an edge created, deleted, then created again under the same id for
/// a different relationship (other endpoints) and for the same one.
#[test]
fn a_deleted_edges_id_is_not_created_again() {
    let world = World::new();
    for (endpoints, (source, target)) in [
        ("different endpoints", (world.c, world.a)),
        ("the same endpoints", (world.a, world.b)),
    ] {
        let reuse = vec![world.create_edge(world.deleted, source, target)];
        assert_eq!(
            refused(world.validate(&world.keeping(), reuse.clone())),
            (
                vec![ValidatorName::Structural],
                vec!["identity-previously-held".to_owned()]
            ),
            "{endpoints}"
        );
        for (profile, pipeline) in world.current_state_profiles() {
            assert_eq!(
                world.validate(&pipeline, reuse.clone()),
                Ok(()),
                "{profile} still admits it, so a store that holds one replays ({endpoints})"
            );
        }
    }
}

/// A node is not minted over an id an edge held, whether that edge was deleted or still exists,
/// and an edge is not minted over a node's id.
#[test]
fn a_node_or_edge_id_is_not_created_again_as_the_other_kind() {
    let world = World::new();
    let cases = [
        (
            "a node over a deleted edge's id",
            vec![world.create_node(node_id(world.deleted))],
            "identity-previously-held",
        ),
        (
            "a node over a live edge's id",
            vec![world.create_node(node_id(world.live))],
            "identity-already-exists",
        ),
        (
            "an edge over a live node's id",
            vec![world.create_edge(edge_id(world.c), world.a, world.b)],
            "identity-already-exists",
        ),
    ];
    for (case, operations, code) in cases {
        assert_eq!(
            refused(world.validate(&world.keeping(), operations.clone())),
            (vec![ValidatorName::Structural], vec![code.to_owned()]),
            "{case}"
        );
        for (profile, pipeline) in world.current_state_profiles() {
            assert_eq!(
                world.validate(&pipeline, operations.clone()),
                Ok(()),
                "{profile} still admits {case}"
            );
        }
    }
}

/// One transaction minting one id as a node and as an edge creates it twice.
#[test]
fn one_id_created_as_a_node_and_an_edge_in_one_transaction_is_refused() {
    let world = World::new();
    let shared = NodeId::mint();
    assert_eq!(
        refused(world.validate(
            &world.keeping(),
            vec![
                world.create_node(shared),
                world.create_edge(edge_id(shared), world.a, world.b),
            ],
        )),
        (
            vec![ValidatorName::Structural],
            vec!["duplicate-identity".to_owned()]
        )
    );
}

/// The rule is about ids a revision held, not about creating: fresh ids, and the node the same
/// transaction creates an edge from, are accepted.
#[test]
fn fresh_ids_are_accepted() {
    let world = World::new();
    let fresh = NodeId::mint();
    assert_eq!(
        world.validate(
            &world.keeping(),
            vec![
                world.create_node(fresh),
                world.create_edge(EdgeId::mint(), fresh, world.c),
                world.create_edge(EdgeId::mint(), world.a, world.b),
            ],
        ),
        Ok(())
    );
}
