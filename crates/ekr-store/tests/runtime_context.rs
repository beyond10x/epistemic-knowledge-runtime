//! Synchronous store calls refuse an entered runtime before invoking persistence.
//!
//! Self-contained: the substitute authority here only counts callbacks and refuses everything, so
//! a case can tell "refused before the authority was asked" from "the authority said no". It is
//! not kernel seed validation, which `crates/ekr-kernel/tests/seed.rs` owns.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ekr_core::{
    AgentId, ContentHash, EdgeId, EventId, GraphRootId, NodeId, PropertyId, RevisionId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, CanonicalValue, Edge, GraphRoot, Node, RevisionEvent,
    RevisionPayload, Space,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    AdmittedRevision, CommitAuthority, EventlogStore, FileStore, GraphDocument, Initialize,
    NativeCommandMeta, NativePublicationRequest, ObjectStore, Publication, PublicationCommandKey,
    PublicationCommandKind, PublicationObject, PublicationPreparationV1, RetainedHistory,
    RevisionLog, SqliteStore, StorageClass, StoreError,
};
use eventlog_core::AtomicBlobEventStore;
use tempfile::TempDir;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// An ontology with no declarations: the store type-checks nothing (invariant 7).
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// Two nodes and the edge between them, so the archived document is not an empty map.
fn graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let (subject, object) = (NodeId::mint(), NodeId::mint());
    let mut holds = Edge::new(
        EdgeId::mint(),
        root_id,
        type_id,
        CanonicalRef::new(subject),
        CanonicalRef::new(object),
    );
    holds.properties.insert(
        PropertyId::mint(),
        vec![CanonicalValue::Enum("canonical".to_owned())],
    );
    CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: [
            (
                subject,
                Node::new(subject, root_id, type_id, "runtime-boundary"),
            ),
            (
                object,
                Node::new(object, root_id, type_id, "runtime-boundary-target"),
            ),
        ]
        .into_iter()
        .collect(),
        edges: [(holds.id, holds)].into_iter().collect(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

/// One occurrence publishing `objects`, the first of which is its retained record.
fn publication(payload: RevisionPayload, objects: &[&[u8]], expected_version: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.to_owned(),
            event_id: EventId::mint(),
            record_hash: ContentHash::of_bytes(objects[0]),
            payload,
        },
        objects: objects
            .iter()
            .map(|bytes| {
                (
                    ContentHash::of_bytes(bytes),
                    PublicationObject {
                        storage_class: StorageClass::Canonical,
                        stored_at: Timestamp::EPOCH,
                        bytes: bytes.to_vec(),
                    },
                )
            })
            .collect(),
        expected_version,
    }
}

/// A well-formed preparation argument that no store ever elected.
fn never_elected(key: &PublicationCommandKey, decision: &Publication) -> PublicationPreparationV1 {
    PublicationPreparationV1 {
        format: PublicationPreparationV1::FORMAT.to_owned(),
        command_key: key.clone(),
        input_hash: ContentHash::of_bytes(b"runtime-boundary command input"),
        decision: decision.clone(),
        attempt_number: 0,
        previous_attempt_hash: None,
        native_request: NativePublicationRequest {
            tenant: "runtime-fixture".to_owned(),
            appends: Vec::new(),
            meta: NativeCommandMeta {
                idempotency_key: String::new(),
                request_hash: String::new(),
                subject: String::new(),
                actor: String::new(),
                request_id: String::new(),
                trace_id: String::new(),
                causation_id: None,
                causation_depth: 0,
                occurred_at_unix_nanos: "0".to_owned(),
                occurred_at_offset_seconds: 0,
                claim: None,
            },
            blobs: Vec::new(),
        },
        native_fingerprint: String::new(),
    }
}

fn refusal<T>(result: Result<T, StoreError>) {
    match result {
        Err(error) => {
            assert_eq!(error, StoreError::RuntimeContext);
            assert_eq!(
                error.to_string(),
                "synchronous store access requires a thread outside a Tokio runtime"
            );
        }
        Ok(_) => panic!("store accepted an ambient-runtime call"),
    }
}

