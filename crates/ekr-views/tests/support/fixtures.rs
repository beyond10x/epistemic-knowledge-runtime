//! Synthetic fixture stores, built through the real kernel handlers on either native provider.
//!
//! Every id is a fixed UUID and every instant a fixed millisecond, so one fixture built twice —
//! on one provider or on both — holds the same canonical state and renders the same bytes. The
//! stores are written through `Runtime::seed`, `propose`, `validate` and `commit`, which only this
//! test support calls: `ekr-views` itself reaches none of them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::str::FromStr;

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, PropertyId, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Edge, Evidence, EvidenceSource, Node,
    Object, Predicate, RetractionReason, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, GraphOperation,
    GraphTransaction, NodeDraft, PropertyModification, PropertyMutation, Retraction, Runtime,
    SeedDocument, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;

/// The native provider a store is opened on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// `Runtime::file`.
    File,
    /// `Runtime::sqlite`.
    Sqlite,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Sqlite => "sqlite",
        }
    }
}

/// The fixed id `n`, of any id kind.
pub fn id<T: FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}")
        .parse()
        .expect("a canonical UUID text")
}

pub fn context() -> BootstrapContext {
    BootstrapContext {
        operator: id(3),
        validator: id(4),
    }
}

/// The schema-evolving (v2) profile, so every fixture may change its ontology.
pub fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::schema_evolving(c.validator),
    }
}

/// Opens (creating if absent) the provider rooted at `directory`.
pub fn open(directory: &Path, provider: Provider) -> Runtime {
    std::fs::create_dir_all(directory).expect("provider root");
    match provider {
        Provider::File => Runtime::file(directory, "views", context(), anchor()),
        Provider::Sqlite => {
            Runtime::sqlite(&directory.join("state.db"), "views", context(), anchor())
        }
    }
    .expect("the provider opens")
}

/// Opens an already provisioned provider rooted at `directory`, creating nothing.
pub fn reopen(directory: &Path, provider: Provider) -> Runtime {
    match provider {
        Provider::File => Runtime::file_existing(directory, "views", context(), anchor()),
        Provider::Sqlite => {
            Runtime::sqlite_existing(&directory.join("state.db"), "views", context(), anchor())
        }
    }
    .expect("the provisioned provider opens")
}

// ---- fixed ids --------------------------------------------------------------------------------

const SUBJECT: u64 = 0x100;
const LABEL: u64 = 0x101;
const LINKS: u64 = 0x102;
const WEIGHT: u64 = 0x103;
const OBSERVATION: u64 = 0x104;
const OBSERVES: u64 = 0x105;
const NOTE: u64 = 0x106;
const VERSION_1: u64 = 0x107;
const SUMMARY: u64 = 0x108;
const STRENGTH: u64 = 0x109;
const VERSION_2: u64 = 0x10a;
const SOURCE: u64 = 0x10b;
const ALPHA: u64 = 0x110;
const BETA: u64 = 0x111;
const GAMMA: u64 = 0x112;
const EVIDENCE: u64 = 0x120;
const SEEDED_CLAIM: u64 = 0x130;
const EDGE_CLAIM: u64 = 0x131;
const ADDED_CLAIM: u64 = 0x132;
const OBSERVED_EDGE_CLAIM: u64 = 0x133;
const SEEDED_EDGE: u64 = 0x140;
const OBSERVED_EDGE: u64 = 0x141;
const TRANSACTIONS: u64 = 0x200;
/// The node the generated `DescribeNode` scenarios ask for in `store`: the ESS synthesizer's
/// witness id, which `Fixture::Evolved` holds from its seed onward.
pub const DESCRIBED: u64 = 0x1bba_d186_42ad;

// `hub`: an-overview-lists-at-most-its-limit-of-top-nodes.yaml.
pub const HUB: u64 = 0x10_0000;
pub const HUB_LEAVES: u64 = 600;
const HUB_NODE_TYPE: u64 = 0x11_0000;
const HUB_EDGE_TYPE: u64 = 0x11_0001;
const HUB_EDGES: u64 = 0x20_0000;

// `timeline`: the-overview-derives-roles-and-timeline-from-valid-time.yaml.
pub const KIND_A: u64 = 0x30_0001;
pub const KIND_B: u64 = 0x30_0002;
pub const KIND_C: u64 = 0x30_0003;
const KIND_A_TO_B: u64 = 0x30_0004;
const KIND_A_VALUE: u64 = 0x30_0005;
const KIND_B_VALUE: u64 = 0x30_0006;
const KIND_C_VALUE: u64 = 0x30_0007;
const TIMELINE_A: u64 = 0x30_0100;
const TIMELINE_B: u64 = 0x30_0200;
const TIMELINE_C: u64 = 0x30_0300;
const TIMELINE_EDGES: u64 = 0x30_0400;
const TIMELINE_EVIDENCE: u64 = 0x30_0500;
const TIMELINE_ASSERTIONS: u64 = 0x31_0000;
/// 2027-01-01T09:00:00Z.
pub const JANUARY_FIRST_0900_MS: i64 = 1_798_794_000_000;
const DAY_MS: i64 = 86_400_000;
const MINUTE_MS: i64 = 60_000;

