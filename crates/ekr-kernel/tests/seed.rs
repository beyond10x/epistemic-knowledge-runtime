//! Seed admission must cross the real kernel on both persistent backends.
#[path = "support/seed_corruption.rs"]
mod seed_corruption;
use std::collections::BTreeSet;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, RevisionNumber,
    SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, Assessment, Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object,
    Predicate, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Commit, SeedDocument, ValidationProfileV1,
};
use ekr_ontology::{EdgeType, NodeType, Ontology, OntologyDocument, SchemaVersion, Value};
use ekr_store::{FileStore, GraphDocument, Initialize, ObjectStore, RevisionLog, SqliteStore};
use tempfile::TempDir;

fn anchor(context: BootstrapContext) -> AuthorityStateV1 {
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [
            (context.operator, "operator"),
            (context.validator, "validator"),
        ]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                Agent {
                    id,
                    name: name.into(),
                    capabilities: Default::default(),
                },
            )
        })
        .collect(),
        validation_profile: ValidationProfileV1::deterministic(context.validator),
    }
}

#[derive(Clone, Copy, Debug)]
enum Backend {
    Sqlite,
    File,
}

struct Fixture {
    ontology: OntologyDocument,
    graph: GraphDocument,
    operator: AgentId,
    validator: AgentId,
    statement: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let schema = SchemaVersionId::mint();
        let root = GraphRootId::mint();
        let (node_type, edge_type) = (TypeId::mint(), TypeId::mint());
        let (operator, validator) = (AgentId::mint(), AgentId::mint());
        let node = Node::<Value>::new(NodeId::mint(), root, node_type, "bootstrap");
        let mut relation = EdgeType::new(edge_type, "depends_on");
        relation.source_types.insert(node_type);
        relation.target_types.insert(node_type);
        let edge = Edge::<Value>::new(EdgeId::mint(), root, edge_type, node.id, node.id);
        let statement = b"Bootstrap this runtime with validated evidence.".to_vec();
        let evidence = Evidence {
            id: EvidenceId::mint(),
            source: EvidenceSource::HumanStatement { identity: None },
            content_hash: ContentHash::of_bytes(&statement),
            extracted_by: operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        let assertion = Assertion {
            id: AssertionId::mint(),
            root_id: root,
            subject: Subject::Node(node.id),
            predicate: Predicate::Relation(edge_type),
            object: Object::Node(node.id),
            evidence: [evidence.id].into_iter().collect(),
            proposed_by: operator,
            assessment: Assessment::Proposed,
            lifecycle: ekr_graph::AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        Self {
            ontology: OntologyDocument {
                version: SchemaVersion::seed(schema, Timestamp::EPOCH),
                node_types: vec![NodeType::new(node_type, "Runtime")],
                edge_types: vec![relation],
            },
            graph: GraphDocument {
                root: GraphRoot {
                    id: root,
                    space: Space::Canonical,
                    schema_version_id: schema,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::SEED,
                nodes: [(node.id, node)].into_iter().collect(),
                edges: [(edge.id, edge)].into_iter().collect(),
                assertions: [(assertion.id, assertion)].into_iter().collect(),
                evidence: [(evidence.id, evidence)].into_iter().collect(),
            },
            operator,
            validator,
            statement,
        }
    }
}

impl Fixture {
    fn document(&self) -> SeedDocument {
        SeedDocument {
            format: "ekr-seed/2".to_owned(),
            ontology: self.ontology.clone(),
            graph: self.graph.clone(),
            evidence_payloads: [(
                ContentHash::of_bytes(&self.statement),
                self.statement.clone(),
            )]
            .into_iter()
            .collect(),
        }
    }
    fn context(&self) -> BootstrapContext {
        BootstrapContext {
            operator: self.operator,
            validator: self.validator,
        }
    }
}

fn admit(backend: Backend, fixture: &Fixture) -> Result<(), String> {
    let directory = TempDir::new().unwrap();
    let ontology = Ontology::load(fixture.ontology.clone()).unwrap();
    fn through<S: RevisionLog + ObjectStore + Initialize>(
        commit: Commit<S>,
        fixture: &Fixture,
    ) -> Result<(), String> {
        commit
            .seed(fixture.document(), || Timestamp::EPOCH)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    match backend {
        Backend::Sqlite => through(
            Commit::over_with_authority(
                fixture.context(),
                anchor(fixture.context()),
                |authority| {
                    SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", ontology)
                        .map(|store| store.under(authority))
                },
            )
            .unwrap(),
            fixture,
        ),
        Backend::File => through(
            Commit::over_with_authority(
                fixture.context(),
                anchor(fixture.context()),
                |authority| {
                    FileStore::file(directory.path(), "ekr", ontology)
                        .map(|store| store.under(authority))
                },
            )
            .unwrap(),
            fixture,
        ),
    }
}

#[test]
fn a_seed_with_a_dangling_edge_is_refused_by_both_backends() {
    let mut fixture = Fixture::new();
    fixture.graph.edges.values_mut().next().unwrap().target = NodeId::mint();
    let accepted: Vec<_> = [Backend::Sqlite, Backend::File]
        .into_iter()
        .filter(|backend| admit(*backend, &fixture).is_ok())
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted a dangling edge: {accepted:?}"
    );
}

#[test]
fn a_seed_with_an_undeclared_type_is_refused_by_both_backends() {
    let mut fixture = Fixture::new();
    fixture.graph.nodes.values_mut().next().unwrap().type_id = TypeId::mint();
    let accepted: Vec<_> = [Backend::Sqlite, Backend::File]
        .into_iter()
        .filter(|backend| admit(*backend, &fixture).is_ok())
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted an undeclared type: {accepted:?}"
    );
}

#[test]
fn a_seed_with_a_caller_verdict_is_refused_by_both_backends() {
    let mut fixture = Fixture::new();
    fixture
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .assessment = Assessment::Accepted {
        validators: BTreeSet::new(),
    };
    let accepted: Vec<_> = [Backend::Sqlite, Backend::File]
        .into_iter()
        .filter(|backend| admit(*backend, &fixture).is_ok())
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted a caller's verdict: {accepted:?}"
    );
}

