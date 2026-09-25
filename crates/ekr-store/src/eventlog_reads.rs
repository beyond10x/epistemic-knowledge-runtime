//! What one history load costs the provider, counted at the provider boundary.
//!
//! `task:store-reads-rehash-the-whole-log`: on the file provider every provider call is one
//! transaction, and each transaction re-reads and re-hashes the whole committed log. A load that
//! reads each object stream and each blob with its own call therefore costs the log's size once
//! per object, and the objects grow with the revisions. These cases put a counter between the
//! store and the file provider and assert that loading a history costs the same, small number of
//! provider calls whatever the history's length: one for the revision stream, one batch for the
//! objects the occurrences name, and one batch for the objects the authority then requires.
//!
//! They live beside the store because the counting provider is injected through the store's
//! private constructor; no public surface exists to hand the store another provider, and none is
//! added for a test.
use super::preparation::native_expected;
use super::*;
use crate::PublicationObject;
use ekr_core::{AgentId, EventId, RevisionId};
use eventlog_core::{
    AppendResult, BoxFuture, CatchUpProgress, Claim, ClaimedCommand, FeedPage, Guard,
    HeadSetDigest, ProjectionPage, ProjectionSpec, Projector, Read, ReadResult, Snapshot,
    SnapshotGeneration, StreamSlice,
};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// The file provider, with every call into it counted.
struct Counting {
    inner: FileEventStore,
    calls: Arc<AtomicUsize>,
}
impl Counting {
    fn tick(&self) -> &FileEventStore {
        self.calls.fetch_add(1, Ordering::SeqCst);
        &self.inner
    }
}
impl EventStore for Counting {
    fn append<'a>(
        &'a self,
        stream: &'a StreamId,
        expected: Expected,
        events: &'a [NewEvent],
        meta: &'a CommandMeta,
    ) -> BoxFuture<'a, Result<AppendResult, EventLogError>> {
        self.tick().append(stream, expected, events, meta)
    }
    fn recorded_claim<'a>(
        &'a self,
        tenant: &'a TenantId,
        claim: &'a Claim,
    ) -> BoxFuture<'a, Result<Option<ClaimedCommand>, EventLogError>> {
        self.tick().recorded_claim(tenant, claim)
    }
    fn recorded_command<'a>(
        &'a self,
        stream: &'a StreamId,
        idempotency_key: &'a str,
        request_hash: &'a str,
    ) -> BoxFuture<'a, Result<Option<AppendResult>, EventLogError>> {
        self.tick()
            .recorded_command(stream, idempotency_key, request_hash)
    }
    fn read_stream<'a>(
        &'a self,
        stream: &'a StreamId,
        after_version: u64,
        limit: usize,
    ) -> BoxFuture<'a, Result<StreamSlice, EventLogError>> {
        self.tick().read_stream(stream, after_version, limit)
    }
    fn stream_version<'a>(
        &'a self,
        stream: &'a StreamId,
    ) -> BoxFuture<'a, Result<Option<u64>, EventLogError>> {
        self.tick().stream_version(stream)
    }
    fn read_feed<'a>(
        &'a self,
        tenant: &'a TenantId,
        after_position: u64,
        limit: usize,
    ) -> BoxFuture<'a, Result<FeedPage, EventLogError>> {
        self.tick().read_feed(tenant, after_position, limit)
    }
    fn redact<'a>(
        &'a self,
        stream: &'a StreamId,
        version: u64,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<RecordedEvent, EventLogError>> {
        self.tick().redact(stream, version, reason)
    }
    fn save_snapshot<'a>(
        &'a self,
        stream: &'a StreamId,
        snapshot: &'a Snapshot,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.tick().save_snapshot(stream, snapshot)
    }
    fn snapshot_generation<'a>(
        &'a self,
        stream: &'a StreamId,
    ) -> BoxFuture<'a, Result<Option<SnapshotGeneration>, EventLogError>> {
        self.tick().snapshot_generation(stream)
    }
    fn save_snapshot_checked<'a>(
        &'a self,
        stream: &'a StreamId,
        snapshot: &'a Snapshot,
        generation: &'a SnapshotGeneration,
    ) -> BoxFuture<'a, Result<bool, EventLogError>> {
        self.tick()
            .save_snapshot_checked(stream, snapshot, generation)
    }
    fn load_snapshot<'a>(
        &'a self,
        stream: &'a StreamId,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, EventLogError>> {
        self.tick().load_snapshot(stream)
    }
    fn forget_tenant<'a>(
        &'a self,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.tick().forget_tenant(tenant)
    }
    fn append_guarded<'a>(
        &'a self,
        stream: &'a StreamId,
        expected: Expected,
        events: &'a [NewEvent],
        meta: &'a CommandMeta,
        guard: Arc<dyn Guard>,
    ) -> BoxFuture<'a, Result<AppendResult, EventLogError>> {
        self.tick()
            .append_guarded(stream, expected, events, meta, guard)
    }
    fn create_projections(
        &self,
        projector: Arc<dyn Projector>,
    ) -> BoxFuture<'_, Result<(), EventLogError>> {
        self.tick().create_projections(projector)
    }
    fn register_inline(
        &self,
        projector: Arc<dyn Projector>,
    ) -> BoxFuture<'_, Result<(), EventLogError>> {
        self.tick().register_inline(projector)
    }
    fn is_inline<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        self.tick().is_inline(name)
    }
    fn run_catch_up<'a>(
        &'a self,
        projector: Arc<dyn Projector>,
        tenant: &'a TenantId,
        batch: usize,
    ) -> BoxFuture<'a, Result<CatchUpProgress, EventLogError>> {
        self.tick().run_catch_up(projector, tenant, batch)
    }
    fn rebuild_projection<'a>(
        &'a self,
        projector: Arc<dyn Projector>,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<u64, EventLogError>> {
        self.tick().rebuild_projection(projector, tenant)
    }
    fn projection_get<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Value>, EventLogError>> {
        self.tick().projection_get(projection, tenant, key)
    }
    fn projection_find<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        field: &'a str,
        value: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Value>, EventLogError>> {
        self.tick()
            .projection_find(projection, tenant, field, value, limit)
    }
    fn projection_list<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        after_key: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<(String, Value)>, EventLogError>> {
        self.tick()
            .projection_list(projection, tenant, after_key, limit)
    }
    fn projection_page<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        prefix: Option<&'a str>,
        cursor: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<ProjectionPage, EventLogError>> {
        self.tick()
            .projection_page(projection, tenant, prefix, cursor, limit)
    }
    fn stream_identity<'a>(
        &'a self,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<String, EventLogError>> {
        self.tick().stream_identity(tenant)
    }
    fn put_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
        bytes: &'a [u8],
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.tick().put_blob(tenant, digest, bytes)
    }
    fn get_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<Option<Vec<u8>>, EventLogError>> {
        self.tick().get_blob(tenant, digest)
    }
    fn delete_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.tick().delete_blob(tenant, digest)
    }
    fn read_many<'a>(
        &'a self,
        reads: &'a [Read],
    ) -> BoxFuture<'a, Result<Vec<ReadResult>, EventLogError>> {
        self.tick().read_many(reads)
    }
}

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