// `subjects`: a-timeline-row-is-a-subject-with-its-events-within-hops.yaml.
pub const HOLDER: u64 = 0xa0_0001;
pub const PLACE: u64 = 0xa0_0002;
pub const HAPPENING: u64 = 0xa0_0003;
pub const NOTICE: u64 = 0xa0_0004;
pub const TOUCHES: u64 = 0xa0_0005;
const SUBJECTS_PROPERTIES: u64 = 0xa0_0010;
/// Holders H1 to H3 are `HOLDERS + 1` to `+ 3`.
pub const HOLDERS: u64 = 0xa0_0100;
/// Places L1 and L2 are `PLACES + 1` and `+ 2`.
pub const PLACES: u64 = 0xa0_0200;
/// Happenings E0 to E6 are `HAPPENINGS` to `+ 6`.
pub const HAPPENINGS: u64 = 0xa0_0300;
/// Notices N1 to N3 are `NOTICES + 1` to `+ 3`.
const NOTICES: u64 = 0xa0_0400;
const SUBJECTS_EDGES: u64 = 0xa0_0500;
const SUBJECTS_EVIDENCE: u64 = 0xa0_0600;
const SUBJECTS_ASSERTIONS: u64 = 0xa1_0000;

// `growth`: a-node-is-not-found-at-a-revision-before-it-existed.yaml.
pub const GROWTH_FIRST: u64 = 0x40_0001;
pub const GROWTH_SECOND: u64 = 0x40_0002;
const GROWTH_VALUE_ASSERTION: u64 = 0x40_0011;
const GROWTH_RELATION_ASSERTION: u64 = 0x40_0012;
pub const GROWTH_EDGE: u64 = 0x40_0021;
const GROWTH_EVIDENCE: u64 = 0x40_0031;
const GROWTH_NODE_TYPE: u64 = 0x40_0100;
const GROWTH_PROPERTY: u64 = 0x40_0101;
const GROWTH_EDGE_TYPE: u64 = 0x40_0102;

// `build_shared_id`: a node and an edge holding one UUID.
pub const SHARED: u64 = 0x90_0001;
pub const SHARED_NODE_CLAIM: u64 = 0x90_0011;
pub const SHARED_EDGE_CLAIM: u64 = 0x90_0012;
pub const SEEDED_EDGE_ID: u64 = SEEDED_EDGE;
pub const SEEDED_EDGE_CLAIM: u64 = EDGE_CLAIM;

/// The first instant a fixture's host clock reads; each sample adds a millisecond.
const CLOCK_START_MS: i64 = 1_800_000_000_000;

// ---- the named stores ---------------------------------------------------------------------------

/// A fixture store the conformance scenarios name by `store`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixture {
    /// The seed alone: one node type, one property, one node.
    SeedOnly,
    /// A seed holding three evidence records and one assertion citing them.
    SeededEvidence,
    /// Two nodes, one edge, and exactly one assertion, about that edge.
    EdgeAssertion,
    /// One assertion added at revision 1 and retracted at revision 2.
    RetractedAssertion,
    /// One schema change at revision 1.
    SchemaEvolution,
    /// The seeded, schema-evolved store: two schema changes, an edge, an edge assertion and a
    /// retraction over six revisions. Its seed also holds [`DESCRIBED`], the node the generated
    /// `DescribeNode` scenarios name.
    Evolved,
    /// A seed only: one hub joined by one edge to each of 600 leaves.
    Hub,
    /// Three node types whose facts gather in valid time or do not, over three revisions.
    Timeline,
    /// One node at the seed and a second, its edge and two assertions at revision 1.
    Growth,
    /// A seed only: holders, places, happenings and notices whose edges exercise every rule of
    /// the timeline's walk.
    Subjects,
}