#[test]
fn a_valid_seed_has_a_positive_control() {
    let fixture = Fixture::new();
    assert_ne!(fixture.operator, fixture.validator);
    assert!(!fixture.statement.is_empty());
    for backend in [Backend::Sqlite, Backend::File] {
        assert!(admit(backend, &fixture).is_ok());
    }
}

#[test]
fn a_seed_ontology_cannot_claim_a_later_version() {
    let mut fixture = Fixture::new();
    fixture.ontology.version.number = 1;
    let accepted: Vec<_> = [Backend::Sqlite, Backend::File]
        .into_iter()
        .filter(|backend| admit(*backend, &fixture).is_ok())
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted a non-genesis ontology: {accepted:?}"
    );
}

#[test]
fn a_seed_ontology_cannot_claim_a_predecessor() {
    let mut fixture = Fixture::new();
    fixture.ontology.version.parent = Some(SchemaVersionId::mint());
    let accepted: Vec<_> = [Backend::Sqlite, Backend::File]
        .into_iter()
        .filter(|backend| admit(*backend, &fixture).is_ok())
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted an ontology predecessor: {accepted:?}"
    );
}

trait Runtime {
    fn seed(&self, document: SeedDocument) -> Result<ekr_graph::Root, ekr_kernel::SeedError>;
    fn head(&self) -> Result<Option<ekr_graph::Root>, ekr_store::StoreError>;
    fn snapshot(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError>;
    fn replay(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError>;
    fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, ekr_store::StoreError>;
}
impl<S: RevisionLog + ObjectStore + Initialize> Runtime for Commit<S> {
    fn seed(&self, document: SeedDocument) -> Result<ekr_graph::Root, ekr_kernel::SeedError> {
        self.seed(document, || Timestamp::EPOCH)
            .map(|record| record.result)
    }
    fn head(&self) -> Result<Option<ekr_graph::Root>, ekr_store::StoreError> {
        self.head()
    }
    fn snapshot(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError> {
        self.snapshot()
    }
    fn replay(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError> {
        self.replay(RevisionNumber::SEED)
    }
    fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, ekr_store::StoreError> {
        self.content(hash)
    }
}
trait Provider: RevisionLog + ObjectStore {}
impl<S: RevisionLog + ObjectStore> Provider for S {}

impl Backend {
    fn open(
        self,
        directory: &std::path::Path,
        ontology: Ontology,
        context: BootstrapContext,
    ) -> Box<dyn Runtime> {
        self.try_open(directory, ontology, context).unwrap()
    }
    fn try_open(
        self,
        directory: &std::path::Path,
        ontology: Ontology,
        context: BootstrapContext,
    ) -> Result<Box<dyn Runtime>, ekr_store::StoreError> {
        match self {
            Self::Sqlite => Ok(Box::new(Commit::over_with_authority(
                context,
                anchor(context),
                |authority| {
                    SqliteStore::sqlite(&directory.join("state.db"), "ekr", ontology)
                        .map(|s| s.under(authority))
                },
            )?)),
            Self::File => Ok(Box::new(Commit::over_with_authority(
                context,
                anchor(context),
                |authority| FileStore::file(directory, "ekr", ontology).map(|s| s.under(authority)),
            )?)),
        }
    }
    fn raw(self, directory: &std::path::Path, ontology: Ontology) -> Box<dyn Provider> {
        match self {
            Self::Sqlite => {
                Box::new(SqliteStore::sqlite(&directory.join("state.db"), "ekr", ontology).unwrap())
            }
            Self::File => Box::new(FileStore::file(directory, "ekr", ontology).unwrap()),
        }
    }
}

fn envelope_bytes(document: &SeedDocument, context: BootstrapContext) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        format: &'a str,
        input: &'a SeedDocument,
        context: BootstrapContext,
        authority: AuthorityStateV1,
        committed_at: Timestamp,
    }
    serde_json::to_vec(&Envelope {
        format: "ekr-seed-envelope/2",
        input: document,
        context,
        authority: anchor(context),
        committed_at: Timestamp::EPOCH,
    })
    .unwrap()
}

fn refuses(fixture: &Fixture, document: &SeedDocument, code: &str) {
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let ontology = Ontology::load(fixture.ontology.clone()).unwrap();
        let runtime = backend.try_open(directory.path(), ontology.clone(), fixture.context());
        let expected_hash = ContentHash::of_bytes(&envelope_bytes(document, fixture.context()));
        let refusal = match runtime {
            Ok(runtime) => runtime
                .seed(document.clone())
                .expect_err("invalid seed must refuse"),
            Err(error) => ekr_kernel::SeedError::Store(error),
        };
        assert!(
            refusal.to_string().contains(code),
            "{backend:?}: expected {code}, got {refusal}"
        );
        let raw = backend.raw(directory.path(), ontology);
        assert_eq!(
            raw.head().unwrap(),
            None,
            "{backend:?}: invalid seed wrote a revision event"
        );
        assert_eq!(raw.seed_bytes().unwrap(), None);
        assert_eq!(
            raw.get(&expected_hash).unwrap(),
            None,
            "{backend:?}: invalid seed wrote its object"
        );
    }
}

#[test]
fn named_seed_refusals_write_neither_the_object_nor_the_revision() {
    let f = Fixture::new();
    let mut dangling = f.document();
    dangling.graph.edges.values_mut().next().unwrap().target = NodeId::mint();
    refuses(&f, &dangling, "unresolved-node");
    let mut unknown = f.document();
    unknown.graph.nodes.values_mut().next().unwrap().type_id = TypeId::mint();
    refuses(&f, &unknown, "unknown-type");
    let mut verdict = f.document();
    verdict
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .assessment = Assessment::Accepted {
        validators: BTreeSet::new(),
    };
    refuses(&f, &verdict, "assertion-states-its-own-verdict");
}

#[test]
fn canonical_seed_root_filing_schema_and_genesis_are_mandatory() {
    let f = Fixture::new();
    let mut transient = f.document();
    transient.graph.root.space = Space::Transient;
    refuses(&f, &transient, "seed-space");
    let mut schema = f.document();
    schema.graph.root.schema_version_id = SchemaVersionId::mint();
    refuses(&f, &schema, "seed-schema-version");
    for parent in [Some(f.graph.root.id), Some(GraphRootId::mint())] {
        let mut document = f.document();
        document.graph.root.parent = parent;
        refuses(&f, &document, "seed-root-lineage");
    }
    let mut later = f.document();
    later.graph.revision = RevisionNumber::new(3);
    refuses(&f, &later, "seed-root-lineage");
}

#[test]
fn every_seed_entity_map_checks_key_and_root_identity() {
    let f = Fixture::new();
    let mut documents = Vec::new();
    let mut d = f.document();
    let (_, node) = d.graph.nodes.pop_first().unwrap();
    d.graph.nodes.insert(NodeId::mint(), node);
    documents.push((d, "seed-misfiled-entity"));
    let mut d = f.document();
    let (_, edge) = d.graph.edges.pop_first().unwrap();
    d.graph.edges.insert(EdgeId::mint(), edge);
    documents.push((d, "seed-misfiled-entity"));
    let mut d = f.document();
    let (_, assertion) = d.graph.assertions.pop_first().unwrap();
    d.graph.assertions.insert(AssertionId::mint(), assertion);
    documents.push((d, "seed-misfiled-entity"));
    let mut d = f.document();
    let (_, evidence) = d.graph.evidence.pop_first().unwrap();
    d.graph.evidence.insert(EvidenceId::mint(), evidence);
    documents.push((d, "seed-misfiled-evidence"));
    let mut d = f.document();
    d.graph.nodes.values_mut().next().unwrap().root_id = GraphRootId::mint();
    documents.push((d, "seed-misrooted-entity"));
    let mut d = f.document();
    d.graph.edges.values_mut().next().unwrap().root_id = GraphRootId::mint();
    documents.push((d, "seed-misrooted-entity"));
    let mut d = f.document();
    d.graph.assertions.values_mut().next().unwrap().root_id = GraphRootId::mint();
    documents.push((d, "seed-misrooted-entity"));
    for (document, code) in documents {
        refuses(&f, &document, code);
    }
}

#[test]
fn floats_are_refused_in_seed_nodes_edges_and_assertion_objects() {
    let f = Fixture::new();
    let property = ekr_core::PropertyId::mint();
    let mut node = f.document();
    node.graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .properties
        .insert(property, vec![Value::List(vec![Value::Float(1.0)])]);
    refuses(&f, &node, "inadmissible-value");
    refuses(&f, &node, &property.to_string());
    let mut edge = f.document();
    edge.graph
        .edges
        .values_mut()
        .next()
        .unwrap()
        .properties
        .insert(property, vec![Value::Float(1.0)]);
    refuses(&f, &edge, "inadmissible-value");
    let mut assertion = f.document();
    assertion
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .object = Object::Value(Value::Float(1.0));
    refuses(&f, &assertion, "inadmissible-value");
}

#[test]
fn seed_support_checks_missing_uncited_and_unsupported_evidence() {
    let f = Fixture::new();
    let mut missing = f.document();
    missing.evidence_payloads.clear();
    refuses(&f, &missing, "seed-evidence-payload-missing");
    let mut mismatched = f.document();
    mismatched
        .evidence_payloads
        .values_mut()
        .next()
        .unwrap()
        .push(0);
    refuses(&f, &mismatched, "seed-evidence-payload-mismatch");
    let mut uncited = f.document();
    uncited.graph.assertions.clear();
    uncited.evidence_payloads.clear();
    refuses(&f, &uncited, "seed-evidence-payload-missing");
    let mut unsupported = f.document();
    unsupported
        .graph
        .evidence
        .values_mut()
        .next()
        .unwrap()
        .source = EvidenceSource::Document {
        document_id: "bootstrap".to_owned(),
        section: None,
    };
    refuses(&f, &unsupported, "seed-unsupported-source");
    let mut no_support = f.document();
    no_support
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .evidence
        .clear();
    refuses(&f, &no_support, "assertion-without-evidence");
    let mut dangling = f.document();
    dangling
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .evidence = [EvidenceId::mint()].into_iter().collect();
    refuses(&f, &dangling, "unresolved-evidence");
}

/// `story:seed-evidence-content-hash`: `seed-evidence-payload-mismatch` names the content hash
/// the payload actually has (expected) and the one the seed wrote for it (found), at both sites
/// that raise it — a cited evidence entry's payload and an uncited `evidence_payloads` entry.
///
/// After the fix: on both providers the refusal carries the code, `expected <payload's hash>`
/// and `found <declared hash>`, and nothing is written.
#[test]
fn a_payload_mismatch_names_the_expected_and_the_found_content_hash() {
    let f = Fixture::new();
    let mut cited = f.document();
    let (&found, payload) = cited.evidence_payloads.iter_mut().next().unwrap();
    payload.push(0);
    let expected = ContentHash::of_bytes(payload);
    assert_ne!(expected, found);
    for needle in [
        "seed-evidence-payload-mismatch".to_owned(),
        format!("expected {expected}"),
        format!("found {found}"),
    ] {
        refuses(&f, &cited, &needle);
    }

    let mut uncited = f.document();
    let found = ContentHash::of_bytes(b"declared bytes");
    let expected = ContentHash::of_bytes(b"retained bytes");
    uncited
        .evidence_payloads
        .insert(found, b"retained bytes".to_vec());
    for needle in [
        "seed-evidence-payload-mismatch".to_owned(),
        format!("expected {expected}"),
        format!("found {found}"),
    ] {
        refuses(&f, &uncited, &needle);
    }
}

/// Correction round 1 (p1-15): `seed-evidence-payload-missing` names the evidence entry's
/// `content_hash`, the `evidence_payloads` keys no entry names, and that the entry's hash and its
/// key must be the same value. The code string is unchanged.
///
/// After the fix: on both providers, a payload filed under another key than its entry's hash is
/// refused with the code, `has content_hash <entry's>`, `keys no evidence entry names: <key>`
/// and "must be the same value"; with no payloads at all the key list reads `none`.
#[test]
fn a_missing_payload_names_the_entry_hash_and_the_keys_no_entry_names() {
    let f = Fixture::new();
    let mut misfiled = f.document();
    let (entry, payload) = misfiled.evidence_payloads.pop_first().unwrap();
    let key = ContentHash::of_bytes(b"another payload");
    misfiled.evidence_payloads.insert(key, payload);
    for needle in [
        "seed-evidence-payload-missing: ".to_owned(),
        format!("has content_hash {entry}"),
        format!("keys no evidence entry names: {key}"),
        "must be the same value".to_owned(),
    ] {
        refuses(&f, &misfiled, &needle);
    }
    let mut none = f.document();
    none.evidence_payloads.clear();
    refuses(&f, &none, "keys no evidence entry names: none");
}

#[test]
fn actual_bootstrap_identities_refuse_self_validation_and_false_attribution() {
    let mut f = Fixture::new();
    f.validator = f.operator;
    refuses(&f, &f.document(), "proposer-is-validator");
    f.validator = AgentId::mint();
    let mut false_claim = f.document();
    false_claim
        .graph
        .assertions
        .values_mut()
        .next()
        .unwrap()
        .proposed_by = AgentId::mint();
    refuses(&f, &false_claim, "seed-attribution-mismatch");
    let mut false_evidence = f.document();
    false_evidence
        .graph
        .evidence
        .values_mut()
        .next()
        .unwrap()
        .extracted_by = AgentId::mint();
    refuses(&f, &false_evidence, "seed-attribution-mismatch");
}

#[test]
fn required_properties_and_unsupported_constraints_keep_their_refusals() {
    use ekr_ontology::{Cardinality, PropertyDefinition, ValueType};
    let mut f = Fixture::new();
    let property = ekr_core::PropertyId::mint();
    let mut definition = PropertyDefinition::new(property, "required", ValueType::String);
    definition.cardinality = Cardinality::One;
    definition.required = true;
    f.ontology.node_types[0]
        .properties
        .insert(property, definition.clone());
    refuses(&f, &f.document(), "missing-required-property");
    definition.required = false;
    definition.constraints.push("must be verified".to_owned());
    f.ontology.node_types[0]
        .properties
        .insert(property, definition);
    refuses(&f, &f.document(), "unsupported-constraint");
}

#[test]
fn seed_lifecycle_is_the_declared_initial_state_and_is_preserved() {
    let mut f = Fixture::new();
    f.ontology.node_types[0].lifecycle = Some(ekr_ontology::Lifecycle {
        initial: "open".to_owned(),
        states: ["open".to_owned(), "closed".to_owned()]
            .into_iter()
            .collect(),
        transitions: BTreeSet::new(),
    });
    refuses(&f, &f.document(), "seed-initial-lifecycle");
    f.graph.nodes.values_mut().next().unwrap().type_state = Some("closed".to_owned());
    refuses(&f, &f.document(), "seed-initial-lifecycle");
    f.graph.nodes.values_mut().next().unwrap().type_state = Some("open".to_owned());
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let runtime = backend.open(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
            f.context(),
        );
        runtime.seed(f.document()).unwrap();
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .nodes
                .values()
                .next()
                .unwrap()
                .type_state
                .as_deref(),
            Some("open")
        );
    }
}

