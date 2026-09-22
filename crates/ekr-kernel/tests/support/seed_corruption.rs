//! Deliberate native-provider corruption setup only. Every tested read uses real kernel authority.
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload, Root};
use ekr_ontology::Ontology;
use ekr_store::{Appended, FileStore, ObjectStore, SqliteStore, StorageClass};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};

pub fn inject(path: &std::path::Path, file: bool, ontology: Ontology, bytes: &[u8]) -> Appended {
    let hash = ContentHash::of_bytes(bytes);
    let zero = ContentHash::from_bytes([0; 32]);
    let root = Root {
        revision: RevisionNumber::SEED,
        parent: None,
        ontology_root: zero,
        knowledge_root: zero,
        evidence_root: zero,
        agent_root: zero,
        transaction: hash,
    };
    let record = ekr_kernel::SeedResultV1 {
        format: ekr_kernel::SeedResultV1::FORMAT.into(),
        event_id: EventId::mint(),
        revision_id: RevisionId::mint(),
        seed_hash: hash,
        authority_root: zero,
        committed_at: Timestamp::EPOCH,
        result: root,
        result_hash: ContentHash::of(&root),
    };
    let record_bytes = record.to_bytes().unwrap();
    let raw: Box<dyn ObjectStore> = if file {
        Box::new(FileStore::file(path, "ekr", ontology).unwrap())
    } else {
        Box::new(SqliteStore::sqlite(&path.join("state.db"), "ekr", ontology).unwrap())
    };
    let record_hash = raw
        .put(StorageClass::Canonical, &record_bytes, Timestamp::EPOCH)
        .unwrap()
        .content_hash;
    raw.put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    drop(raw);
    let event = RevisionEvent {
        format: RevisionEvent::FORMAT.into(),
        event_id: record.event_id,
        record_hash,
        payload: RevisionPayload::Seeded {
            revision_id: record.revision_id,
            seed_hash: hash,
        },
    };
    let native = NewEvent::new(event.name(), 2, serde_json::to_value(&event).unwrap()).unwrap();
    let stream = StreamId::new(TenantId::new("ekr").unwrap(), "ekr.revision", "canonical").unwrap();
    let key = record.event_id.to_string();
    let meta = CommandMeta {
        idempotency_key: key.clone(),
        request_hash: key.clone(),
        subject: "corruption-fixture".into(),
        actor: "corruption-fixture".into(),
        request_id: key.clone(),
        trace_id: key,
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider
                .append(&stream, Expected::NoStream, &[native], &meta)
                .await
                .unwrap()
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                path.join("state.db").to_str().unwrap(),
                "ekr",
            )
            .await
            .unwrap();
            provider
                .append(&stream, Expected::NoStream, &[native], &meta)
                .await
                .unwrap()
        }
    });
    assert!(!result.deduplicated);
    Appended::Written
}
