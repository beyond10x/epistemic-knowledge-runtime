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
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
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
    /// retraction over six revisions.
    Evolved,
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
            Self::Evolved => {
                writer.seed(seed(3, true, false, 2));
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
    let mut document = SeedDocument::from_yaml(
        &std::fs::read_to_string(
            Path::new(
                &std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest dir"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .expect("the empty seed fixture"),
    )
    .expect("the empty seed parses");
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