#[test]
fn an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes() {
    let mut f = Fixture::new();
    f.graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .aliases
        .push("bootstrap-alias".to_owned());
    f.graph.root.created_at = Timestamp::from_millis(42);
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let ontology = Ontology::load(f.ontology.clone()).unwrap();
        let runtime = backend.open(directory.path(), ontology.clone(), f.context());
        let head = runtime.seed(f.document()).unwrap();
        let graph = runtime.snapshot().unwrap();
        assert_eq!(head.revision, RevisionNumber::SEED);
        let mut expected = f.graph.clone();
        expected.assertions.values_mut().next().unwrap().assessment = Assessment::Accepted {
            validators: [f.validator].into_iter().collect(),
        };
        assert_eq!(GraphDocument::of(&graph), expected);
        let hash = ContentHash::of_bytes(&f.statement);
        assert_eq!(runtime.content(&hash).unwrap(), Some(f.statement.clone()));
        drop(runtime);
        let reopened = backend.open(directory.path(), ontology, f.context());
        assert_eq!(reopened.head().unwrap(), Some(head));
        assert_eq!(reopened.snapshot().unwrap(), graph);
        assert_eq!(reopened.replay().unwrap(), graph);
        assert_eq!(reopened.content(&hash).unwrap(), Some(f.statement.clone()));
        let second_directory = TempDir::new().unwrap();
        let independent = backend.open(
            second_directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
            f.context(),
        );
        assert_eq!(independent.seed(f.document()).unwrap(), head);
    }
}