fn canonical(bytes: &[u8]) -> (ContentHash, PublicationObject) {
    (
        ContentHash::of_bytes(bytes),
        PublicationObject {
            storage_class: StorageClass::Canonical,
            stored_at: Timestamp::EPOCH,
            bytes: bytes.to_vec(),
        },
    )
}

fn publication(event_id: u64, payload: RevisionPayload, record: &[u8], at: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(event_id),
            record_hash: ContentHash::of_bytes(record),
            payload,
        },
        objects: BTreeMap::from([canonical(record)]),
        expected_version: at,
    }
}

/// Appends an occurrence and its objects as `publish` does once admission has passed.
///
/// No admitting authority is involved: a crate's `src/` other than `ekr-kernel`'s may not implement
/// one (AGENTS.md invariant 1, held by `crates/ekr/tests/story_contract.rs`), and a history load's
/// cost does not depend on who admitted what it reads.
fn append(store: &FileStore, publication: &Publication) {
    let version = publication.expected_version;
    let mut appends = vec![StreamAppend {
        stream: store.revision_stream().unwrap(),
        expected: if version == 0 {
            Expected::NoStream
        } else {
            Expected::Exact(version)
        },
        events: vec![NewEvent::new(
            publication.event.name(),
            2,
            serde_json::to_value(&publication.event).unwrap(),
        )
        .unwrap()],
    }];
    for (hash, object) in &publication.objects {
        if let Some(object) = store.object_append(*hash, object).unwrap() {
            appends.push(object);
        }
    }
    let request = BlobAppendGroup {
        group: AppendGroup {
            tenant: store.tenant.clone(),
            appends,
            meta: envelope(
                &format!("read-count.{}", publication.event.event_id),
                ContentHash::of(&publication.event).to_hex(),
            ),
        },
        blobs: publication
            .objects
            .iter()
            .map(|(hash, object)| BlobWrite {
                digest: hash.to_hex(),
                bytes: object.bytes.clone(),
            })
            .collect(),
    };
    store.atomic(&request).unwrap();
}

const EXTRAS: u64 = 5;

