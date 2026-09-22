//! Whether "two byte-identical events cannot both be in the log" is safe for this vocabulary.
//!
//! Correction round 1 derived the append idempotency key from the event's own content, which fixed
//! the duplicate-append defect and bought a consequence the implementor stated in `eventlog.rs`:
//!
//! > two byte-identical events cannot both be in the log, because the store cannot tell a second
//! > one from a retry of the first. That is right for this vocabulary rather than a limitation of
//! > it — every `RevisionEvent` variant carries a minted `RevisionId` or names a `TransactionId`,
//! > and each of the six says something that happens once for that id.
//!
//! Naming a `TransactionId` is not the same as happening once for it. Two of the six payloads
//! carry **no minted id and no discriminating field**: `TransactionRejected { transaction_id,
//! issues }` and `TransactionStale { transaction_id, validated_against, current }`.
//!
//! # What was decided
//!
//! Design § 89 closed `task:two-revision-events-have-no-discriminator` at the envelope rather than
//! in the payloads. Every occurrence is an `ekr.revision-event/2` carrying an `EventId` allocated
//! once for the occurrence, and "a new proposal, validation, refusal or stale decision receives a
//! new occurrence identity even when its payload repeats earlier content. A retry uses the same
//! identity and exact payload." So the store recognises a retry by identity, and two refusals with
//! byte-identical payloads are two occurrences the fold is handed.
//!
//! What the fold then *does* with a commit after a refusal is not the store's to say any more:
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels` moved that into the kernel's
//! replay authority, and `crates/ekr-kernel/tests/durable_commands.rs`'s
//! `retained_terminal_states_exact_retries_and_absent_targets_survive_reopen` holds that a
//! rejected transaction cannot commit. [`Witness`] here admits no semantics; it records what the
//! store handed it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EventId, EvidenceId, GraphRootId, NodeId,
    RevisionId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, Edge, Evidence, GraphRoot, Node, RevisionEvent, RevisionPayload,
    Root, Space,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, GraphDocument,
    Initialize, Publication, PublicationObject, RetainedHistory, RevisionLog, SqliteStore,
    StorageClass, StoreError,
};
use tempfile::TempDir;

/// The two validation results this file's case writes, in the order it writes them.
const RESULTS: [&[u8]; 2] = [b"a first validation result", b"a second validation result"];

/// A store over a fresh SQLite database, typed by `ontology`, replaying through `witness`.
fn store(directory: &TempDir, ontology: &Ontology, witness: Witness) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(witness)
}

/// A rejection repeating an earlier one's payload is a new occurrence, not a swallowed retry.
///
/// The lineage the caller publishes is ordinary and every payload of it is one the vocabulary
/// admits: a transaction is proposed, validated, **rejected**, proposed again with different
/// operations, validated again, and **rejected again with the same issue count**. The second
/// rejection's payload is byte-identical to the first's — `TransactionRejected` carries only the
/// transaction and a count.
///
/// This case first pinned the defect: the store read the second rejection as a retry, answered
/// `AlreadyRecorded`, never wrote it, and the fold never saw the refusal that was appended to it.
/// Design § 89 decided the other way, and this is that decision asserted: the second rejection is
/// `Written`, the fold is handed both, and only the same occurrence offered again is a retry.
#[test]
fn a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let witness = Witness::default();
    let store = store(&directory, &ontology, witness.clone());

    let document = GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed_hash = ContentHash::of_bytes(&document);
    assert_eq!(
        store
            .initialize(&occurrence(
                RevisionPayload::Seeded {
                    revision_id: RevisionId::mint(),
                    seed_hash,
                },
                0,
                Some((seed_hash, document)),
            ))
            .expect("seeded"),
        Appended::Written,
    );

    let transaction = TransactionId::mint();
    let proposer = AgentId::mint();
    let rejected = RevisionPayload::TransactionRejected {
        transaction_id: transaction,
        issues: 3,
    };
    let propose = |operations: &[u8]| RevisionPayload::TransactionProposed {
        transaction_id: transaction,
        proposer,
        operations_hash: Some(ContentHash::of_bytes(operations)),
    };
    let validate = |result: &[u8]| RevisionPayload::TransactionValidated {
        transaction_id: transaction,
        against: RevisionNumber::SEED,
        validation_hash: ContentHash::of_bytes(result),
    };

    // The first attempt: proposed, validated, refused; then a second attempt, with different
    // operations and a different validation result. Each is a new fact and each is written.
    let mut version = 1;
    for (payload, what) in [
        (propose(b"the first operations"), "proposed"),
        (validate(RESULTS[0]), "validated"),
        (rejected.clone(), "rejected"),
        (propose(b"the second operations"), "proposed again"),
        (validate(RESULTS[1]), "validated again"),
    ] {
        assert_eq!(
            store
                .publish(&occurrence(payload, version, None))
                .expect(what),
            Appended::Written,
            "the {what} occurrence is a new fact"
        );
        version += 1;
    }

    // And refused again, for the same number of issues: the same payload, a new occurrence.
    let again = occurrence(rejected.clone(), version, None);
    assert_eq!(
        store.publish(&again).expect("rejected again"),
        Appended::Written,
        "the second rejection repeats the first's payload and is still a new occurrence"
    );
    // The same occurrence offered again is the retry, and the store says so.
    assert_eq!(
        store.publish(&again).expect("the retry is answered"),
        Appended::AlreadyRecorded,
        "the second rejection, retried, is recognised by its identity"
    );

    let history = store.history().expect("the lineage reads");
    let rejections: Vec<&RevisionEvent> = history
        .occurrences
        .iter()
        .map(|occurrence| &occurrence.event)
        .filter(|event| event.payload == rejected)
        .collect();
    assert_eq!(
        rejections.len(),
        2,
        "the log holds both rejections, with byte-identical payloads"
    );
    assert_ne!(
        rejections[0].event_id, rejections[1].event_id,
        "and two occurrence identities"
    );

    store.fold().expect("the lineage folds");
    assert_eq!(
        *witness.rejections.lock().unwrap(),
        rejections
            .iter()
            .map(|event| event.event_id)
            .collect::<Vec<_>>(),
        "the fold was handed both refusals, in the order they were published"
    );
}

/// Records which rejections the store handed it on its latest replay, and admits no semantics.
///
/// Reads every record and the seed through [`RetainedHistory::content`] at `Canonical`, decodes
/// the seed document the lineage names, and answers the seed state: this file's lineage commits
/// nothing.
#[derive(Clone, Default)]
struct Witness {
    rejections: Arc<Mutex<Vec<EventId>>>,
}

impl CommitAuthority for Witness {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        let Some((first, later)) = history.occurrences.split_first() else {
            return Ok(None);
        };
        let RevisionPayload::Seeded {
            revision_id,
            seed_hash,
        } = first.event.payload
        else {
            return Err(StoreError::NotSeeded);
        };
        let mut rejections = Vec::new();
        for occurrence in later {
            history.content(occurrence.event.record_hash, StorageClass::Canonical)?;
            match occurrence.event.payload {
                RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
                RevisionPayload::TransactionRejected { .. } => {
                    rejections.push(occurrence.event.event_id);
                }
                _ => {}
            }
        }
        *self.rejections.lock().unwrap() = rejections;
        let ontology =
            ontology.ok_or_else(|| StoreError::Document("no ontology was supplied".into()))?;
        history.content(first.event.record_hash, StorageClass::Canonical)?;
        let graph = decode(
            history.content(seed_hash, StorageClass::Canonical)?,
            ontology,
        )?;
        Ok(Some(AdmittedRevision {
            root: Root {
                revision: RevisionNumber::SEED,
                parent: None,
                ontology_root: seed_hash,
                knowledge_root: knowledge_root(&graph),
                evidence_root: evidence_root(&graph),
                agent_root: seed_hash,
                transaction: first.event.record_hash,
            },
            graph,
            revision_id,
            event_id: first.event.event_id,
            record_hash: first.event.record_hash,
            committed_at: Timestamp::EPOCH,
        }))
    }
}

/// The graph a seed document carries, read with serde and admitted by nothing.
fn decode(bytes: &[u8], ontology: &Ontology) -> Result<CanonicalGraph, StoreError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        format: String,
        graph: Fields,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        root: GraphRoot,
        revision: RevisionNumber,
        nodes: BTreeMap<NodeId, Node>,
        edges: BTreeMap<EdgeId, Edge>,
        assertions: BTreeMap<AssertionId, Assertion>,
        evidence: BTreeMap<EvidenceId, Evidence>,
    }
    let Envelope { format, graph } =
        serde_json::from_slice(bytes).map_err(|error| StoreError::Document(error.to_string()))?;
    if format != "ekr.graph-document/2" {
        return Err(StoreError::Document(format!(
            "unsupported document {format}"
        )));
    }
    Ok(CanonicalGraph {
        root: graph.root,
        revision: graph.revision,
        ontology: ontology.clone(),
        nodes: graph.nodes,
        edges: graph.edges,
        assertions: graph.assertions,
        evidence: graph.evidence,
    })
}

/// One occurrence of `payload` at stream position `expected_version`, with a record of its own and
/// any `extra` object published beside it.
fn occurrence(
    payload: RevisionPayload,
    expected_version: u64,
    extra: Option<(ContentHash, Vec<u8>)>,
) -> Publication {
    let event_id = EventId::mint();
    let record = format!("provider-mechanics {} record {event_id}", payload.name()).into_bytes();
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    let mut objects = BTreeMap::from([(record_hash, canonical(record))]);
    if let Some((hash, bytes)) = extra {
        objects.insert(hash, canonical(bytes));
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id,
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

/// An ontology with no types: the store type-checks nothing (`AGENTS.md` invariant 7).
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with one node: this case is about the occurrences after it, not its content.
fn seed_graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let node = Node::new(NodeId::mint(), root_id, TypeId::mint(), "revision-lineage");
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
        nodes: BTreeMap::from([(node.id, node)]),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}