#[test]
fn empty_bootstrap_does_not_make_empty_transactions_valid() {
    let mut f = Fixture::new();
    f.graph.nodes.clear();
    f.graph.edges.clear();
    f.graph.assertions.clear();
    f.graph.evidence.clear();
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let runtime = backend.open(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
            f.context(),
        );
        let mut document = f.document();
        document.evidence_payloads.clear();
        runtime.seed(document).unwrap();
        let graph = runtime.snapshot().unwrap();
        let tx = ekr_kernel::GraphTransaction {
            id: ekr_core::TransactionId::mint(),
            proposer: f.operator,
            operations: Vec::new(),
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        assert!(ekr_kernel::Pipeline::deterministic(f.validator)
            .validate(&ekr_graph::GraphSnapshot::of(&graph), &tx)
            .unwrap_err()
            .iter()
            .any(|issue| issue.code == "empty-transaction"));
    }
}

#[test]
fn reopen_checks_full_ontology_and_execution_context() {
    let f = Fixture::new();
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let runtime = backend.open(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
            f.context(),
        );
        runtime.seed(f.document()).unwrap();
        drop(runtime);
        let mut changed = f.ontology.clone();
        changed.node_types[0].name = "Different".to_owned();
        let runtime = backend.open(
            directory.path(),
            Ontology::load(changed).unwrap(),
            f.context(),
        );
        assert!(runtime
            .head()
            .unwrap_err()
            .to_string()
            .contains("seed-ontology-mismatch"));
        let context = BootstrapContext {
            validator: AgentId::mint(),
            ..f.context()
        };
        let runtime = backend.open(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
            context,
        );
        assert!(runtime
            .head()
            .unwrap_err()
            .to_string()
            .contains("bootstrap-authority-mismatch"));
        let raw = backend.raw(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
        );
        assert_eq!(raw.head(), Err(ekr_store::StoreError::NoSeedAuthority));
        assert_eq!(raw.fold(), Err(ekr_store::StoreError::NoSeedAuthority));
    }
}

