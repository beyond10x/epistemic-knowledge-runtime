//! Attacks the synchronous boundary with populated stores and actual entered handles.
//! Substitute authority here measures callback reachability, not kernel seed validation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
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
    RevisionPayload, Root, Space,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, EventlogStore,
    FileStore, GraphDocument, Initialize, NativeBlobWrite, NativeClaim, NativeExpected,
    NativeExpectedKind, NativeNewEvent, NativePublicationRequest, NativeStreamAppend,
    NativeStreamId, ObjectStore, Publication, PublicationCommandKey, PublicationCommandKind,
    PublicationObject, PublicationPreparationV1, RecordedOccurrence, RetainedHistory, RevisionLog,
    SqliteStore, StorageClass, StoreError,
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

/// Two nodes and the edge between them, so the seed document is not an empty map.
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
                Node::new(subject, root_id, type_id, "populated-boundary"),
            ),
            (
                object,
                Node::new(object, root_id, type_id, "populated-boundary-target"),
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

fn proposal(transaction: TransactionId, record: &[u8], expected_version: u64) -> Publication {
    publication(
        RevisionPayload::TransactionProposed {
            transaction_id: transaction,
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        &[record],
        expected_version,
    )
}

fn refused<T>(result: Result<T, StoreError>) {
    assert!(
        matches!(result, Err(StoreError::RuntimeContext)),
        "every synchronous I/O path must refuse before provider or authority work"
    );
}

fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn entered_handle_constructor_refusal_preserves_existing_provider_bytes_and_missing_parents() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("existing.db");
    let file_path = directory.path().join("existing-files");
    let bytes = b"constructor boundary retained object";
    let hash = ContentHash::of_bytes(bytes);
    SqliteStore::sqlite(&sqlite_path, "constructor-probe", ontology())
        .unwrap()
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    FileStore::file(&file_path, "constructor-probe", ontology())
        .unwrap()
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    let before = bytes_under(directory.path());
    let ambient = runtime();
    let handle = ambient.handle().clone();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let _entered = handle.enter();
                for tenant in ["constructor-probe", "invalid\ntenant"] {
                    refused(SqliteStore::sqlite(&sqlite_path, tenant, ontology()));
                    refused(FileStore::file(&file_path, tenant, ontology()));
                    refused(SqliteStore::sqlite(
                        &directory.path().join("missing/child/store.db"),
                        tenant,
                        ontology(),
                    ));
                    refused(FileStore::file(
                        &directory.path().join("missing/child/files"),
                        tenant,
                        ontology(),
                    ));
                }
                assert_eq!(bytes_under(directory.path()), before);
            })
            .join()
            .unwrap();
    });
    assert!(!directory.path().join("missing").exists());
    assert_eq!(
        SqliteStore::sqlite(&sqlite_path, "constructor-probe", ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
    assert_eq!(
        FileStore::file(&file_path, "constructor-probe", ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
}

/// Counts both callbacks and admits any structurally ordered lineage: the seed is revision zero
/// and each `RevisionCommitted` is the revision it names. A stand-in for reachability only.
struct AuthorityCalls {
    discovery: Arc<AtomicUsize>,
    replay: Arc<AtomicUsize>,
    graph: CanonicalGraph,
}

impl CommitAuthority for AuthorityCalls {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        self.discovery.fetch_add(1, Ordering::SeqCst);
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.replay.fetch_add(1, Ordering::SeqCst);
        let mut head = None;
        for occurrence in &history.occurrences {
            let (number, revision_id) = match occurrence.event.payload {
                RevisionPayload::Seeded { revision_id, .. } => (RevisionNumber::SEED, revision_id),
                RevisionPayload::RevisionCommitted {
                    number,
                    revision_id,
                    ..
                } => (number, revision_id),
                _ => continue,
            };
            let mut graph = self.graph.clone();
            graph.revision = number;
            head = Some(AdmittedRevision {
                root: Root {
                    revision: number,
                    parent: None,
                    ontology_root: ContentHash::of_bytes(b"substitute ontology root"),
                    knowledge_root: knowledge_root(&graph),
                    evidence_root: evidence_root(&graph),
                    agent_root: ContentHash::of_bytes(b"substitute agent root"),
                    transaction: occurrence.event.record_hash,
                },
                graph,
                revision_id,
                event_id: occurrence.event.event_id,
                record_hash: occurrence.event.record_hash,
                committed_at: Timestamp::EPOCH,
            });
            if revision == Some(number) {
                return Ok(head);
            }
        }
        match revision {
            Some(_) => Err(StoreError::Document(
                "substitute-revision-not-reached".into(),
            )),
            None => Ok(head),
        }
    }
}

/// The elected native request (design § 94, ADR 0009) is the decision it was prepared from: one
/// atomic group, no single-stream claim, a conditional append at the decision's expected version,
/// and exactly the decision's objects as blobs, bytes unchanged.
fn native_request_matches_its_decision(
    prepared: &PublicationPreparationV1,
    decision: &Publication,
) {
    let request: &NativePublicationRequest = &prepared.native_request;
    let claim: &Option<NativeClaim> = &request.meta.claim;
    assert!(
        claim.is_none(),
        "an atomic group carries no single-stream claim"
    );
    let conditional: Vec<&NativeStreamAppend> = request
        .appends
        .iter()
        .filter(|append| {
            let expected: &NativeExpected = &append.expected;
            expected.kind == NativeExpectedKind::Exact
                && expected.version == Some(decision.expected_version)
        })
        .collect();
    assert_eq!(
        conditional.len(),
        1,
        "exactly one append is conditional on the decision's expected revision-stream version: {:?}",
        request.appends
    );
    let stream: &NativeStreamId = &conditional[0].stream;
    assert_eq!(stream.tenant, request.tenant);
    let events: &Vec<NativeNewEvent> = &conditional[0].events;
    assert_eq!(events.len(), 1, "one occurrence per decision");
    let blobs: BTreeMap<String, Vec<u8>> = request
        .blobs
        .iter()
        .map(|blob: &NativeBlobWrite| (blob.digest.clone(), blob.bytes.clone()))
        .collect();
    let staged: BTreeMap<String, Vec<u8>> = decision
        .objects
        .iter()
        .map(|(hash, object)| (hash.to_hex(), object.bytes.clone()))
        .collect();
    assert_eq!(
        blobs, staged,
        "the blobs are exactly the decision's objects"
    );
}

fn populated_boundary<S: AtomicBlobEventStore>(store: EventlogStore<S>, root: &Path) {
    let discovery_calls = Arc::new(AtomicUsize::new(0));
    let replay_calls = Arc::new(AtomicUsize::new(0));
    let graph = graph(&ontology());
    let store = store.under(AuthorityCalls {
        discovery: discovery_calls.clone(),
        replay: replay_calls.clone(),
        graph: graph.clone(),
    });
    let seed_bytes = GraphDocument::of(&graph).to_bytes().unwrap();
    let seed = publication(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&seed_bytes),
        },
        &[b"populated seed record", &seed_bytes],
        0,
    );
    let transaction = TransactionId::mint();
    assert_eq!(store.initialize(&seed).unwrap(), Appended::Written);
    for decision in [
        proposal(transaction, b"populated proposal record", 1),
        publication(
            RevisionPayload::TransactionValidated {
                transaction_id: transaction,
                against: RevisionNumber::SEED,
                validation_hash: ContentHash::of_bytes(b"populated validation material"),
            },
            &[b"populated validation record"],
            2,
        ),
        publication(
            RevisionPayload::RevisionCommitted {
                transaction_id: transaction,
                revision_id: RevisionId::mint(),
                number: RevisionNumber::new(1),
                knowledge_root: knowledge_root(&graph),
            },
            &[b"populated commit record"],
            3,
        ),
    ] {
        assert_eq!(store.publish(&decision).unwrap(), Appended::Written);
    }
    let prior = store.head().unwrap().unwrap();
    assert_eq!(prior.revision, RevisionNumber::new(1));
    assert!(discovery_calls.load(Ordering::SeqCst) > 0);
    assert!(replay_calls.load(Ordering::SeqCst) > 0);
    let object = store
        .put(StorageClass::Ephemeral, b"retained", Timestamp::EPOCH)
        .unwrap();
    // A pending, unpublished preparation: the attempt must neither re-elect nor resume it.
    let pending_transaction = TransactionId::mint();
    let key = PublicationCommandKey {
        kind: PublicationCommandKind::Propose,
        transaction_id: Some(pending_transaction),
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    let input_hash = ContentHash::of_bytes(b"populated pending command input");
    let pending = proposal(pending_transaction, b"populated pending record", 4);
    let prepared = store.prepare(&key, input_hash, &pending, None).unwrap();
    native_request_matches_its_decision(&prepared, &pending);
    let occurrences: Vec<RecordedOccurrence> = store.history().unwrap().occurrences;
    assert_eq!(
        occurrences.iter().map(|o| o.version).collect::<Vec<_>>(),
        vec![1, 2, 3, 4],
        "the retained revision stream is the four published occurrences, in order"
    );
    let next = proposal(TransactionId::mint(), b"populated next record", 5);
    let discovery_before = discovery_calls.load(Ordering::SeqCst);
    let replay_before = replay_calls.load(Ordering::SeqCst);
    let before = bytes_under(root);
    let malformed = proposal(TransactionId::mint(), b"not a seed occurrence", 0);
    let attempt = || {
        // Dispatch through public trait objects as well as the inherent graph archival method.
        let objects: &dyn ObjectStore = &store;
        let revisions: &dyn RevisionLog = &store;
        let initialization: &dyn Initialize = &store;
        refused(objects.get(&object.content_hash));
        refused(objects.get(&ContentHash::of_bytes(b"absent")));
        refused(objects.put(StorageClass::Canonical, b"retained", Timestamp::EPOCH));
        refused(objects.put(StorageClass::Canonical, b"new", Timestamp::EPOCH));
        refused(revisions.preparation(&key));
        refused(revisions.prepare(&key, input_hash, &pending, None));
        refused(revisions.prepare(&key, input_hash, &pending, Some(&prepared)));
        refused(revisions.resume(&prepared));
        refused(revisions.history());
        refused(revisions.history_at(RevisionNumber::SEED));
        refused(revisions.history_at(RevisionNumber::new(u64::MAX)));
        refused(revisions.seed_bytes());
        refused(revisions.head());
        refused(revisions.fold());
        refused(revisions.replay(RevisionNumber::SEED));
        refused(revisions.replay(RevisionNumber::new(u64::MAX)));
        refused(revisions.publish(&next));
        refused(initialization.initialize(&seed));
        refused(initialization.initialize(&malformed));
        refused(store.store_graph(&graph, Timestamp::EPOCH));
        assert_eq!(discovery_calls.load(Ordering::SeqCst), discovery_before);
        assert_eq!(replay_calls.load(Ordering::SeqCst), replay_before);
        assert_eq!(bytes_under(root), before);
    };
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        attempt();
    }
    ambient.block_on(async { attempt() });
    // No rejected call consumes an occurrence identity, elects an attempt or promotes retention.
    assert_eq!(
        store
            .put(StorageClass::Ephemeral, b"retained", Timestamp::EPOCH)
            .unwrap(),
        object
    );
    assert_eq!(store.preparation(&key).unwrap(), Some(prepared.clone()));
    assert_eq!(store.resume(&prepared).unwrap(), Appended::Written);
    assert_eq!(store.publish(&next).unwrap(), Appended::Written);
    assert_eq!(store.publish(&next).unwrap(), Appended::AlreadyRecorded);
    assert_eq!(store.head().unwrap(), Some(prior));
    assert_eq!(store.get(&ContentHash::of_bytes(b"new")).unwrap(), None);
    assert_eq!(store.seed_bytes().unwrap(), Some(seed_bytes));
}