impl Fixture {
    /// The fixture a scenario's `store` input names.
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "seed-only" => Self::SeedOnly,
            "seeded-evidence" => Self::SeededEvidence,
            "edge-assertion" => Self::EdgeAssertion,
            "retracted-assertion" => Self::RetractedAssertion,
            "schema-evolution" => Self::SchemaEvolution,
            "store" => Self::Evolved,
            "hub" => Self::Hub,
            "timeline" => Self::Timeline,
            "growth" => Self::Growth,
            "subjects" => Self::Subjects,
            _ => return None,
        })
    }

    /// Seeds and commits this fixture into the empty provider `runtime`.
    pub fn build(self, runtime: &Runtime) {
        let mut writer = Writer {
            runtime,
            clock: CLOCK_START_MS,
            transactions: TRANSACTIONS,
        };
        match self {
            Self::SeedOnly => writer.seed(seed(0, false, false, 1)),
            Self::SeededEvidence => writer.seed(seed(3, true, false, 1)),
            Self::EdgeAssertion => writer.seed(seed(1, false, true, 2)),
            Self::RetractedAssertion => {
                writer.seed(seed(1, false, false, 1));
                writer.commit(
                    vec![GraphOperation::AddAssertion(Box::new(claim(
                        ADDED_CLAIM,
                        Subject::Node(id(ALPHA)),
                        LABEL,
                        "alpha",
                        &[0],
                    )))],
                    None,
                );
                writer.commit(retraction(ADDED_CLAIM), None);
            }
            Self::SchemaEvolution => {
                writer.seed(seed(0, false, false, 1));
                writer.commit(first_schema_change(false), Some(id(VERSION_1)));
            }
            Self::Hub => writer.seed(hub()),
            Self::Timeline => {
                writer.seed(timeline_seed());
                writer.commit(
                    [TIMELINE_B + 1, TIMELINE_B + 2]
                        .into_iter()
                        .enumerate()
                        .map(|(n, node)| {
                            GraphOperation::AddAssertion(Box::new(timeline_fact(
                                TIMELINE_ASSERTIONS + 0x100 + n as u64,
                                node,
                                KIND_B_VALUE,
                                Some(JANUARY_FIRST_0900_MS + 10 * DAY_MS + 180 * MINUTE_MS),
                            )))
                        })
                        .collect(),
                    None,
                );
                writer.commit(
                    vec![GraphOperation::AddAssertion(Box::new(timeline_fact(
                        TIMELINE_ASSERTIONS + 0x200,
                        TIMELINE_B + 1,
                        KIND_B_VALUE,
                        // 2027-06-01T12:00:00Z.
                        Some(1_811_851_200_000),
                    )))],
                    None,
                );
            }
            Self::Growth => {
                writer.seed(growth_seed());
                writer.commit(growth_second(), None);
            }
            Self::Subjects => writer.seed(subjects_seed()),
            Self::Evolved => {
                let mut document = seed(3, true, false, 2);
                let described = Node::<Value>::new(
                    id(DESCRIBED),
                    document.graph.root.id,
                    id(SUBJECT),
                    "described",
                );
                document.graph.nodes.insert(described.id, described);
                writer.seed(document);
                writer.commit(first_schema_change(true), Some(id(VERSION_1)));
                writer.commit(observed(), None);
                writer.commit(retraction(SEEDED_CLAIM), None);
                writer.commit(
                    vec![GraphOperation::ModifyProperty(PropertyModification {
                        owner: Some(id(OBSERVATION)),
                        property: PropertyDefinition::new(id(SOURCE), "source", ValueType::String),
                    })],
                    Some(id(VERSION_2)),
                );
                writer.commit(
                    vec![GraphOperation::UpdateProperty(PropertyMutation {
                        node: id(BETA),
                        property: id(NOTE),
                        values: vec![Value::String("second note".into())],
                    })],
                    None,
                );
            }
        }
    }
}

/// Seeds `store`, then commits `extra` revisions, each updating one property: the store the
/// render-time measurement reads.
pub fn build_long(runtime: &Runtime, extra: u64) {
    let mut writer = Writer {
        runtime,
        clock: CLOCK_START_MS,
        transactions: TRANSACTIONS,
    };
    writer.seed(seed(3, true, false, 2));
    writer.commit(first_schema_change(true), Some(id(VERSION_1)));
    for n in 0..extra {
        writer.commit(
            vec![GraphOperation::UpdateProperty(PropertyMutation {
                node: id(ALPHA),
                property: id(NOTE),
                values: vec![Value::String(format!("note {n}"))],
            })],
            None,
        );
    }
}

/// Commits one more transaction onto a built [`Fixture::Evolved`] store, updating `beta`'s note
/// to `note {n}`: a commit unrelated to any earlier revision, which moves only the head. Each `n`
/// is a distinct transaction, timed after everything the fixture itself committed.
pub fn commit_unrelated(runtime: &Runtime, n: u64) {
    let mut writer = Writer {
        runtime,
        clock: CLOCK_START_MS + 1_000_000 + 10 * i64::try_from(n).expect("a small n"),
        transactions: TRANSACTIONS + 0x1000 + n,
    };
    writer.commit(
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: id(BETA),
            property: id(NOTE),
            values: vec![Value::String(format!("note {n}"))],
        })],
        None,
    );
}

