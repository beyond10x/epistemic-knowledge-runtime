//! One fixed current-format seed and its trusted host context, shared by `current_vectors.rs` and
//! `current_root_sensitivity.rs`. Every identity, time and payload is a literal, so a seed through
//! the real kernel produces the same retained bytes on every run and on both providers.
//!
//! Fixtures use the runtime's own vocabulary only.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use ekr_core::{AgentId, ContentHash, Timestamp};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Edge, Evidence, EvidenceSource,
    GraphRoot, Node, Object, Predicate, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Runtime, SeedDocument, SeedResultV1,
    ValidationProfileV1,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, OntologyDocument, PropertyDefinition, SchemaVersion, Value,
    ValueType,
};
use ekr_store::GraphDocument;

/// A fixed identity in the UUID text form the other kernel fixtures use.
pub fn id<T: FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

/// Exact retained HumanStatement bytes.
pub const STATEMENT: &[u8] = b"synthetic human statement for the current seed";

/// The seed's clock.
pub const SEEDED_AT: Timestamp = Timestamp::from_millis(10);

pub fn context() -> BootstrapContext {
    BootstrapContext {
        operator: id(0x03),
        validator: id(0x04),
    }
}

fn agent(id: AgentId, name: &str, capabilities: &[&str]) -> (AgentId, Agent) {
    (
        id,
        Agent {
            id,
            name: name.into(),
            capabilities: capabilities.iter().map(|c| (*c).to_owned()).collect(),
        },
    )
}

/// The host anchor: both bootstrap identities and one registered agent the seed never uses.
pub fn anchor_for(context: BootstrapContext) -> AuthorityStateV1 {
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [
            agent(context.operator, "operator", &["propose"]),
            agent(context.validator, "validator", &["validate"]),
            agent(id(0x09), "observer", &["read"]),
        ]
        .into_iter()
        .collect(),
        validation_profile: ValidationProfileV1::deterministic(context.validator),
    }
}

pub fn anchor() -> AuthorityStateV1 {
    anchor_for(context())
}

pub fn ontology() -> OntologyDocument {
    let (subject, label, relates) = (id(0x05), id(0x06), id(0x07));
    let mut node_type = NodeType::new(subject, "Subject");
    let mut definition = PropertyDefinition::new(label, "label", ValueType::String);
    definition.cardinality = Cardinality::Many;
    node_type.properties.insert(label, definition);
    let mut edge_type = EdgeType::new(relates, "relates");
    edge_type.source_types.insert(subject);
    edge_type.target_types.insert(subject);
    OntologyDocument {
        version: SchemaVersion::seed(id(0x01), Timestamp::EPOCH),
        node_types: vec![node_type],
        edge_types: vec![edge_type],
    }
}

/// Two nodes, one edge, one retained human statement and one proposed assertion citing it.
pub fn seed() -> SeedDocument {
    let root = id(0x02);
    let (subject, label, relates) = (id(0x05), id(0x06), id(0x07));
    let mut first = Node::<Value>::new(id(0x10), root, subject, "first");
    first
        .properties
        .insert(label, vec![Value::String("alpha".into())]);
    let second = Node::<Value>::new(id(0x11), root, subject, "second");
    let edge = Edge::<Value>::new(id(0x12), root, relates, first.id, second.id);
    let evidence = Evidence {
        id: id(0x13),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(STATEMENT),
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let assertion = Assertion {
        id: id(0x14),
        root_id: root,
        subject: Subject::Node(first.id),
        predicate: Predicate::Property(label),
        object: Object::Value(Value::String("alpha".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    SeedDocument {
        format: "ekr-seed/2".into(),
        ontology: ontology(),
        graph: GraphDocument {
            root: GraphRoot {
                id: root,
                space: Space::Canonical,
                schema_version_id: id(0x01),
                parent: None,
                created_at: Timestamp::EPOCH,
            },
            revision: ekr_core::RevisionNumber::SEED,
            nodes: BTreeMap::from([(first.id, first), (second.id, second)]),
            edges: BTreeMap::from([(edge.id, edge)]),
            assertions: BTreeMap::from([(assertion.id, assertion)]),
            evidence: BTreeMap::from([(evidence.id, evidence)]),
        },
        evidence_payloads: BTreeMap::from([(ContentHash::of_bytes(STATEMENT), STATEMENT.to_vec())]),
    }
}

pub fn open(
    path: &std::path::Path,
    file: bool,
    context: BootstrapContext,
    anchor: AuthorityStateV1,
) -> Runtime {
    if file {
        Runtime::file(path, "ekr", context, anchor)
    } else {
        Runtime::sqlite(&path.join("state.db"), "ekr", context, anchor)
    }
    .unwrap()
}

/// A fresh store seeded through the real kernel. The directory is returned so it outlives the
/// runtime.
pub fn seeded(
    document: SeedDocument,
    context: BootstrapContext,
    anchor: AuthorityStateV1,
    file: bool,
    at: Timestamp,
) -> (tempfile::TempDir, Runtime, SeedResultV1) {
    let directory = tempfile::tempdir().unwrap();
    let runtime = open(directory.path(), file, context, anchor);
    let result = runtime.seed(document, || at).unwrap();
    (directory, runtime, result)
}