#[test]
fn repeated_initialization_preserves_the_lineage_and_writes_no_second_object() {
    let f = Fixture::new();
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let ontology = Ontology::load(f.ontology.clone()).unwrap();
        let runtime = backend.open(directory.path(), ontology.clone(), f.context());
        let head = runtime.seed(f.document()).unwrap();
        assert_eq!(runtime.seed(f.document()), Ok(head));
        let mut second = f.document();
        second
            .graph
            .nodes
            .values_mut()
            .next()
            .unwrap()
            .canonical_name = "replacement".to_owned();
        let hash = ContentHash::of_bytes(&envelope_bytes(&second, f.context()));
        assert_eq!(
            runtime.seed(second),
            Err(ekr_kernel::SeedError::Store(
                ekr_store::StoreError::AlreadySeeded
            ))
        );
        assert_eq!(runtime.head().unwrap(), Some(head));
        assert_eq!(
            backend.raw(directory.path(), ontology).get(&hash).unwrap(),
            None
        );
    }
}

#[test]
fn concurrent_independent_handles_publish_exactly_one_seed_and_no_losing_object() {
    use std::sync::{Arc, Barrier};
    let f = Fixture::new();
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        // Provision schema/directories before racing independently opened handles.
        drop(backend.raw(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
        ));
        let barrier = Arc::new(Barrier::new(2));
        let outcomes = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|number| {
                    let mut document = f.document();
                    document
                        .graph
                        .nodes
                        .values_mut()
                        .next()
                        .unwrap()
                        .canonical_name = format!("writer-{number}");
                    let hash = ContentHash::of_bytes(&envelope_bytes(&document, f.context()));
                    let barrier = Arc::clone(&barrier);
                    let path = directory.path();
                    let context = f.context();
                    scope.spawn(move || {
                        let runtime = backend.open(
                            path,
                            Ontology::load(document.ontology.clone()).unwrap(),
                            context,
                        );
                        barrier.wait();
                        (hash, runtime.seed(document))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            outcomes.iter().filter(|(_, result)| result.is_ok()).count(),
            1,
            "{backend:?}: {outcomes:?}"
        );
        let raw = backend.raw(
            directory.path(),
            Ontology::load(f.ontology.clone()).unwrap(),
        );
        for (hash, result) in outcomes {
            if result.is_err() {
                assert!(matches!(
                    result,
                    Err(ekr_kernel::SeedError::Store(
                        ekr_store::StoreError::AlreadySeeded
                            | ekr_store::StoreError::PublicationInputConflict
                    ))
                ));
                assert_eq!(raw.get(&hash).unwrap(), None);
            }
        }
    }
}

#[test]
fn legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted() {
    let f = Fixture::new();
    let mut tampered = f.document();
    tampered
        .evidence_payloads
        .values_mut()
        .next()
        .unwrap()
        .push(0);
    let mut transient = f.document();
    transient.graph.root.space = Space::Transient;
    let mut other_schema = f.document();
    other_schema.graph.root.schema_version_id = SchemaVersionId::mint();
    let fixtures = [
        (
            serde_json::to_vec(&serde_json::to_value(&f.graph).unwrap()["graph"]).unwrap(),
            "legacy seed requires migration",
        ),
        (
            envelope_bytes(&tampered, f.context()),
            "seed-evidence-payload-mismatch",
        ),
        (envelope_bytes(&transient, f.context()), "seed-space"),
        (
            envelope_bytes(&other_schema, f.context()),
            "seed-schema-version",
        ),
    ];
    for backend in [Backend::Sqlite, Backend::File] {
        for (bytes, code) in &fixtures {
            let directory = TempDir::new().unwrap();
            let ontology = Ontology::load(f.ontology.clone()).unwrap();
            let raw = backend.raw(directory.path(), ontology.clone());
            let object = raw
                .put(ekr_store::StorageClass::Canonical, bytes, Timestamp::EPOCH)
                .unwrap();
            assert_eq!(
                seed_corruption::inject(
                    directory.path(),
                    matches!(backend, Backend::File),
                    ontology.clone(),
                    bytes
                ),
                ekr_store::Appended::Written
            );
            let runtime = backend.open(directory.path(), ontology, f.context());
            assert!(runtime.snapshot().unwrap_err().to_string().contains(code));
            assert_eq!(raw.get(&object.content_hash).unwrap().as_ref(), Some(bytes));
        }
    }
}

#[test]
fn seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary() {
    let f = Fixture::new();
    let document = serde_json::to_value(f.document()).unwrap();
    let node = f.graph.nodes.keys().next().unwrap();
    let edge = f.graph.edges.keys().next().unwrap();
    let assertion = f.graph.assertions.keys().next().unwrap();
    let evidence = f.graph.evidence.keys().next().unwrap();
    for path in [
        String::new(),
        "/graph".to_owned(),
        "/graph/graph/root".to_owned(),
        format!("/graph/graph/nodes/{node}"),
        format!("/graph/graph/edges/{edge}"),
        format!("/graph/graph/assertions/{assertion}"),
        format!("/graph/graph/evidence/{evidence}"),
        format!("/graph/graph/assertions/{assertion}/valid_time"),
        format!("/graph/graph/assertions/{assertion}/transaction_time"),
        format!("/graph/graph/evidence/{evidence}/source/HumanStatement"),
    ] {
        let mut changed = document.clone();
        changed
            .pointer_mut(&path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unsupported_semantics".to_owned(), true.into());
        assert!(
            serde_json::from_value::<SeedDocument>(changed).is_err(),
            "ignored field at {path}"
        );
    }
    let yaml = serde_yaml_ng::to_string(&f.document()).unwrap();
    assert_eq!(SeedDocument::from_yaml(&yaml).unwrap(), f.document());
    assert!(
        SeedDocument::from_yaml(&yaml.replace("ekr-seed/2", "ekr-seed/999"))
            .unwrap_err()
            .to_string()
            .contains("unsupported-seed-format")
    );
}

#[test]
fn initialization_reuses_exact_cached_seed_bytes_atomically() {
    let f = Fixture::new();
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let ontology = Ontology::load(f.ontology.clone()).unwrap();
        let raw = backend.raw(directory.path(), ontology.clone());
        let bytes = envelope_bytes(&f.document(), f.context());
        let cached = raw
            .put(ekr_store::StorageClass::Cache, &bytes, Timestamp::EPOCH)
            .unwrap();
        let runtime = backend.open(directory.path(), ontology, f.context());
        let head = runtime.seed(f.document()).unwrap();
        assert_eq!(head.transaction, cached.content_hash);
        assert_eq!(runtime.content(&cached.content_hash).unwrap(), Some(bytes));
    }
}

#[test]
fn seed_multiplicity_and_property_types_are_checked() {
    let mut f = Fixture::new();
    f.ontology.edge_types[0].cardinality = ekr_ontology::Cardinality::One;
    let mut document = f.document();
    let mut edge = document.graph.edges.values().next().unwrap().clone();
    edge.id = EdgeId::mint();
    document.graph.edges.insert(edge.id, edge);
    refuses(&f, &document, "edge-cardinality");
    let property = ekr_core::PropertyId::mint();
    f.ontology.node_types[0].properties.insert(
        property,
        ekr_ontology::PropertyDefinition::new(property, "name", ekr_ontology::ValueType::String),
    );
    let mut document = f.document();
    document
        .graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .properties
        .insert(property, vec![Value::Boolean(true)]);
    refuses(&f, &document, "wrong-type");
}

#[test]
fn legitimate_record_keys_remain_data_in_strict_seed_decoding() {
    let f = Fixture::new();
    let mut document = f.document();
    document
        .graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .properties
        .insert(
            ekr_core::PropertyId::mint(),
            vec![Value::Record(
                [(
                    "arbitrary-user-key".to_owned(),
                    Value::String("content".to_owned()),
                )]
                .into_iter()
                .collect(),
            )],
        );
    let yaml = serde_yaml_ng::to_string(&document).unwrap();
    assert_eq!(SeedDocument::from_yaml(&yaml).unwrap(), document);
}

#[test]
fn the_versioned_minimal_yaml_fixture_initializes_both_real_providers() {
    let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/fixtures/seed-minimal-v2.yaml");
    let document = SeedDocument::from_yaml(&std::fs::read_to_string(path).unwrap()).unwrap();
    let context = BootstrapContext {
        operator: AgentId::mint(),
        validator: AgentId::mint(),
    };
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let runtime = backend.open(
            directory.path(),
            Ontology::load(document.ontology.clone()).unwrap(),
            context,
        );
        let head = runtime.seed(document.clone()).unwrap();
        assert_eq!(head.revision, RevisionNumber::SEED);
        assert!(runtime.snapshot().unwrap().nodes.is_empty());
    }
}
