//! Seed admission must cross the real kernel on both persistent backends.
use std::collections::BTreeSet;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, RevisionNumber,
    SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate,
    Space, Subject, TemporalRange, TransactionTime, ValidationState,
};
use ekr_kernel::{BootstrapContext, Commit, SeedDocument};
use ekr_ontology::{EdgeType, NodeType, Ontology, OntologyDocument, SchemaVersion, Value};
use ekr_store::{FileStore, GraphDocument, Initialize, ObjectStore, RevisionLog, SqliteStore};
use tempfile::TempDir;

#[derive(Clone, Copy, Debug)]
enum Backend {
    Sqlite,
    File,
}

#[derive(Clone)]
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
            validation: ValidationState::Proposed,
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
            format: "ekr-seed/1".to_owned(),
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

// The fixture above is copied from seed.rs; adversarial cases and the helpers below are new.
trait Runtime {
    fn seed(&self, document: SeedDocument) -> Result<ekr_graph::Root, ekr_kernel::SeedError>;
    fn snapshot(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError>;
}
impl<S: RevisionLog + ObjectStore + Initialize> Runtime for Commit<S> {
    fn seed(&self, document: SeedDocument) -> Result<ekr_graph::Root, ekr_kernel::SeedError> {
        self.seed(document)
    }
    fn snapshot(&self) -> Result<ekr_graph::CanonicalGraph, ekr_store::StoreError> {
        self.snapshot()
    }
}
trait Provider: RevisionLog + ObjectStore {}
impl<S: RevisionLog + ObjectStore> Provider for S {}
impl Backend {
    fn open(self, path: &std::path::Path, fixture: &Fixture) -> Box<dyn Runtime> {
        let ontology = Ontology::load(fixture.ontology.clone()).unwrap();
        match self {
            Self::Sqlite => Box::new(
                Commit::over_with_bootstrap(fixture.context(), |authority| {
                    SqliteStore::sqlite(&path.join("state.db"), "ekr", ontology)
                        .map(|s| s.under(authority))
                })
                .unwrap(),
            ),
            Self::File => Box::new(
                Commit::over_with_bootstrap(fixture.context(), |authority| {
                    FileStore::file(path, "ekr", ontology).map(|s| s.under(authority))
                })
                .unwrap(),
            ),
        }
    }
    fn raw(self, path: &std::path::Path, fixture: &Fixture) -> Box<dyn Provider> {
        let ontology = Ontology::load(fixture.ontology.clone()).unwrap();
        match self {
            Self::Sqlite => {
                Box::new(SqliteStore::sqlite(&path.join("state.db"), "ekr", ontology).unwrap())
            }
            Self::File => Box::new(FileStore::file(path, "ekr", ontology).unwrap()),
        }
    }
}
fn envelope(document: &SeedDocument, context: BootstrapContext) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        format: &'a str,
        input: &'a SeedDocument,
        context: BootstrapContext,
    }
    serde_json::to_vec(&Envelope {
        format: "ekr-seed-envelope/1",
        input: document,
        context,
    })
    .unwrap()
}