/// A file store holding a seed and `proposals` further occurrences, each with its own record, and
/// `EXTRAS` objects no occurrence names. Returns the hashes of those objects, which stand in for
/// the evidence and documents the kernel's authority requires.
fn written(path: &Path, proposals: u64) -> BTreeSet<ContentHash> {
    let store = FileStore::file(path, "ekr", None).unwrap();
    let mut extras = BTreeSet::new();
    for n in 0..EXTRAS {
        let stored = store
            .put(
                StorageClass::Canonical,
                format!("required object {n}").as_bytes(),
                Timestamp::EPOCH,
            )
            .unwrap();
        extras.insert(stored.content_hash);
    }
    let mut seed = publication(
        0x40,
        RevisionPayload::Seeded {
            revision_id: id::<RevisionId>(0x31),
            seed_hash: ContentHash::of_bytes(b"read count seed envelope"),
        },
        b"read count seed record",
        0,
    );
    seed.objects
        .extend([canonical(b"read count seed envelope")]);
    append(&store, &seed);
    for n in 0..proposals {
        let record = format!("read count proposal record {n}");
        append(
            &store,
            &publication(
                0x100 + n,
                RevisionPayload::TransactionProposed {
                    transaction_id: id(0x1000 + n),
                    proposer: id::<AgentId>(0x03),
                    operations_hash: None,
                },
                record.as_bytes(),
                n + 1,
            ),
        );
    }
    extras
}

/// The same directory reopened behind the counter.
fn counted(path: &Path) -> (EventlogStore<Counting>, Arc<AtomicUsize>) {
    let runtime = new_runtime().unwrap();
    let inner = runtime.block_on(FileEventStore::open(path)).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = EventlogStore::assemble(
        runtime,
        Counting {
            inner,
            calls: Arc::clone(&calls),
        },
        "ekr",
        None,
    )
    .unwrap();
    (store, calls)
}

/// Provider calls one full history load makes, and what it loaded.
fn load_cost(proposals: u64) -> (usize, RetainedHistory) {
    let directory = tempfile::tempdir().unwrap();
    let extras = written(directory.path(), proposals);
    let (store, calls) = counted(directory.path());
    calls.store(0, Ordering::SeqCst);
    let history = store
        .load_history_requiring(MAX_READ_LIMIT, None, |_| Ok(extras.clone()))
        .unwrap();
    (calls.load(Ordering::SeqCst), history)
}

#[test]
fn a_history_load_costs_three_provider_calls_whatever_its_length() {
    let mut costs = Vec::new();
    for proposals in [2, 24] {
        let (calls, history) = load_cost(proposals);
        assert_eq!(
            history.occurrences.len() as u64,
            proposals + 1,
            "the load dropped an occurrence"
        );
        // Every proposal's record, the seed's record and envelope, and every required extra.
        assert_eq!(
            history.objects.len() as u64,
            proposals + 2 + EXTRAS,
            "the load dropped an object"
        );
        costs.push((proposals, calls));
    }
    assert!(
        costs.iter().all(|&(_, calls)| calls == 3),
        "a history load is one revision-stream read, one batch for the objects the occurrences \
         name and one for the objects the authority requires; (proposals, provider calls) \
         measured {costs:?}"
    );
}

/// The batched path runs the checks the single-object path runs: an object whose blob is missing
/// is refused with the same code on both, never loaded as present.
#[test]
fn a_missing_blob_is_refused_alike_by_the_single_and_the_batched_path() {
    let directory = tempfile::tempdir().unwrap();
    let extras = written(directory.path(), 1);
    let (store, _) = counted(directory.path());
    let victim = *extras.iter().next().unwrap();
    store
        .runtime()
        .block_on(
            store
                .store
                .inner
                .delete_blob(&store.tenant, &victim.to_hex()),
        )
        .unwrap();
    let single = store.object(victim).unwrap_err().to_string();
    let batched = store
        .load_history_requiring(MAX_READ_LIMIT, None, |_| Ok(extras.clone()))
        .unwrap_err()
        .to_string();
    assert!(
        single.contains("object-integrity: native blob missing"),
        "single-object path: {single}"
    );
    assert_eq!(single, batched, "the two paths refused differently");
}

/// Eventlog 0.4.0 adds a merge expectation for forked streams. A publication preparation can
/// record none of them, so capturing one is refused as a store error, not mapped onto another kind.
#[test]
fn a_merge_expectation_is_refused_by_the_preparation_capture() {
    let refused = native_expected(Expected::Merge(HeadSetDigest::of(&["head-a", "head-b"])));
    assert!(
        matches!(
            &refused,
            Err(StoreError::Document(code)) if code == "preparation-merge-expectation"
        ),
        "{refused:?}"
    );
    assert_eq!(
        native_expected(Expected::Exact(3)).unwrap(),
        NativeExpected {
            kind: NativeExpectedKind::Exact,
            version: Some(3),
        }
    );
}
