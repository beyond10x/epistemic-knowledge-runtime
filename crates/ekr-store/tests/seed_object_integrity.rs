//! Corrupt address, length, bytes, retention or shape of a seed object is refused, and all but
//! retention before the authority is asked. Provider integrity coverage; semantic admission
//! remains the real kernel's suite.
//!
//! Design § 89 and § 91.6 changed the record these faults are written into. The original wrote
//! `ekr.store.ObjectStored` schema 1, whose body carried the bytes inline; a new object is now
//! schema 2 metadata — `content_hash`, `storage_class`, `byte_len`, `stored_at` — with its bytes
//! behind a native provider blob binding, and the schema 1 inline body is frozen for verification
//! and migration, refused as new ingestion. So each fault below is written into the current shape,
//! straight through the provider as a corrupted store would hold it, and the original's own
//! shape is the `legacy` fault.
//!
//! Self-contained: [`Mechanics`] stands in for the kernel's replay authority for provider
//! mechanics only, and counts how often the store asks it anything.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EventId, EvidenceId, GraphRootId, NodeId,
    PropertyId, RevisionId, RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate, RevisionEvent,
    RevisionPayload, Root, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, CommitAuthority, FileStore, GraphDocument,
    RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
use tempfile::TempDir;

/// The tenant the provider is written under and the store is opened on.
const TENANT: &str = "ekr";

/// Each fault, the text its refusal names, and whether the authority is asked before it.
///
/// Retention is the one the authority reaches: the object is intact, and it is the authority's
/// read of the seed at `Canonical` through [`RetainedHistory::content`] — the kernel's own read —
/// that the store answers with a refusal.
const FAULTS: [(&str, &str, bool); 8] = [
    ("control", "", true),
    ("hash", "object-integrity", false),
    ("length", "object-integrity", false),
    ("bytes", "object-integrity", false),
    ("missing", "object-integrity: native blob missing", false),
    ("retention", "required-object-integrity", true),
    ("unknown", "unknown field", false),
    ("legacy", "unsupported-object-envelope", false),
];

fn metadata(key: &str) -> CommandMeta {
    CommandMeta {
        idempotency_key: key.to_owned(),
        request_hash: key.to_owned(),
        subject: "ekr.test".to_owned(),
        actor: "ekr.test".to_owned(),
        request_id: key.to_owned(),
        trace_id: key.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}

/// One `ekr.store.ObjectStored` record as the provider holds it, and the blob bound to it if any.
struct Written {
    hash: ContentHash,
    schema: u32,
    body: serde_json::Value,
    blob: Option<Vec<u8>>,
}

impl Written {
    /// The current, intact record of `bytes` under `class`.
    fn intact(bytes: &[u8], class: &str) -> Self {
        let hash = ContentHash::of_bytes(bytes);
        Self {
            hash,
            schema: 2,
            body: serde_json::json!({
                "content_hash": hash, "storage_class": class, "byte_len": bytes.len(),
                "stored_at": Timestamp::EPOCH,
            }),
            blob: Some(bytes.to_vec()),
        }
    }
}

/// Writes `objects` and then `seeded` into a provider the store has never opened.
async fn install<S: EventStore>(provider: &S, objects: &[Written], seeded: &RevisionEvent) {
    let tenant = TenantId::new(TENANT).unwrap();
    for (index, object) in objects.iter().enumerate() {
        if let Some(blob) = &object.blob {
            provider
                .put_blob(&tenant, &object.hash.to_hex(), blob)
                .await
                .unwrap();
        }
        let stream =
            StreamId::new(tenant.clone(), "ekr.store.object", object.hash.to_hex()).unwrap();
        let event =
            NewEvent::new("ekr.store.ObjectStored", object.schema, object.body.clone()).unwrap();
        provider
            .append(
                &stream,
                Expected::NoStream,
                &[event],
                &metadata(&format!("integrity-probe-object-{index}")),
            )
            .await
            .unwrap();
    }
    let stream = StreamId::new(tenant, "ekr.revision", "canonical").unwrap();
    let event = NewEvent::new(seeded.name(), 2, serde_json::to_value(seeded).unwrap()).unwrap();
    provider
        .append(
            &stream,
            Expected::NoStream,
            &[event],
            &metadata("integrity-probe-revision"),
        )
        .await
        .unwrap();
}

fn run(file: bool) {
    for (fault, expected, reaches_authority) in FAULTS {
        let directory = TempDir::new().unwrap();
        let ontology = ontology();
        let graph = seed_graph(&ontology);
        let bytes = GraphDocument::of(&graph).to_bytes().unwrap();
        let mut seed = Written::intact(&bytes, "Canonical");
        match fault {
            "control" => {}
            "hash" => {
                seed.body["content_hash"] =
                    serde_json::to_value(ContentHash::of_bytes(b"other")).unwrap();
            }
            "length" => seed.body["byte_len"] = 1.into(),
            "bytes" => {
                // Same length, so the address and not the length is what disagrees.
                let mut altered = bytes.clone();
                altered[0] ^= 1;
                seed.blob = Some(altered);
            }
            "missing" => seed.blob = None,
            "retention" => seed.body["storage_class"] = "Cache".into(),
            "unknown" => seed.body["unexpected_rule"] = true.into(),
            "legacy" => {
                seed.schema = 1;
                seed.body["bytes"] = serde_json::json!(bytes);
                seed.blob = None;
            }
            _ => unreachable!(),
        }
        let record = Written::intact(b"provider-mechanics seed record", "Canonical");
        let seeded = RevisionEvent {
            format: RevisionEvent::FORMAT.to_owned(),
            event_id: EventId::mint(),
            record_hash: record.hash,
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: seed.hash,
            },
        };
        let objects = [record, seed];
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let store: Box<dyn RevisionLog> = if file {
            let raw = runtime
                .block_on(eventlog_file::FileEventStore::open(directory.path()))
                .unwrap();
            runtime.block_on(install(&raw, &objects, &seeded));
            drop(raw);
            Box::new(
                FileStore::file(directory.path(), TENANT, ontology)
                    .unwrap()
                    .under(Mechanics(calls.clone())),
            )
        } else {
            let path = directory.path().join("state.db");
            let raw = runtime
                .block_on(eventlog_sqlite::SqliteEventStore::open(
                    path.to_str().unwrap(),
                    "ekr",
                ))
                .unwrap();
            runtime.block_on(install(&raw, &objects, &seeded));
            drop(raw);
            Box::new(
                SqliteStore::sqlite(&path, TENANT, ontology)
                    .unwrap()
                    .under(Mechanics(calls.clone())),
            )
        };
        if fault == "control" {
            assert_eq!(store.fold().unwrap(), graph);
        } else {
            let error = store.fold().unwrap_err().to_string();
            assert!(error.contains(expected), "{fault}: {error}");
        }
        assert_eq!(
            calls.load(Ordering::SeqCst) > 0,
            reaches_authority,
            "{fault}: the authority was asked {} time(s)",
            calls.load(Ordering::SeqCst)
        );
    }
}