/// The entry points a case drove into a refusal, by name, so the set can be held to the source.
#[derive(Default)]
struct Exercised(RefCell<BTreeSet<&'static str>>);

impl Exercised {
    fn refuses<T>(&self, entry: &'static str, result: Result<T, StoreError>) {
        refusal(result);
        self.0.borrow_mut().insert(entry);
    }
}

/// Every method of the three persistence ports and every inherent `pub fn` of the store, read off
/// `src/` so that a method added to the port is a method this suite must refuse or place.
fn declared_entry_points() -> BTreeSet<String> {
    let source = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let read = |file: &str| {
        std::fs::read_to_string(source.join(file))
            .unwrap_or_else(|error| panic!("reading src/{file}: {error}"))
    };
    let names = |text: &str, head: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.trim_start().strip_prefix(head))
            .map(|rest| {
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect()
    };
    let mut declared = BTreeSet::new();
    for (file, port) in [
        ("log.rs", "RevisionLog"),
        ("log.rs", "Initialize"),
        ("objects.rs", "ObjectStore"),
    ] {
        let text = read(file);
        let start = text
            .find(&format!("pub trait {port} {{"))
            .unwrap_or_else(|| panic!("src/{file} no longer declares {port}"));
        let body = &text[start..];
        let body = &body[..body.find("\n}").expect("the trait closes")];
        declared.extend(names(body, "fn "));
    }
    declared.extend(names(&read("eventlog.rs"), "pub fn "));
    declared
}

#[test]
fn entered_runtime_reads_refuse_and_the_store_remains_usable() {
    let directory = TempDir::new().unwrap();
    let sqlite = SqliteStore::sqlite(
        &directory.path().join("store.db"),
        "runtime-fixture",
        ontology(),
    )
    .unwrap();
    let file = FileStore::file(
        &directory.path().join("files"),
        "runtime-fixture",
        ontology(),
    )
    .unwrap();
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        refusal(sqlite.head());
        refusal(file.head());
    }
    assert_eq!(sqlite.head().unwrap(), None);
    assert_eq!(file.head().unwrap(), None);
}

#[test]
fn running_runtime_reads_return_a_refusal_instead_of_panicking() {
    let directory = TempDir::new().unwrap();
    let store = SqliteStore::sqlite(
        &directory.path().join("store.db"),
        "runtime-fixture",
        ontology(),
    )
    .unwrap();
    runtime().block_on(async { refusal(store.head()) });
    assert_eq!(store.head().unwrap(), None);
}

#[test]
fn constructors_refuse_before_creating_paths() {
    let directory = TempDir::new().unwrap();
    let check = || {
        let sqlite_path = directory.path().join("store.db");
        let file_path = directory.path().join("files");
        refusal(SqliteStore::sqlite(
            &sqlite_path,
            "runtime-fixture",
            ontology(),
        ));
        refusal(FileStore::file(&file_path, "runtime-fixture", ontology()));
        refusal(SqliteStore::sqlite_existing(
            &sqlite_path,
            "runtime-fixture",
            ontology(),
        ));
        refusal(FileStore::file_existing(
            &file_path,
            "runtime-fixture",
            ontology(),
        ));
        assert!(!sqlite_path.exists());
        assert!(!file_path.exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    };
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        check();
    }
    ambient.block_on(async { check() });
}

/// Counts every callback and refuses every one of them.
struct CountingAuthority(Arc<AtomicUsize>);

impl CommitAuthority for CountingAuthority {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::InvalidSeed("test authority refuses".to_owned()))
    }

    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::InvalidSeed("test authority refuses".to_owned()))
    }
}

