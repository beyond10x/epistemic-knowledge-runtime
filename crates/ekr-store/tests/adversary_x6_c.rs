//! Adversary pass on `story:commit-cost-flat-with-store-size` (wave extract-06, unit C).
//!
//! The unit lets a store handle serve a verified `Provenance` object (an evidence payload) from
//! its memo until the tenant log shows a new event on that object's stream. Eventlog's redaction
//! (`EventStore::redact`, the mechanism the roadmap names for deletion requests) erases an event's
//! body in place: it keeps the event's id, version and log position, so it appends nothing the
//! log feed would show.
//!
//! Before the unit, a handle read the stream of every held object below `Canonical` again on
//! every history load, and `StreamRead::absorb` refuses a redacted event there, so a handle that
//! already held a redacted evidence payload refused its next load as a fresh handle does. These
//! cases hold a long-lived handle to that: once an evidence event is redacted, the held handle's
//! next history load does not serve the payload — on SQLite it refuses exactly as a fresh handle
//! does; on the file provider it refuses the rewritten journal as a diverged history, which the
//! held handle did before the unit too.
//!
//! Every case runs on one provider; the store is written through `ekr_store` and redacted
//! through the provider. Nothing in this runtime calls `redact` yet: roadmap D1 and P6 route
//! deletion requests through it.

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::TransactionId;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, ObjectStore, Publication,
    PublicationObject, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use eventlog_core::{EventStore, StreamId, TenantId};
use eventlog_file::FileEventStore;
use eventlog_sqlite::SqliteEventStore;
use tempfile::TempDir;

const TENANT: &str = "ekr";

/// Requires the one extra object it names with every history, as the kernel requires an evidence
/// payload, and admits nothing.
struct Requires(Option<ContentHash>);

impl CommitAuthority for Requires {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(self.0.into_iter().collect())
    }

    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn object_stream(hash: ContentHash) -> StreamId {
    StreamId::new(
        TenantId::new(TENANT).unwrap(),
        "ekr.store.object",
        hash.to_hex(),
    )
    .unwrap()
}

fn payload(label: &str) -> Vec<u8> {
    format!("adversary x6 c {label} {} ", EventId::mint())
        .repeat(4)
        .into_bytes()
}

fn occurrence(
    payload: RevisionPayload,
    expected_version: u64,
    record: Vec<u8>,
    extra: Option<Vec<u8>>,
) -> Publication {
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    let mut objects = std::collections::BTreeMap::from([(record_hash, canonical(record))]);
    if let Some(bytes) = extra {
        objects.insert(ContentHash::of_bytes(&bytes), canonical(bytes));
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

/// Seeds the store, stores `evidence` as a `Provenance` object and publishes one proposal.
fn seeded_with_evidence<S: RevisionLog + ObjectStore + Initialize>(store: &S, evidence: &[u8]) {
    let (record, document) = (payload("seed record"), payload("seed document"));
    let seed = occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        0,
        record,
        Some(document),
    );
    assert_eq!(store.initialize(&seed).unwrap(), Appended::Written);
    let stored = store
        .put(StorageClass::Provenance, evidence, Timestamp::EPOCH)
        .unwrap();
    assert_eq!(stored.storage_class, StorageClass::Provenance);
    let proposal = occurrence(
        RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        1,
        payload("record 1"),
        None,
    );
    assert_eq!(store.publish(&proposal).unwrap(), Appended::Written);
}

/// What the held handle's next load answered, and what a fresh handle's answered: whether the
/// evidence payload is in the history, or the refusal.
type Answers = (Result<bool, String>, Result<bool, String>);

/// A handle holds the evidence payload through two history loads; then its stream's only event
/// is redacted. Returns the held handle's next load and a fresh handle's load of the same store.
fn held_handle_after_redaction<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn(Option<ContentHash>) -> S,
    redact: impl FnOnce(&StreamId),
) -> Answers {
    let evidence = payload("redacted evidence");
    let address = ContentHash::of_bytes(&evidence);
    seeded_with_evidence(&open(None), &evidence);

    let reader = open(Some(address));
    let first = reader.history().unwrap();
    assert_eq!(
        first.objects[&address].metadata.storage_class,
        StorageClass::Provenance
    );
    reader.history().unwrap();

    redact(&object_stream(address));

    let fresh = open(Some(address))
        .history()
        .map(|history| history.objects.contains_key(&address))
        .map_err(|error| error.to_string());
    let held = reader
        .history()
        .map(|history| history.objects.contains_key(&address))
        .map_err(|error| error.to_string());
    assert!(
        fresh.is_err(),
        "control: a fresh handle loaded a history whose evidence event is redacted: {fresh:?}"
    );
    (held, fresh)
}

/// SQLite redaction is an `UPDATE` of the event row: no new log position, and the connection the
/// held handle uses sees the redacted row at once. Before the unit the held handle read the
/// evidence stream again on this load and refused it, `stream-envelope-disagrees`, as the fresh
/// handle does.
#[test]
#[ignore = "task:held-bytes-notice-deleted-blobs: an in-place redaction of a held non-canonical object's stream event moves no feed position"]
fn a_held_evidence_payload_whose_event_is_redacted_is_refused_as_a_fresh_handle_refuses_it_on_sqlite(
) {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("sqlite");
    std::fs::create_dir_all(&root).unwrap();
    let db = root.join("state.db");
    let open = |required| {
        SqliteStore::sqlite(&db, TENANT, None)
            .expect("the SQLite provider opens")
            .under(Requires(required))
    };
    let (held, fresh) = held_handle_after_redaction(open, |stream| {
        let rt = runtime();
        let provider = rt
            .block_on(SqliteEventStore::open(db.to_str().unwrap(), "ekr"))
            .unwrap();
        rt.block_on(provider.redact(stream, 1, "adversary deletion request"))
            .unwrap();
    });
    assert_eq!(
        held, fresh,
        "sqlite: the handle that held the evidence answered {held:?} after its stream event was \
         redacted; a fresh handle answers {fresh:?}"
    );
}

/// File redaction rewrites the journal's frames in place, so a handle that observed the journal
/// before refuses it as a diverged history, and `ekr session` then opens the store again and
/// answers as the fresh handle does. The held handle must not serve the payload either way.
#[test]
fn a_held_evidence_payload_whose_event_is_redacted_is_not_served_on_file() {
    let directory = TempDir::new().unwrap();
    let root = directory.path().join("file").join("state");
    let open = |required| {
        FileStore::file(&root, TENANT, None)
            .expect("the file provider opens")
            .under(Requires(required))
    };
    let (held, fresh) = held_handle_after_redaction(open, |stream| {
        let rt = runtime();
        let provider = rt.block_on(FileEventStore::open(Path::new(&root))).unwrap();
        rt.block_on(provider.redact(stream, 1, "adversary deletion request"))
            .unwrap();
    });
    assert!(
        held.is_err(),
        "file: the handle that held the evidence answered {held:?} after its stream event was \
         redacted; a fresh handle answers {fresh:?}"
    );
}