#[test]
fn sqlite_seed_objects_verify_address_length_and_retention_before_admission() {
    run(false);
}
#[test]
fn file_seed_objects_verify_address_length_and_retention_before_admission() {
    run(true);
}

/// Replays a seed-only lineage for provider mechanics, admits no semantics, and counts every
/// callback the store makes.
///
/// Reads the seed's record and document through [`RetainedHistory::content`] at `Canonical`, as
/// the kernel does, and decodes the document the seed names.
struct Mechanics(Arc<AtomicUsize>);

impl CommitAuthority for Mechanics {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
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
        if !later.is_empty() {
            return Err(StoreError::Document("a seed-only lineage".into()));
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

/// An ontology with no types: the store type-checks nothing (`AGENTS.md` invariant 7).
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with two nodes, an edge between them, an assertion and the evidence it rests on, so the
/// control's `fold() == graph` compares content rather than two empty maps.
fn seed_graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let property = PropertyId::mint();
    let (subject, object) = (NodeId::mint(), NodeId::mint());
    let evidence_id = EvidenceId::mint();

    let mut observed = Node::new(subject, root_id, type_id, "revision-lineage");
    observed
        .properties
        .insert(property, vec![CanonicalValue::Decimal("1.0".to_owned())]);
    let reached = Node::new(object, root_id, type_id, "revision-lineage-target");
    let holds = Edge::new(
        EdgeId::mint(),
        root_id,
        type_id,
        CanonicalRef::new(subject),
        CanonicalRef::new(object),
    );
    let evidence = Evidence {
        id: evidence_id,
        source: EvidenceSource::Document {
            document_id: "docs/roadmap.md".to_owned(),
            section: Some("P1".to_owned()),
        },
        content_hash: ContentHash::of_bytes(b"the roadmap's P1 exit criterion"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id,
        subject: Subject::Node(CanonicalRef::new(subject)),
        predicate: Predicate::Relation(type_id),
        object: Object::Node(CanonicalRef::new(object)),
        evidence: BTreeSet::from([evidence_id]),
        proposed_by: AgentId::mint(),
        assessment: Assessment::Accepted {
            validators: BTreeSet::new(),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::EPOCH),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

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
        nodes: BTreeMap::from([(subject, observed), (object, reached)]),
        edges: BTreeMap::from([(holds.id, holds)]),
        assertions: BTreeMap::from([(assertion.id, assertion)]),
        evidence: BTreeMap::from([(evidence_id, evidence)]),
    }
}