#[test]
fn seed_ontology_property_definitions_must_be_filed_under_their_own_ids() {
    use ekr_core::PropertyId;
    use ekr_ontology::{PropertyDefinition, ValueType};
    let mut accepted = Vec::new();
    for backend in [Backend::Sqlite, Backend::File] {
        for edge_property in [false, true] {
            let mut fixture = Fixture::new();
            let valid_compatibility = fixture.clone();
            let key = PropertyId::mint();
            let declared_id = PropertyId::mint();
            let definition = PropertyDefinition::new(declared_id, "support", ValueType::String);
            assert_ne!(key, definition.id);
            if edge_property {
                fixture.ontology.edge_types[0]
                    .properties
                    .insert(key, definition);
                fixture
                    .graph
                    .edges
                    .values_mut()
                    .next()
                    .unwrap()
                    .properties
                    .insert(key, Value::String("retained".to_owned()));
            } else {
                fixture.ontology.node_types[0]
                    .properties
                    .insert(key, definition);
                fixture
                    .graph
                    .nodes
                    .values_mut()
                    .next()
                    .unwrap()
                    .properties
                    .insert(key, Value::String("retained".to_owned()));
            }
            // Preserve the current admitted-schema witness. Once the shared loader refuses
            // it, still exercise kernel admission with valid compatibility configuration.
            let configured = match Ontology::load(fixture.ontology.clone()) {
                Ok(_) => &fixture,
                Err(error) => {
                    let refusal = error.to_string();
                    assert!(refusal.contains(&key.to_string()), "{refusal}");
                    assert!(refusal.contains(&declared_id.to_string()), "{refusal}");
                    &valid_compatibility
                }
            };
            let directory = TempDir::new().unwrap();
            let runtime = backend.open(directory.path(), configured);
            match runtime.seed(fixture.document()) {
                Ok(_) => {
                    drop(runtime);
                    let reopened = backend.open(directory.path(), &fixture);
                    assert!(
                        reopened.snapshot().is_ok(),
                        "accepted malformed schema must survive reopen to expose durable boundary"
                    );
                    accepted.push((backend, edge_property));
                }
                Err(error) => {
                    assert!(error.to_string().contains("seed-ontology"), "{error}");
                    assert!(
                        !error.to_string().contains("seed-ontology-mismatch"),
                        "compatibility mismatch is not malformed declaration admission: {error}"
                    );
                    let raw = backend.raw(directory.path(), configured);
                    assert_eq!(raw.head().unwrap(), None);
                    assert_eq!(raw.seed_bytes().unwrap(), None);
                    let hash =
                        ContentHash::of_bytes(&envelope(&fixture.document(), fixture.context()));
                    assert_eq!(raw.get(&hash).unwrap(), None);
                }
            }
        }
    }
    assert!(accepted.is_empty(), "canonical seed admitted misfiled property definitions (backend, edge_property): {accepted:?}");
}