fn all_io_refuses<S: AtomicBlobEventStore>(store: EventlogStore<S>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store.under(CountingAuthority(calls.clone()));
    let retained = b"completed before entering runtime";
    let pending = b"must not be persisted";
    let seed_record = b"seed record that must not be persisted";
    let proposal_record = b"proposal record that must not be persisted";
    let object = store
        .put(StorageClass::Ephemeral, retained, Timestamp::EPOCH)
        .unwrap();
    let graph = graph(&ontology());
    let graph_bytes = GraphDocument::of(&graph).to_bytes().unwrap();
    let seed = publication(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(pending),
        },
        &[seed_record, pending],
        0,
    );
    let transaction = TransactionId::mint();
    let proposal = publication(
        RevisionPayload::TransactionProposed {
            transaction_id: transaction,
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        &[proposal_record],
        0,
    );
    let key = PublicationCommandKey {
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    let unelected = never_elected(&key, &seed);
    let exercised = Exercised::default();
    runtime().block_on(async {
        exercised.refuses("preparation", store.preparation(&key));
        exercised.refuses(
            "prepare",
            store.prepare(&key, unelected.input_hash, &seed, None),
        );
        exercised.refuses("resume", store.resume(&unelected));
        exercised.refuses("history", store.history());
        exercised.refuses("history_at", store.history_at(RevisionNumber::SEED));
        exercised.refuses("seed_bytes", store.seed_bytes());
        exercised.refuses("head", store.head());
        exercised.refuses("fold", store.fold());
        exercised.refuses("replay", store.replay(RevisionNumber::SEED));
        exercised.refuses("publish", store.publish(&proposal));
        exercised.refuses("initialize", store.initialize(&seed));
        exercised.refuses("store_graph", store.store_graph(&graph, Timestamp::EPOCH));
        exercised.refuses("get", store.get(&object.content_hash));
        exercised.refuses("published_events", store.published_events());
        exercised.refuses(
            "write_checkpoint",
            store.write_checkpoint(
                1,
                ContentHash::of_bytes(b"a binding"),
                Some(b"a replay checkpoint"),
            ),
        );
        exercised.refuses(
            "put",
            store.put(StorageClass::Canonical, retained, Timestamp::EPOCH),
        );
        exercised.refuses(
            "put",
            store.put(StorageClass::Canonical, pending, Timestamp::EPOCH),
        );
    });
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(store.head().unwrap(), None);
    assert_eq!(store.seed_bytes().unwrap(), None);
    assert_eq!(store.history().unwrap(), RetainedHistory::default());
    assert_eq!(store.preparation(&key).unwrap(), None);
    assert_eq!(
        store.get(&object.content_hash).unwrap(),
        Some(retained.to_vec())
    );
    for absent in [&pending[..], seed_record, proposal_record, &graph_bytes] {
        assert_eq!(store.get(&ContentHash::of_bytes(absent)).unwrap(), None);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // A refused retention promotion must leave the previous class intact.
    assert_eq!(
        store
            .put(StorageClass::Ephemeral, retained, Timestamp::EPOCH)
            .unwrap(),
        object
    );
    // Returning to a synchronous context restores normal persistence.
    let written = store
        .put(StorageClass::Canonical, pending, Timestamp::EPOCH)
        .unwrap();
    assert_eq!(
        store.get(&written.content_hash).unwrap(),
        Some(pending.to_vec())
    );
    // The constructors are refused by `constructors_refuse_before_creating_paths`; `under` only
    // installs the authority and `set_full_replay` only sets a flag, and neither performs I/O.
    // Everything else must have been refused above.
    let mut reached: BTreeSet<String> = exercised
        .0
        .into_inner()
        .into_iter()
        .map(str::to_owned)
        .collect();
    reached.extend(
        [
            "sqlite",
            "sqlite_existing",
            "file",
            "file_existing",
            "under",
            "set_full_replay",
        ]
        .map(str::to_owned),
    );
    assert_eq!(
        reached,
        declared_entry_points(),
        "a store entry point is not driven into the runtime-context refusal, or is no longer declared"
    );
}

#[test]
fn every_file_operation_refuses_before_authority_or_persistence() {
    let directory = TempDir::new().unwrap();
    all_io_refuses(FileStore::file(directory.path(), "runtime-fixture", ontology()).unwrap());
}

#[test]
fn every_sqlite_operation_refuses_before_authority_or_persistence() {
    let directory = TempDir::new().unwrap();
    all_io_refuses(
        SqliteStore::sqlite(
            &directory.path().join("store.db"),
            "runtime-fixture",
            ontology(),
        )
        .unwrap(),
    );
}

#[test]
fn dropping_inside_a_running_runtime_preserves_completed_writes() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("store.db");
    let file_path = directory.path().join("files");
    let sqlite = SqliteStore::sqlite(&sqlite_path, "runtime-fixture", ontology()).unwrap();
    let file = FileStore::file(&file_path, "runtime-fixture", ontology()).unwrap();
    let bytes = b"completed write survives drop";
    let hash = ContentHash::of_bytes(bytes);
    sqlite
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    file.put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    runtime().block_on(async move {
        drop(sqlite);
        drop(file);
    });
    let sqlite = SqliteStore::sqlite(&sqlite_path, "runtime-fixture", ontology()).unwrap();
    let file = FileStore::file(&file_path, "runtime-fixture", ontology()).unwrap();
    assert_eq!(sqlite.get(&hash).unwrap(), Some(bytes.to_vec()));
    assert_eq!(file.get(&hash).unwrap(), Some(bytes.to_vec()));
}