#[test]
fn populated_file_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes() {
    let directory = TempDir::new().unwrap();
    populated_boundary(
        FileStore::file(directory.path(), "populated-file", ontology()).unwrap(),
        directory.path(),
    );
}

#[test]
fn populated_sqlite_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes() {
    let directory = TempDir::new().unwrap();
    populated_boundary(
        SqliteStore::sqlite(
            &directory.path().join("store.db"),
            "populated-sqlite",
            ontology(),
        )
        .unwrap(),
        directory.path(),
    );
}

fn unwind_after_refusal<S: AtomicBlobEventStore>(store: EventlogStore<S>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let owned_store = store;
        refused(owned_store.head());
        panic!("synthetic caller unwind after named refusal");
    }));
    let panic = result.expect_err("the deliberate caller panic must be observed");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"synthetic caller unwind after named refusal")
    );
}

#[test]
fn store_drop_during_caller_unwind_in_entered_handle_keeps_completed_data_reopenable() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("store.db");
    let file_path = directory.path().join("files");
    let sqlite = SqliteStore::sqlite(&sqlite_path, "drop-probe", ontology()).unwrap();
    let file = FileStore::file(&file_path, "drop-probe", ontology()).unwrap();
    let bytes = b"completed before caller unwind";
    let hash = ContentHash::of_bytes(bytes);
    sqlite
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    file.put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        unwind_after_refusal(sqlite);
        unwind_after_refusal(file);
    }
    assert_eq!(
        SqliteStore::sqlite(&sqlite_path, "drop-probe", ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
    assert_eq!(
        FileStore::file(&file_path, "drop-probe", ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
}

#[test]
fn plain_worker_thread_can_open_and_write_while_another_thread_runs_tokio() {
    let directory = TempDir::new().unwrap();
    runtime().block_on(async {
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    assert!(tokio::runtime::Handle::try_current().is_err());
                    let sqlite = SqliteStore::sqlite(
                        &directory.path().join("worker.db"),
                        "worker",
                        ontology(),
                    )
                    .unwrap();
                    let file = FileStore::file(
                        &directory.path().join("worker-files"),
                        "worker",
                        ontology(),
                    )
                    .unwrap();
                    let bytes = b"ordinary synchronous worker";
                    let sqlite_object = sqlite
                        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
                        .unwrap();
                    let file_object = file
                        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
                        .unwrap();
                    assert_eq!(
                        sqlite.get(&sqlite_object.content_hash).unwrap(),
                        Some(bytes.to_vec())
                    );
                    assert_eq!(
                        file.get(&file_object.content_hash).unwrap(),
                        Some(bytes.to_vec())
                    );
                })
                .join()
                .unwrap();
        });
    });
}