/// The `edge-assertion` seed, then one transaction creating the node [`SHARED`] and a `links`
/// edge from it to `alpha` whose id is the same UUID, with one assertion about each: a node and
/// an edge that share an id, which the kernel admits because it keeps node and edge identities
/// apart.
pub fn build_shared_id(runtime: &Runtime) {
    let mut writer = Writer {
        runtime,
        clock: CLOCK_START_MS,
        transactions: TRANSACTIONS,
    };
    writer.seed(seed(1, false, true, 2));
    writer.commit(
        vec![
            GraphOperation::CreateNode(NodeDraft {
                id: id(SHARED),
                root_id: id(2),
                type_id: id(SUBJECT),
                canonical_name: "shared".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            GraphOperation::CreateEdge(EdgeDraft {
                id: id(SHARED),
                root_id: id(2),
                type_id: id(LINKS),
                source: id(SHARED),
                target: id(ALPHA),
                properties: BTreeMap::new(),
            }),
            GraphOperation::AddAssertion(Box::new(claim(
                SHARED_NODE_CLAIM,
                Subject::Node(id(SHARED)),
                LABEL,
                "the node",
                &[0],
            ))),
            GraphOperation::AddAssertion(Box::new(claim(
                SHARED_EDGE_CLAIM,
                Subject::Edge(id(SHARED)),
                WEIGHT,
                "the edge",
                &[0],
            ))),
        ],
        None,
    );
}

struct Writer<'a> {
    runtime: &'a Runtime,
    clock: i64,
    transactions: u64,
}

impl Writer<'_> {
    fn tick(&mut self) -> Timestamp {
        self.clock += 1;
        Timestamp::from_millis(self.clock)
    }

    fn seed(&mut self, document: SeedDocument) {
        let now = self.tick();
        self.runtime
            .seed(document, || now)
            .expect("the fixture seed is admitted");
    }

    fn commit(&mut self, operations: Vec<GraphOperation>, schema_version: Option<SchemaVersionId>) {
        let evidence = operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        self.transactions += 1;
        let transaction = GraphTransaction {
            id: id::<TransactionId>(self.transactions),
            proposer: context().operator,
            operations,
            evidence,
            schema_version,
        };
        let (proposed, validated, committed) = (self.tick(), self.tick(), self.tick());
        self.runtime
            .propose(&encode(&transaction), context().operator, || proposed)
            .expect("the fixture proposal is retained");
        let head = self.runtime.head().expect("head").expect("seeded").revision;
        let verdict = self
            .runtime
            .validate(transaction.id, head, || validated)
            .expect("validation runs");
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "the fixture transaction validates: {verdict:?}"
        );
        let result = self
            .runtime
            .commit(transaction.id, context().operator, || committed)
            .expect("commit runs");
        assert!(
            matches!(result, CommitCommandResult::Committed(_)),
            "{result:?}"
        );
    }
}

fn encode(transaction: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction,
    })
    .expect("a transaction document")
    .into_bytes()
}

fn evidence_id(n: u64) -> EvidenceId {
    id(EVIDENCE + n)
}

