//! Two claims the store's own comments make, driven against the store.
//!
//! One is about the object store's retention answer when the same bytes arrive twice under two
//! classes; the other is about how a retried publication is recognised. The public
//! `RevisionLog::append` this file was written against is gone
//! (`architecture-decision-record:0007-the-commit-path-is-the-kernels`); a lineage now moves only
//! through [`Initialize::initialize`] and [`RevisionLog::publish`], and the retry case is written
//! against those.
//!
//! Self-contained: [`Mechanics`] stands in for the kernel's replay authority for provider
//! mechanics only and admits no semantics. `crates/ekr-kernel/tests/seed.rs` owns seed admission.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EventId, EvidenceId, GraphRootId, NodeId, RevisionId,
    RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, Edge, Evidence, GraphRoot, Node, RevisionEvent, RevisionPayload,
    Root, Space,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, GraphDocument,
    Initialize, ObjectStore, Publication, PublicationObject, RetainedHistory, RevisionLog,
    SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

/// A SQLite store over a fresh database inside `directory`, typed by `ontology`.
fn store(directory: &TempDir, ontology: &Ontology) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(Mechanics)
}

/// Content addressing merges two retention answers into the weaker one, silently.
///
/// `StorageClass` is what a reclamation sweep reads: `Cache` is "reproducible, and freely
/// deletable" and `Canonical` is "durable, revisioned, strongly governed". `ObjectStore::put`
/// keys on the bytes alone and answers a later write with the earlier record, so bytes first
/// stored as a cache entry stay freely deletable after `store_graph` has named them a lineage's
/// seed. `providers.rs`'s `a_stored_object_carries_the_class_it_was_written_under` writes two
/// different payloads under two classes and never the same payload twice, so it does not see this.
#[test]
fn storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let store = store(&directory, &ontology);
    let bytes = b"a payload that was cached before it was canonical";

    let cached = store
        .put(StorageClass::Cache, bytes, Timestamp::EPOCH)
        .expect("the cache write lands");
    assert_eq!(cached.storage_class, StorageClass::Cache);

    let canonical = store
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .expect("the canonical write is answered");

    assert_eq!(
        canonical.storage_class,
        StorageClass::Canonical,
        "bytes canonical state depends on are recorded as freely deletable, because a cache write \
         reached them first"
    );
}

/// A retried publication is recognised by its occurrence identity, so it writes once.
///
/// The removed `append` built its idempotency key from the stream position it claimed, read
/// *before* the append; a landed append moved that position, so a retry after a lost
/// acknowledgement computed a different key and the byte-identical event was written a second
/// time. Design § 89 is the rule the port now keeps: "use occurrence identity for idempotency" and
/// "never derive identity from content or the stream position about to be written". A retry is
/// the same immutable `Publication` — same `EventId`, same bytes — offered again.
///
/// Observed through the seed, because a doubled seed is the one duplicate the fold names:
/// [`Mechanics`] answers `SeedIsNotFirst` for a lineage holding two, as the store's fold did.
#[test]
fn retrying_an_append_writes_the_same_event_once_rather_than_twice() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = store(&directory, &ontology);

    let seed = seed(&graph);
    assert_eq!(
        store
            .initialize(&seed)
            .expect("the first publication lands"),
        Appended::Written,
        "a new fact, not a retry: the first publication lands"
    );
    // The retry, and the one publication in this suite that must *not* write. Correction round 2
    // made the port say which happened, so the retry is not merely harmless — it is legible.
    assert_eq!(
        store
            .initialize(&seed)
            .expect("the retry of that same publication is answered"),
        Appended::AlreadyRecorded,
        "a retried occurrence is recognised as the retry it is, and says so"
    );

    let folded = store.fold();
    assert!(
        folded.is_ok(),
        "one publication retried once produced two seeds: {folded:?}"
    );
    assert_eq!(
        store
            .history()
            .expect("the lineage reads")
            .occurrences
            .len(),
        1,
        "one publication retried once is one occurrence in the log"
    );
}

/// Replays a seed-only lineage for provider mechanics, and admits no semantics.
///
/// Reads the seed's record and document through [`RetainedHistory::content`] at `Canonical` and
/// decodes the document the seed names; a second seed anywhere after the first is refused.
struct Mechanics;

impl CommitAuthority for Mechanics {
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
        if later
            .iter()
            .any(|occurrence| matches!(occurrence.event.payload, RevisionPayload::Seeded { .. }))
        {
            return Err(StoreError::SeedIsNotFirst);
        }
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

/// The seed occurrence naming `graph`'s document, published with it and with a record of its own.
fn seed(graph: &CanonicalGraph) -> Publication {
    let document = GraphDocument::of(graph)
        .to_bytes()
        .expect("the document serialises");
    let seed_hash = ContentHash::of_bytes(&document);
    let event_id = EventId::mint();
    let record = format!("provider-mechanics seed record {event_id}").into_bytes();
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id,
            record_hash,
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash,
            },
        },
        objects: BTreeMap::from([
            (record_hash, canonical(record)),
            (seed_hash, canonical(document)),
        ]),
        expected_version: 0,
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

/// A seed with one node. What this file asks is how many occurrences the log holds, and a
/// one-node seed answers that as well as a rich one; `providers.rs` holds the content.
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