#[test]
fn losing_cached_seed_keeps_its_original_retention_class() {
    use ekr_store::StorageClass;
    use std::sync::{Arc, Barrier};
    for backend in [Backend::Sqlite, Backend::File] {
        let fixture = Fixture::new();
        let directory = TempDir::new().unwrap();
        let raw = backend.raw(directory.path(), &fixture);
        let documents: Vec<_> = (0..2)
            .map(|n| {
                let mut document = fixture.document();
                document
                    .graph
                    .nodes
                    .values_mut()
                    .next()
                    .unwrap()
                    .canonical_name = format!("candidate-{n}");
                document
            })
            .collect();
        let payloads: Vec<_> = documents
            .iter()
            .map(|d| envelope(d, fixture.context()))
            .collect();
        for bytes in &payloads {
            assert_eq!(
                raw.put(StorageClass::Cache, bytes, Timestamp::EPOCH)
                    .unwrap()
                    .storage_class,
                StorageClass::Cache
            );
        }
        drop(raw);
        let barrier = Arc::new(Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = documents
                .into_iter()
                .map(|document| {
                    let barrier = Arc::clone(&barrier);
                    let fixture = &fixture;
                    let path = directory.path();
                    scope.spawn(move || {
                        let runtime = backend.open(path, fixture);
                        barrier.wait();
                        runtime.seed(document)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            results.iter().filter(|r| r.is_ok()).count(),
            1,
            "{backend:?}: {results:?}"
        );
        let raw = backend.raw(directory.path(), &fixture);
        for (result, bytes) in results.iter().zip(&payloads) {
            // Requesting the already-held Cache class cannot raise retention; it returns metadata.
            let observed = raw
                .put(StorageClass::Cache, bytes, Timestamp::EPOCH)
                .unwrap();
            if result.is_ok() {
                assert_eq!(observed.storage_class, StorageClass::Canonical);
            } else {
                assert_eq!(
                    result,
                    &Err(ekr_kernel::SeedError::Store(
                        ekr_store::StoreError::AlreadySeeded
                    ))
                );
                assert_eq!(
                    observed.storage_class,
                    StorageClass::Cache,
                    "losing seed changed retention on {backend:?}"
                );
            }
        }
    }
}

#[test]
fn persisted_seed_format_and_context_fields_refuse_before_admission() {
    let fixture = Fixture::new();
    for backend in [Backend::Sqlite, Backend::File] {
        for fault in [
            "envelope-version",
            "input-version",
            "context-field",
            "envelope-field",
        ] {
            let mut body: serde_json::Value =
                serde_json::from_slice(&envelope(&fixture.document(), fixture.context())).unwrap();
            let code = match fault {
                "envelope-version" => {
                    body["format"] = "ekr-seed-envelope/999".into();
                    "unsupported-seed-envelope"
                }
                "input-version" => {
                    body["input"]["format"] = "ekr-seed/999".into();
                    "unsupported-seed-format"
                }
                "context-field" => {
                    body["context"]["grant_acceptance"] = true.into();
                    "unknown field"
                }
                "envelope-field" => {
                    body["grant_acceptance"] = true.into();
                    "unknown field"
                }
                _ => unreachable!(),
            };
            let bytes = serde_json::to_vec(&body).unwrap();
            let directory = TempDir::new().unwrap();
            let raw = backend.raw(directory.path(), &fixture);
            let object = raw
                .put(ekr_store::StorageClass::Canonical, &bytes, Timestamp::EPOCH)
                .unwrap();
            let _appended = raw
                .append(&ekr_graph::RevisionEvent::Seeded {
                    revision_id: ekr_core::RevisionId::mint(),
                    seed_hash: object.content_hash,
                })
                .unwrap();
            let runtime = backend.open(directory.path(), &fixture);
            let error = runtime.snapshot().unwrap_err().to_string();
            assert!(error.contains(code), "{backend:?}/{fault}: {error}");
            assert_eq!(raw.get(&object.content_hash).unwrap(), Some(bytes));
        }
    }
}

#[test]
fn inherited_record_properties_remain_typed_and_constraints_cannot_disappear() {
    use ekr_ontology::{PropertyDefinition, ValueType};
    let mut fixture = Fixture::new();
    let property = ekr_core::PropertyId::mint();
    let parent = TypeId::mint();
    let mut definition = PropertyDefinition::new(
        property,
        "record",
        ValueType::Record(
            [("validation".to_owned(), ValueType::String)]
                .into_iter()
                .collect(),
        ),
    );
    definition.required = true;
    let mut parent_type = NodeType::new(parent, "Supported");
    parent_type.abstract_type = true;
    parent_type.properties.insert(property, definition);
    fixture.ontology.node_types[0].parents = BTreeSet::from([parent]);
    fixture.ontology.node_types.push(parent_type);
    fixture
        .graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .properties
        .insert(
            property,
            Value::Record(
                [(
                    "validation".to_owned(),
                    Value::String("a user key".to_owned()),
                )]
                .into_iter()
                .collect(),
            ),
        );
    for backend in [Backend::Sqlite, Backend::File] {
        let directory = TempDir::new().unwrap();
        let runtime = backend.open(directory.path(), &fixture);
        let document =
            SeedDocument::from_yaml(&serde_yaml_ng::to_string(&fixture.document()).unwrap())
                .unwrap();
        runtime.seed(document).unwrap();
        drop(runtime);
        assert_eq!(
            backend
                .open(directory.path(), &fixture)
                .snapshot()
                .unwrap()
                .nodes
                .len(),
            1
        );
        let mut constrained = fixture.document();
        constrained.ontology.node_types[1]
            .properties
            .get_mut(&property)
            .unwrap()
            .constraints
            .push("unsupported".to_owned());
        let directory = TempDir::new().unwrap();
        let error = backend
            .open(directory.path(), &fixture)
            .seed(constrained)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("unsupported-constraint"),
            "{backend:?}: {error}"
        );
    }
}