/// A seed: the `Subject` type with `label`, `nodes` subjects, `evidence` retained human
/// statements, optionally one assertion citing all of them, and optionally a `links` edge type
/// with one edge between the first two subjects carrying one assertion.
fn seed(evidence: u64, claimed: bool, edge: bool, nodes: u64) -> SeedDocument {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let mut subject = NodeType::new(id(SUBJECT), "Subject");
    subject.properties.insert(
        id(LABEL),
        PropertyDefinition::new(id(LABEL), "label", ValueType::String),
    );
    document.ontology.node_types.push(subject);
    for n in 0..nodes {
        let name = ["alpha", "beta"][usize::try_from(n).expect("small")];
        let mut node = Node::<Value>::new(id(ALPHA + n), root, id(SUBJECT), name);
        node.aliases = vec![format!("{name}-alias-b"), format!("{name}-alias-a")];
        document.graph.nodes.insert(node.id, node);
    }
    for n in 0..evidence {
        let bytes = format!("synthetic statement {n}").into_bytes();
        let hash = ContentHash::of_bytes(&bytes);
        let record = Evidence {
            id: evidence_id(n),
            source: EvidenceSource::HumanStatement {
                identity: Some(["operator", "reviewer", "observer"][n as usize].into()),
            },
            content_hash: hash,
            extracted_by: context().operator,
            observed_at: Timestamp::from_millis(1_000 + i64::try_from(n).expect("small")),
            confidence: Confidence::from_basis_points(9_000).expect("basis points"),
        };
        document.graph.evidence.insert(record.id, record);
        document.evidence_payloads.insert(hash, bytes);
    }
    if claimed {
        let cited: Vec<u64> = (0..evidence).collect();
        let assertion = claim(
            SEEDED_CLAIM,
            Subject::Node(id(ALPHA)),
            LABEL,
            "alpha",
            &cited,
        );
        document.graph.assertions.insert(assertion.id, assertion);
    }
    if edge {
        let mut links = EdgeType::new(id(LINKS), "links");
        links.source_types = [id(SUBJECT)].into_iter().collect();
        links.target_types = [id(SUBJECT)].into_iter().collect();
        links.properties.insert(
            id(WEIGHT),
            PropertyDefinition::new(id(WEIGHT), "weight", ValueType::String),
        );
        document.ontology.edge_types.push(links);
        let edge = Edge::<Value> {
            id: id(SEEDED_EDGE),
            root_id: root,
            type_id: id(LINKS),
            source: id(ALPHA),
            target: id(BETA),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
        let assertion = claim(
            EDGE_CLAIM,
            Subject::Edge(id(SEEDED_EDGE)),
            WEIGHT,
            "strong",
            &[0],
        );
        document.graph.assertions.insert(assertion.id, assertion);
    }
    document
}

/// `tests/fixtures/seed-empty.yaml`: schema version 1, the root, and nothing else.
fn empty_seed() -> SeedDocument {
    SeedDocument::from_yaml(
        &std::fs::read_to_string(
            Path::new(
                &std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest dir"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .expect("the empty seed fixture"),
    )
    .expect("the empty seed parses")
}

/// One retained human statement with the id `n`, in `document`.
fn statement(document: &mut SeedDocument, n: u64) -> EvidenceId {
    let bytes = format!("synthetic statement {n:x}").into_bytes();
    let hash = ContentHash::of_bytes(&bytes);
    let record = Evidence {
        id: id(n),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_000),
        confidence: Confidence::from_basis_points(9_000).expect("basis points"),
    };
    document.graph.evidence.insert(record.id, record);
    document.evidence_payloads.insert(hash, bytes);
    id(n)
}

/// A proposed assertion, citing `evidence`, valid from `valid_from` when one is given and
/// unbounded otherwise.
fn fact(
    assertion: u64,
    subject: Subject<NodeId, EdgeId>,
    predicate: Predicate,
    object: Object<Value>,
    valid_from: Option<i64>,
    evidence: EvidenceId,
) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(assertion),
        root_id: id(2),
        subject,
        predicate,
        object,
        evidence: [evidence].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: valid_from.map_or(TemporalRange::UNBOUNDED, |from| {
            TemporalRange::since(Timestamp::from_millis(from))
        }),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

/// The `hub` seed: one node type with no property, one edge type from it to itself, the hub,
/// 600 leaves named `leaf-001` to `leaf-600` in id order — leaf-600 alone with an alias,
/// `Hub-Leaf` — and edge n from the hub to leaf n. No assertion and no evidence.
fn hub() -> SeedDocument {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    document
        .ontology
        .node_types
        .push(NodeType::new(id(HUB_NODE_TYPE), "vertex"));
    let mut joins = EdgeType::new(id(HUB_EDGE_TYPE), "joins");
    joins.source_types = [id(HUB_NODE_TYPE)].into_iter().collect();
    joins.target_types = [id(HUB_NODE_TYPE)].into_iter().collect();
    joins.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(joins);
    let hub = Node::<Value>::new(id(HUB), root, id(HUB_NODE_TYPE), "hub");
    document.graph.nodes.insert(hub.id, hub);
    for n in 1..=HUB_LEAVES {
        let mut leaf =
            Node::<Value>::new(id(HUB + n), root, id(HUB_NODE_TYPE), format!("leaf-{n:03}"));
        if n == HUB_LEAVES {
            leaf.aliases = vec!["Hub-Leaf".into()];
        }
        document.graph.nodes.insert(leaf.id, leaf);
        let edge = Edge::<Value> {
            id: id(HUB_EDGES + n),
            root_id: root,
            type_id: id(HUB_EDGE_TYPE),
            source: id(HUB),
            target: id(HUB + n),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
    }
    document
}

/// A timeline assertion: `node`'s own String property `property`, valid from `valid_from`.
fn timeline_fact(
    assertion: u64,
    node: u64,
    property: u64,
    valid_from: Option<i64>,
) -> Assertion<Value> {
    fact(
        assertion,
        Subject::Node(id(node)),
        Predicate::Property(id(property)),
        Object::Value(Value::String("observed".into())),
        valid_from,
        id(TIMELINE_EVIDENCE),
    )
}

/// The `timeline` seed, as the scenario's header states it.
fn timeline_seed() -> SeedDocument {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    for (type_id, name, property, property_name) in [
        (KIND_A, "kind-a", KIND_A_VALUE, "a-value"),
        (KIND_B, "kind-b", KIND_B_VALUE, "b-value"),
        (KIND_C, "kind-c", KIND_C_VALUE, "c-value"),
    ] {
        let mut declared = NodeType::new(id(type_id), name);
        declared.properties.insert(
            id(property),
            PropertyDefinition::new(id(property), property_name, ValueType::String),
        );
        document.ontology.node_types.push(declared);
    }
    let mut a_to_b = EdgeType::new(id(KIND_A_TO_B), "a-to-b");
    a_to_b.source_types = [id(KIND_A)].into_iter().collect();
    a_to_b.target_types = [id(KIND_B)].into_iter().collect();
    document.ontology.edge_types.push(a_to_b);
    statement(&mut document, TIMELINE_EVIDENCE);

    let mut facts = Vec::new();
    let mut next = TIMELINE_ASSERTIONS;
    let mut fact_at = |node: u64, property: u64, valid_from: Option<i64>| {
        next += 1;
        facts.push(timeline_fact(next, node, property, valid_from));
    };
    let mut nodes = Vec::new();
    for i in 1..=5_u64 {
        nodes.push((TIMELINE_A + i, KIND_A, format!("a-{i}")));
        let day = JANUARY_FIRST_0900_MS + (i as i64 - 1) * DAY_MS;
        fact_at(TIMELINE_A + i, KIND_A_VALUE, Some(day));
        fact_at(TIMELINE_A + i, KIND_A_VALUE, Some(day + 30 * MINUTE_MS));
    }
    for i in 1..=2_u64 {
        nodes.push((TIMELINE_B + i, KIND_B, format!("b-{i}")));
        fact_at(
            TIMELINE_B + i,
            KIND_B_VALUE,
            Some(JANUARY_FIRST_0900_MS + 180 * MINUTE_MS),
        );
    }
    fact_at(TIMELINE_B + 2, KIND_B_VALUE, None);
    for i in 1..=6_u64 {
        nodes.push((TIMELINE_C + i, KIND_C, format!("c-{i}")));
        let second_day_1000 = JANUARY_FIRST_0900_MS + DAY_MS + 60 * MINUTE_MS;
        fact_at(TIMELINE_C + i, KIND_C_VALUE, Some(second_day_1000));
        fact_at(
            TIMELINE_C + i,
            KIND_C_VALUE,
            Some(second_day_1000 + 20 * MINUTE_MS),
        );
    }
    for (node, type_id, name) in nodes {
        let node = Node::<Value>::new(id(node), root, id(type_id), name);
        document.graph.nodes.insert(node.id, node);
    }
    for assertion in facts {
        document.graph.assertions.insert(assertion.id, assertion);
    }
    for (n, target) in [1_u64, 2, 1, 2, 1].into_iter().enumerate() {
        let n = n as u64 + 1;
        let edge = Edge::<Value> {
            id: id(TIMELINE_EDGES + n),
            root_id: root,
            type_id: id(KIND_A_TO_B),
            source: id(TIMELINE_A + n),
            target: id(TIMELINE_B + target),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
    }
    document
}

/// The `subjects` seed, as a-timeline-row-is-a-subject-with-its-events-within-hops.yaml states
/// it: holders, places, happenings E0 to E6 (E1 to E6 each with two facts ten minutes apart on
/// January i, E0 with none) and notices N1 to N3 timed by an Integer property, joined by thirteen
/// `touches` edges.
fn subjects_seed() -> SeedDocument {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let types = [
        (HOLDER, "holder", ValueType::String),
        (PLACE, "place", ValueType::String),
        (HAPPENING, "happening", ValueType::String),
        (NOTICE, "notice", ValueType::Integer),
    ];
    for (n, (type_id, name, kind)) in types.iter().enumerate() {
        let property = SUBJECTS_PROPERTIES + n as u64;
        let mut declared = NodeType::new(id(*type_id), *name);
        declared.properties.insert(
            id(property),
            PropertyDefinition::new(id(property), format!("{name}-value"), kind.clone()),
        );
        document.ontology.node_types.push(declared);
    }
    let mut touches = EdgeType::new(id(TOUCHES), "touches");
    touches.source_types = types.iter().map(|(type_id, _, _)| id(*type_id)).collect();
    touches.target_types = touches.source_types.clone();
    touches.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(touches);
    let evidence = statement(&mut document, SUBJECTS_EVIDENCE);

    let mut add =
        |node: u64, type_id: u64, name: String, properties: BTreeMap<PropertyId, Vec<Value>>| {
            let mut held = Node::<Value>::new(id(node), root, id(type_id), name);
            held.properties = properties;
            document.graph.nodes.insert(held.id, held);
        };
    for n in 1..=3 {
        add(HOLDERS + n, HOLDER, format!("h{n}"), BTreeMap::new());
    }
    for n in 1..=2 {
        add(PLACES + n, PLACE, format!("l{n}"), BTreeMap::new());
    }
    for n in 0..=6 {
        add(HAPPENINGS + n, HAPPENING, format!("e{n}"), BTreeMap::new());
    }
    // January 8 and 9 and March 1 2027, at 12:00 UTC.
    let noon = JANUARY_FIRST_0900_MS + 180 * MINUTE_MS;
    for (n, day) in [(1_u64, 7_i64), (2, 8), (3, 59)] {
        add(
            NOTICES + n,
            NOTICE,
            format!("n{n}"),
            BTreeMap::from([(
                id(SUBJECTS_PROPERTIES + 3),
                vec![Value::Integer(noon + day * DAY_MS)],
            )]),
        );
    }
    let mut next = SUBJECTS_ASSERTIONS;
    for n in 1..=6_u64 {
        let ten = JANUARY_FIRST_0900_MS + (n as i64 - 1) * DAY_MS + 60 * MINUTE_MS;
        for at in [ten, ten + 10 * MINUTE_MS] {
            next += 1;
            let claim = fact(
                next,
                Subject::Node(id(HAPPENINGS + n)),
                Predicate::Property(id(SUBJECTS_PROPERTIES + 2)),
                Object::Value(Value::String("happened".into())),
                Some(at),
                evidence,
            );
            document.graph.assertions.insert(claim.id, claim);
        }
    }
    let (h, l, e, n) = (HOLDERS, PLACES, HAPPENINGS, NOTICES);
    let joins = [
        (h + 1, e + 1),
        (e + 2, h + 1),
        (h + 1, e),
        (h + 1, l + 1),
        (e + 3, l + 1),
        (l + 1, e + 4),
        (h + 1, l + 2),
        (h + 2, l + 2),
        (e + 5, l + 2),
        (e + 1, e + 6),
        (e + 6, n + 1),
        (e + 1, h + 3),
        (h + 2, n + 2),
    ];
    for (at, (source, target)) in joins.into_iter().enumerate() {
        let edge = Edge::<Value> {
            id: id(SUBJECTS_EDGES + 1 + at as u64),
            root_id: root,
            type_id: id(TOUCHES),
            source: id(source),
            target: id(target),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
    }
    document
}

/// The `growth` seed: one node type with one String property, one edge type from it to itself,
/// the node `first`, and one retained statement for revision 1 to cite.
fn growth_seed() -> SeedDocument {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let mut item = NodeType::new(id(GROWTH_NODE_TYPE), "item");
    item.properties.insert(
        id(GROWTH_PROPERTY),
        PropertyDefinition::new(id(GROWTH_PROPERTY), "remark", ValueType::String),
    );
    document.ontology.node_types.push(item);
    let mut follows = EdgeType::new(id(GROWTH_EDGE_TYPE), "follows");
    follows.source_types = [id(GROWTH_NODE_TYPE)].into_iter().collect();
    follows.target_types = [id(GROWTH_NODE_TYPE)].into_iter().collect();
    document.ontology.edge_types.push(follows);
    let first = Node::<Value>::new(id(GROWTH_FIRST), root, id(GROWTH_NODE_TYPE), "first");
    document.graph.nodes.insert(first.id, first);
    statement(&mut document, GROWTH_EVIDENCE);
    document
}

/// `growth`'s revision 1: `second`, a value assertion about it, an edge from it to `first`, and
/// the Relation assertion that matches that edge on source, type and target.
fn growth_second() -> Vec<GraphOperation> {
    vec![
        GraphOperation::CreateNode(NodeDraft {
            id: id(GROWTH_SECOND),
            root_id: id(2),
            type_id: id(GROWTH_NODE_TYPE),
            canonical_name: "second".into(),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }),
        GraphOperation::AddAssertion(Box::new(fact(
            GROWTH_VALUE_ASSERTION,
            Subject::Node(id(GROWTH_SECOND)),
            Predicate::Property(id(GROWTH_PROPERTY)),
            Object::Value(Value::String("added".into())),
            None,
            id(GROWTH_EVIDENCE),
        ))),
        GraphOperation::CreateEdge(EdgeDraft {
            id: id(GROWTH_EDGE),
            root_id: id(2),
            type_id: id(GROWTH_EDGE_TYPE),
            source: id(GROWTH_SECOND),
            target: id(GROWTH_FIRST),
            properties: BTreeMap::new(),
        }),
        GraphOperation::AddAssertion(Box::new(fact(
            GROWTH_RELATION_ASSERTION,
            Subject::Node(id(GROWTH_SECOND)),
            Predicate::Relation(id(GROWTH_EDGE_TYPE)),
            Object::Node(id(GROWTH_FIRST)),
            None,
            id(GROWTH_EVIDENCE),
        ))),
    ]
}

/// A generated store for the index measurement: `nodes` nodes of three types, `edges` edges
/// between them chosen by a fixed linear congruential sequence (every tenth one into a hub of a
/// few hundred nodes, so the degree distribution has a tail), and one dated assertion per node.
/// A seed only, so the kernel admits it once.
pub fn build_large(runtime: &Runtime, nodes: u64, edges: u64) {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let types = [0x50_0001_u64, 0x50_0002, 0x50_0003];
    let property = 0x50_0010_u64;
    let edge_type = 0x50_0004_u64;
    for (n, type_id) in types.iter().enumerate() {
        let mut declared = NodeType::new(id(*type_id), format!("generated-{n}"));
        declared.properties.insert(
            id(property + n as u64),
            PropertyDefinition::new(id(property + n as u64), "value", ValueType::String),
        );
        document.ontology.node_types.push(declared);
    }
    let mut joins = EdgeType::new(id(edge_type), "joins");
    joins.source_types = types.iter().map(|t| id(*t)).collect();
    joins.target_types = types.iter().map(|t| id(*t)).collect();
    joins.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(joins);
    let evidence = statement(&mut document, 0x50_0020);
    let node_base = 0x60_0000_0000_u64;
    for n in 0..nodes {
        let kind = (n % 3) as usize;
        let node = Node::<Value>::new(
            id(node_base + n),
            root,
            id(types[kind]),
            format!("generated node {n}"),
        );
        document.graph.nodes.insert(node.id, node);
        let assertion = fact(
            0x70_0000_0000 + n,
            Subject::Node(id(node_base + n)),
            Predicate::Property(id(property + kind as u64)),
            Object::Value(Value::String("generated".into())),
            Some(JANUARY_FIRST_0900_MS + (n % 400) as i64 * DAY_MS),
            evidence,
        );
        document.graph.assertions.insert(assertion.id, assertion);
    }
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut step = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };
    for e in 0..edges {
        let source = step() % nodes;
        let mut target = if e % 10 == 0 {
            step() % 300.min(nodes)
        } else {
            step() % nodes
        };
        if target == source {
            target = (source + 1) % nodes;
        }
        let edge = Edge::<Value> {
            id: id(0x80_0000_0000 + e),
            root_id: root,
            type_id: id(edge_type),
            source: id(node_base + source),
            target: id(node_base + target),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
    }
    Writer {
        runtime,
        clock: CLOCK_START_MS,
        transactions: TRANSACTIONS,
    }
    .seed(document);
}

/// A proposed property assertion about `subject`, citing the fixture's evidence `cited`.
fn claim(
    assertion: u64,
    subject: Subject<NodeId, EdgeId>,
    property: u64,
    value: &str,
    cited: &[u64],
) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(assertion),
        root_id: id(2),
        subject,
        predicate: Predicate::Property(id::<PropertyId>(property)),
        object: Object::Value(Value::String(value.into())),
        evidence: cited.iter().map(|n| evidence_id(*n)).collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(500)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

fn retraction(assertion: u64) -> Vec<GraphOperation> {
    vec![GraphOperation::RetractAssertion(Retraction {
        assertion: id(assertion),
        reason: RetractionReason::new("withdrawn by the operator"),
    })]
}

/// Version 1: the `Observation` node type, the `observes` edge type from it to `Subject`, and
/// `note` on `Subject`. `rich` gives `Observation` a `summary` and `observes` a `strength`.
fn first_schema_change(rich: bool) -> Vec<GraphOperation> {
    let mut observation = NodeType::new(id::<TypeId>(OBSERVATION), "Observation");
    let mut observes = EdgeType::new(id::<TypeId>(OBSERVES), "observes");
    observes.source_types = [id(OBSERVATION)].into_iter().collect();
    observes.target_types = [id(SUBJECT)].into_iter().collect();
    if rich {
        observation.properties.insert(
            id(SUMMARY),
            PropertyDefinition::new(id(SUMMARY), "summary", ValueType::String),
        );
        observes.properties.insert(
            id(STRENGTH),
            PropertyDefinition::new(id(STRENGTH), "strength", ValueType::String),
        );
    }
    vec![
        GraphOperation::DefineNodeType(Box::new(observation)),
        GraphOperation::DefineEdgeType(Box::new(observes)),
        GraphOperation::ModifyProperty(PropertyModification {
            owner: Some(id(SUBJECT)),
            property: PropertyDefinition::new(id(NOTE), "note", ValueType::String),
        }),
    ]
}

/// Data using version 1: an observation, an `observes` edge to `alpha` with an assertion about
/// it, and a note on `alpha`.
fn observed() -> Vec<GraphOperation> {
    vec![
        GraphOperation::CreateNode(NodeDraft {
            id: id(GAMMA),
            root_id: id(2),
            type_id: id(OBSERVATION),
            canonical_name: "first observation".into(),
            properties: BTreeMap::from([(
                id(SUMMARY),
                vec![Value::String("observed after the schema change".into())],
            )]),
            aliases: Vec::new(),
        }),
        GraphOperation::CreateEdge(EdgeDraft {
            id: id(OBSERVED_EDGE),
            root_id: id(2),
            type_id: id(OBSERVES),
            source: id(GAMMA),
            target: id(ALPHA),
            properties: BTreeMap::from([(id(STRENGTH), vec![Value::String("weak".into())])]),
        }),
        GraphOperation::UpdateProperty(PropertyMutation {
            node: id(ALPHA),
            property: id(NOTE),
            values: vec![Value::String("annotated".into())],
        }),
        GraphOperation::AddAssertion(Box::new(claim(
            OBSERVED_EDGE_CLAIM,
            Subject::Edge(id(OBSERVED_EDGE)),
            STRENGTH,
            "weak",
            &[1],
        ))),
    ]
}
