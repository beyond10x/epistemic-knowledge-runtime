//! Native eventlog persistence. The kernel owns interpretation; this layer verifies physical
//! envelopes and bytes and atomically publishes conditional event/blob groups.
use crate::{
    AdmittedRevision, Appended, CommitAuthority, GraphDocument, Initialize, ObjectStore,
    Publication, RecordedOccurrence, RetainedHistory, RetainedObject, RevisionLog, StorageClass,
    StoreError, StoredObject,
};
use ekr_core::{ContentHash, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionEvent, RevisionPayload, Root};
use ekr_ontology::Ontology;
use eventlog_core::{
    AppendGroup, AtomicBlobEventStore, BlobAppendGroup, BlobWrite, CommandMeta, EventLogError,
    EventStore, Expected, NewEvent, Read, ReadResult, RecordedEvent, StreamAppend, StreamId,
    StreamSlice, TenantId, MAX_READ_LIMIT,
};
use eventlog_file::FileEventStore;
use eventlog_sqlite::SqliteEventStore;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use time::OffsetDateTime;
use tokio::runtime::{Builder, Handle, Runtime};

const REVISION_STREAM_TYPE: &str = "ekr.revision";
const REVISION_STREAM_ID: &str = "canonical";
const OBJECT_STREAM_TYPE: &str = "ekr.store.object";
const OBJECT_STORED: &str = "ekr.store.ObjectStored";
const OBJECT_RETENTION_RAISED: &str = "ekr.store.ObjectRetentionRaised";
const WRITER: &str = "ekr.store";
#[path = "preparation.rs"]
mod preparation;
pub use preparation::{
    NativeBlobWrite, NativeClaim, NativeCommandMeta, NativeExpected, NativeExpectedKind,
    NativeNewEvent, NativePublicationRequest, NativeStreamAppend, NativeStreamId,
    PublicationCommandKey, PublicationCommandKind, PublicationPreparationV1,
};
#[cfg(test)]
#[path = "eventlog_reads.rs"]
mod reads;

/// SQLite-backed synchronous runtime storage.
pub type SqliteStore = EventlogStore<SqliteEventStore>;
/// File-backed synchronous runtime storage.
pub type FileStore = EventlogStore<FileEventStore>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectMetadata {
    content_hash: ContentHash,
    storage_class: StorageClass,
    byte_len: u64,
    stored_at: Timestamp,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetentionRaised {
    content_hash: ContentHash,
    from: StorageClass,
    to: StorageClass,
}

/// One event the provider log published, as the provider recorded it.
///
/// Plain values only: holding one grants nothing, and no provider type crosses this boundary, so a
/// reader of the log never has to name the eventlog crates.
#[derive(Clone, Debug, PartialEq)]
pub struct PublishedEvent {
    /// Its place in the tenant's whole log; strictly ascending in [`EventlogStore::published_events`].
    pub position: u64,
    /// The stream it was appended to.
    pub stream_type: String,
    /// The stream instance it was appended to.
    pub stream_id: String,
    /// Its version within that stream.
    pub version: u64,
    /// The provider's identity for it.
    pub event_id: String,
    /// The event name, e.g. `ekr.store.ObjectStored`.
    pub name: String,
    /// The schema version it was written under.
    pub schema_version: u32,
    /// The payload exactly as logged.
    pub data: serde_json::Value,
}

/// Synchronous storage with a fallible injected kernel authority and an owned async runtime.
pub struct EventlogStore<S: EventStore> {
    runtime: Option<Runtime>,
    store: S,
    tenant: TenantId,
    ontology: Option<Ontology>,
    authority: Option<Box<dyn CommitAuthority>>,
}
impl EventlogStore<SqliteEventStore> {
    /// Opens the SQLite provider outside an entered async runtime.
    /// # Errors
    /// Runtime-context refusal, invalid tenant or provider failure.
    pub fn sqlite(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let path = path.to_string_lossy();
        let store =
            waiting_out_the_lock(|| runtime.block_on(SqliteEventStore::open(&path, "ekr")))?;
        Self::assemble(runtime, store, tenant, ontology.into())
    }
    /// Opens an already provisioned SQLite store, creating no database and no tables: a path
    /// holding none is refused and left as it was.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, a missing store or provider failure.
    pub fn sqlite_existing(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let path = path.to_string_lossy();
        let store = waiting_out_the_lock(|| {
            runtime.block_on(SqliteEventStore::open_existing(&path, "ekr"))
        })?;
        Self::assemble(runtime, store, tenant, ontology.into())
    }
}
/// How long a SQLite open waits for another connection's lock before reporting it.
///
/// `SqliteEventStore::open` runs `PRAGMA journal_mode=WAL`, and on a database another process is
/// converting at the same moment that statement returns `database is locked` at once, without
/// consulting the connection's 5 s busy handler (measured: 2 of 48 opens by six concurrent
/// processes on a new database). Every other statement in the open is `BEGIN IMMEDIATE` under
/// that handler. Opening is idempotent, so the open is retried until this bound.
const SQLITE_OPEN_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(10);
/// The provider's rendering of `SQLITE_BUSY` (`sqlite3_errstr`).
const SQLITE_BUSY: &str = "database is locked";
/// Retries `open` while it reports a held SQLite lock, for at most [`SQLITE_OPEN_LOCK_WAIT`].
fn waiting_out_the_lock<T>(
    mut open: impl FnMut() -> Result<T, EventLogError>,
) -> Result<T, EventLogError> {
    let deadline = std::time::Instant::now() + SQLITE_OPEN_LOCK_WAIT;
    let mut pause = std::time::Duration::from_millis(5);
    loop {
        match open() {
            Err(EventLogError::Backend(message))
                if message == SQLITE_BUSY && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(pause);
                pause = (pause * 2).min(std::time::Duration::from_millis(100));
            }
            result => return result,
        }
    }
}
impl EventlogStore<FileEventStore> {
    /// Opens the File provider outside an entered async runtime.
    /// # Errors
    /// Runtime-context refusal, invalid tenant or provider failure.
    pub fn file(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        std::fs::create_dir_all(path).map_err(|e| StoreError::Backend(e.to_string()))?;
        let store = runtime.block_on(FileEventStore::open(path))?;
        Self::assemble(runtime, store, tenant, ontology.into())
    }
    /// Opens an already provisioned File store, creating no directory, lock or manifest: a path
    /// holding none is refused and left as it was.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, a missing store or provider failure.
    pub fn file_existing(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let store = runtime.block_on(FileEventStore::open_existing(path))?;
        Self::assemble(runtime, store, tenant, ontology.into())
    }
}
fn ensure_sync_context() -> Result<(), StoreError> {
    if Handle::try_current().is_ok() {
        Err(StoreError::RuntimeContext)
    } else {
        Ok(())
    }
}
fn new_runtime() -> Result<Runtime, StoreError> {
    ensure_sync_context()?;
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| StoreError::Backend(e.to_string()))
}
impl<S: EventStore> Drop for EventlogStore<S> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            if Handle::try_current().is_ok() {
                runtime.shutdown_background();
            }
        }
    }
}
impl<S: EventStore> EventlogStore<S> {
    fn runtime(&self) -> &Runtime {
        self.runtime.as_ref().expect("runtime is owned until drop")
    }
    fn assemble(
        runtime: Runtime,
        store: S,
        tenant: &str,
        ontology: Option<Ontology>,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            runtime: Some(runtime),
            store,
            tenant: TenantId::new(tenant)?,
            ontology,
            authority: None,
        })
    }
    /// Installs the kernel's full replay authority. It receives immutable verified inputs.
    #[must_use]
    pub fn under(mut self, authority: impl CommitAuthority + 'static) -> Self {
        self.authority = Some(Box::new(authority));
        self
    }
    fn authority(&self) -> Result<&dyn CommitAuthority, StoreError> {
        self.authority.as_deref().ok_or(StoreError::NoSeedAuthority)
    }
    fn revision_stream(&self) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            REVISION_STREAM_TYPE,
            REVISION_STREAM_ID,
        )?)
    }
    fn object_stream(&self, hash: ContentHash) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            OBJECT_STREAM_TYPE,
            hash.to_hex(),
        )?)
    }
    /// Every event this tenant's provider log has published, in log order, across every stream.
    ///
    /// Read through this store's own open provider handle (`EventStore::read_feed`), so it opens
    /// no second connection and appends nothing. It interprets nothing either: a kernel revision,
    /// a stored object and a publication preparation come back exactly as the provider recorded
    /// them. A redacted event, another tenant's event, or a position that does not ascend is
    /// refused rather than skipped, because a log read that silently drops an entry reports less
    /// than was written.
    /// # Errors
    /// Runtime-context refusal, provider failure or a feed that disagrees with itself.
    pub fn published_events(&self) -> Result<Vec<PublishedEvent>, StoreError> {
        ensure_sync_context()?;
        let mut events: Vec<PublishedEvent> = Vec::new();
        let mut after = 0;
        loop {
            let page = self.runtime().block_on(self.store.read_feed(
                &self.tenant,
                after,
                MAX_READ_LIMIT,
            ))?;
            for event in page.events {
                if event.is_redacted()
                    || event.tenant != self.tenant
                    || events
                        .last()
                        .is_some_and(|last| last.position >= event.global_seq)
                {
                    return Err(StoreError::Document("feed-envelope-disagrees".into()));
                }
                events.push(PublishedEvent {
                    position: event.global_seq,
                    stream_type: event.stream_type,
                    stream_id: event.stream_id,
                    version: event.version,
                    event_id: event.event_id,
                    name: event.name,
                    schema_version: event.schema_version,
                    data: event.data,
                });
            }
            if !page.has_more {
                return Ok(events);
            }
            if page.next_position <= after {
                return Err(StoreError::Document("feed-cursor-stalled".into()));
            }
            after = page.next_position;
        }
    }
    fn read_all(&self, stream: &StreamId, limit: usize) -> Result<Vec<RecordedEvent>, StoreError> {
        self.read_until(stream, limit, |_| false)
    }
    fn read_until(
        &self,
        stream: &StreamId,
        limit: usize,
        stop: impl Fn(&RecordedEvent) -> bool,
    ) -> Result<Vec<RecordedEvent>, StoreError> {
        self.read_rest(StreamRead::new(stream), limit, stop)
    }
    /// Reads `stream` on from where `read` stopped, through the same checks as its first slice.
    fn read_rest(
        &self,
        mut read: StreamRead<'_>,
        limit: usize,
        stop: impl Fn(&RecordedEvent) -> bool,
    ) -> Result<Vec<RecordedEvent>, StoreError> {
        loop {
            if read.finished {
                return Ok(read.events);
            }
            let slice =
                self.runtime()
                    .block_on(self.store.read_stream(read.stream, read.after, limit))?;
            read.absorb(&self.tenant, slice, &stop)?;
        }
    }
    fn occurrences(
        &self,
        limit: usize,
        selected: Option<RevisionNumber>,
    ) -> Result<Vec<RecordedOccurrence>, StoreError> {
        let mut ids = BTreeSet::new();
        self.read_until(&self.revision_stream()?, limit, |record| {
            selected.is_some_and(|revision| {
                serde_json::from_value::<RevisionEvent>(record.data.clone()).is_ok_and(|event| {
                    match event.payload {
                        RevisionPayload::Seeded { .. } => revision == RevisionNumber::SEED,
                        RevisionPayload::RevisionCommitted { number, .. } => number == revision,
                        _ => false,
                    }
                })
            })
        })?
        .into_iter()
        .map(|recorded| {
            let event: RevisionEvent = serde_json::from_value(recorded.data)
                .map_err(|e| StoreError::Document(e.to_string()))?;
            if event.format != RevisionEvent::FORMAT
                || recorded.schema_version != 2
                || recorded.name != event.name()
                || !ids.insert(event.event_id)
            {
                return Err(StoreError::Document("revision-envelope-disagrees".into()));
            }
            Ok(RecordedOccurrence {
                version: recorded.version,
                provider_event_id: recorded.event_id,
                event,
            })
        })
        .collect()
    }
    fn object(&self, hash: ContentHash) -> Result<Option<RetainedObject>, StoreError> {
        Ok(self.object_versioned(hash)?.map(|(object, _)| object))
    }
    fn object_versioned(
        &self,
        hash: ContentHash,
    ) -> Result<Option<(RetainedObject, u64)>, StoreError> {
        let events = self.read_all(&self.object_stream(hash)?, MAX_READ_LIMIT)?;
        let Some(meta) = stored_metadata(&events)? else {
            return Ok(None);
        };
        let blob = self
            .runtime()
            .block_on(self.store.get_blob(&self.tenant, &hash.to_hex()))?;
        retained_object(hash, &events, meta, blob).map(Some)
    }
    /// Every object in `hashes`, checked in their order as [`Self::object_versioned`] checks one,
    /// through two provider batches: every object's stream, then the blobs of the objects before
    /// the first that fails a stream check. An absent object is refused `required-object-missing`
    /// in its place in that order: every object before it is checked in full first, and no blob at
    /// or after it is read. A stream longer than one page continues with ordinary reads, which
    /// only an object whose retention was raised more than a page's worth of times needs.
    ///
    /// Each object passes [`StreamRead::absorb`], [`stored_metadata`] and [`retained_object`], the
    /// functions [`Self::object_versioned`] applies to the same reads made separately, so a
    /// batched load refuses what the per-object load refused, and first the refusal it met first.
    /// Two orderings differ, both for a provider failure rather than a check of this crate: a
    /// failure of the stream batch as a whole comes before any blob check, where per-object reads
    /// would have met it at the failing stream; and a provider refusal of any blob in the blob
    /// batch (a damaged SQLite blob, for example) comes before the blob checks of earlier objects,
    /// because a provider without its own `read_many` fails the whole batch on its first error.
    fn required(&self, hashes: &[ContentHash]) -> Result<Vec<RetainedObject>, StoreError> {
        let streams = hashes
            .iter()
            .map(|hash| self.object_stream(*hash))
            .collect::<Result<Vec<_>, _>>()?;
        let reads: Vec<Read> = streams
            .iter()
            .map(|stream| Read::Stream {
                stream: stream.clone(),
                after_version: 0,
                limit: MAX_READ_LIMIT,
            })
            .collect();
        let slices = self.batch(&reads)?;
        let mut stored = Vec::with_capacity(hashes.len());
        let mut refusal = None;
        for ((hash, stream), slice) in hashes.iter().zip(&streams).zip(slices) {
            match self.stored_object(stream, slice) {
                Ok((events, meta)) => stored.push((*hash, events, meta)),
                Err(error) => {
                    refusal = Some(error);
                    break;
                }
            }
        }
        let mut objects = Vec::with_capacity(stored.len());
        if !stored.is_empty() {
            let reads: Vec<Read> = stored
                .iter()
                .map(|(hash, _, _)| Read::Blob {
                    tenant: self.tenant.clone(),
                    digest: hash.to_hex(),
                })
                .collect();
            for ((hash, events, meta), blob) in stored.into_iter().zip(self.batch(&reads)?) {
                let ReadResult::Blob(blob) = blob else {
                    return Err(StoreError::Document("batch-read-disagrees".into()));
                };
                objects.push(retained_object(hash, &events, meta, blob)?.0);
            }
        }
        match refusal {
            Some(error) => Err(error),
            None => Ok(objects),
        }
    }
    /// One provider batch, answered read for read.
    fn batch(&self, reads: &[Read]) -> Result<Vec<ReadResult>, StoreError> {
        let results = self.runtime().block_on(self.store.read_many(reads))?;
        if results.len() == reads.len() {
            Ok(results)
        } else {
            Err(StoreError::Document("batch-read-disagrees".into()))
        }
    }
    /// A required object's whole stream from its first batched slice, with its stored metadata.
    fn stored_object(
        &self,
        stream: &StreamId,
        slice: ReadResult,
    ) -> Result<(Vec<RecordedEvent>, ObjectMetadata), StoreError> {
        let ReadResult::Stream(slice) = slice else {
            return Err(StoreError::Document("batch-read-disagrees".into()));
        };
        let mut read = StreamRead::new(stream);
        read.absorb(&self.tenant, slice, &|_| false)?;
        let events = self.read_rest(read, MAX_READ_LIMIT, |_| false)?;
        let meta = stored_metadata(&events)?
            .ok_or_else(|| StoreError::Document("required-object-missing".into()))?;
        Ok((events, meta))
    }
    fn load_object(
        &self,
        history: &mut RetainedHistory,
        hash: ContentHash,
    ) -> Result<(), StoreError> {
        self.load_objects(history, [hash])
    }
    /// Loads every object in `hashes` the history does not already hold, in one provider batch.
    fn load_objects(
        &self,
        history: &mut RetainedHistory,
        hashes: impl IntoIterator<Item = ContentHash>,
    ) -> Result<(), StoreError> {
        let missing: Vec<ContentHash> = hashes
            .into_iter()
            .filter(|hash| !history.objects.contains_key(hash))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        for (hash, object) in missing.iter().zip(self.required(&missing)?) {
            history.objects.insert(*hash, object);
        }
        Ok(())
    }
    fn load_history(
        &self,
        limit: usize,
        selected: Option<RevisionNumber>,
    ) -> Result<RetainedHistory, StoreError> {
        self.load_history_requiring(limit, selected, |history| {
            self.authority()?.required_objects(history)
        })
    }
    fn load_history_requiring(
        &self,
        limit: usize,
        selected: Option<RevisionNumber>,
        required_objects: impl Fn(&RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError>,
    ) -> Result<RetainedHistory, StoreError> {
        let mut history = RetainedHistory {
            occurrences: self.occurrences(limit, selected)?,
            objects: BTreeMap::new(),
        };
        let mut required = BTreeSet::new();
        for occurrence in &history.occurrences {
            required.insert(occurrence.event.record_hash);
            if let RevisionPayload::Seeded { seed_hash, .. } = occurrence.event.payload {
                required.insert(seed_hash);
            }
        }
        self.load_objects(&mut history, required)?;
        if !history.occurrences.is_empty() {
            let required = required_objects(&history)?;
            self.load_objects(&mut history, required)?;
        }
        Ok(history)
    }
    fn admitted(
        &self,
        revision: Option<RevisionNumber>,
        limit: usize,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        let history = self.load_history(limit, revision)?;
        if history.occurrences.is_empty() {
            return Ok(None);
        }
        self.authority()?
            .replay(&history, self.ontology.as_ref(), revision)
    }
}
impl<S: AtomicBlobEventStore> EventlogStore<S> {
    /// Archives a current graph document. This alone cannot initialize canonical state.
    /// # Errors
    /// Runtime-context refusal, serialization or native publication failure.
    pub fn store_graph(
        &self,
        graph: &CanonicalGraph,
        at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        self.put(
            StorageClass::Canonical,
            &GraphDocument::of(graph).to_bytes()?,
            at,
        )
    }
    fn object_append(
        &self,
        hash: ContentHash,
        object: &crate::PublicationObject,
    ) -> Result<Option<StreamAppend>, StoreError> {
        if ContentHash::of_bytes(&object.bytes) != hash {
            return Err(StoreError::Document("staged-object-address".into()));
        }
        let (expected, event) = if let Some((held, version)) = self.object_versioned(hash)? {
            if held.bytes != object.bytes {
                return Err(StoreError::Document("object-address-collision".into()));
            }
            if held.metadata.storage_class.retention_rank() >= object.storage_class.retention_rank()
            {
                return Ok(None);
            }
            let data = RetentionRaised {
                content_hash: hash,
                from: held.metadata.storage_class,
                to: object.storage_class,
            };
            (
                Expected::Exact(version),
                NewEvent::new(
                    OBJECT_RETENTION_RAISED,
                    1,
                    serde_json::to_value(data).map_err(json_error)?,
                )?,
            )
        } else {
            let data = ObjectMetadata {
                content_hash: hash,
                storage_class: object.storage_class,
                byte_len: object.bytes.len() as u64,
                stored_at: object.stored_at,
            };
            (
                Expected::NoStream,
                NewEvent::new(
                    OBJECT_STORED,
                    2,
                    serde_json::to_value(data).map_err(json_error)?,
                )?,
            )
        };
        Ok(Some(StreamAppend {
            stream: self.object_stream(hash)?,
            expected,
            events: vec![event],
        }))
    }
    fn atomic(
        &self,
        request: &BlobAppendGroup,
    ) -> Result<eventlog_core::AppendGroupResult, StoreError> {
        // Retry an uncertain outcome with exactly the same group and bindings, never a new identity
        // or an object cleanup. Native receipt lookup precedes blob revalidation on such a retry.
        for _ in 0..16 {
            match self
                .runtime()
                .block_on(AtomicBlobEventStore::append_group_with_blobs(
                    &self.store,
                    request,
                )) {
                Err(EventLogError::UnknownCommit) => continue,
                result => return result.map_err(StoreError::from),
            }
        }
        Err(StoreError::UnknownCommit)
    }
}
impl<S: AtomicBlobEventStore> RevisionLog for EventlogStore<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.read_preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input_hash: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.elect_preparation(key, input_hash, decision, previous)
    }
    fn resume(&self, prepared: &PublicationPreparationV1) -> Result<Appended, StoreError> {
        self.resume_preparation(prepared)
    }
    fn history_at(&self, revision: RevisionNumber) -> Result<RetainedHistory, StoreError> {
        ensure_sync_context()?;
        let history = self.load_history(1, Some(revision))?;
        if !history.occurrences.is_empty() {
            self.authority()?
                .replay(&history, self.ontology.as_ref(), Some(revision))?;
        }
        Ok(history)
    }
    fn history(&self) -> Result<RetainedHistory, StoreError> {
        ensure_sync_context()?;
        let history = self.load_history(MAX_READ_LIMIT, None)?;
        if !history.occurrences.is_empty() {
            self.authority()?
                .replay(&history, self.ontology.as_ref(), None)?;
        }
        Ok(history)
    }
    fn publish(&self, publication: &Publication) -> Result<Appended, StoreError> {
        ensure_sync_context()?;
        for attempt in 0..16 {
            let mut history = self.load_history(MAX_READ_LIMIT, None)?;
            self.authority()?
                .replay(&history, self.ontology.as_ref(), None)?;
            if let Some(prior) = history
                .occurrences
                .iter()
                .find(|o| o.event.event_id == publication.event.event_id)
            {
                return if prior.event == publication.event {
                    Ok(Appended::AlreadyRecorded)
                } else {
                    Err(StoreError::Document("occurrence-identity-conflict".into()))
                };
            }
            if history.occurrences.len() as u64 != publication.expected_version {
                return Err(StoreError::Conflict);
            }
            if publication.event.format != RevisionEvent::FORMAT || publication.objects.is_empty() {
                return Err(StoreError::Document("invalid-publication-envelope".into()));
            }
            for (hash, object) in &publication.objects {
                if ContentHash::of_bytes(&object.bytes) != *hash {
                    return Err(StoreError::Document("staged-object-address".into()));
                }
                let held = RetainedObject {
                    metadata: StoredObject {
                        content_hash: *hash,
                        storage_class: object.storage_class,
                        byte_len: object.bytes.len() as u64,
                        stored_at: object.stored_at,
                    },
                    bytes: object.bytes.clone(),
                };
                if let Some(prior) = history.objects.get(hash) {
                    if prior.bytes != held.bytes {
                        return Err(StoreError::Document("object-address-collision".into()));
                    }
                }
                history.objects.insert(*hash, held);
            }
            history.occurrences.push(RecordedOccurrence {
                version: publication.expected_version + 1,
                provider_event_id: format!("pending:{}", publication.event.event_id),
                event: publication.event.clone(),
            });
            self.load_object(&mut history, publication.event.record_hash)?;
            let required = self.authority()?.required_objects(&history)?;
            self.load_objects(&mut history, required)?;
            self.authority()?
                .replay(&history, self.ontology.as_ref(), None)?;
            let mut appends = vec![StreamAppend {
                stream: self.revision_stream()?,
                expected: if publication.expected_version == 0 {
                    Expected::NoStream
                } else {
                    Expected::Exact(publication.expected_version)
                },
                events: vec![NewEvent::new(
                    publication.event.name(),
                    2,
                    serde_json::to_value(&publication.event).map_err(json_error)?,
                )?],
            }];
            for (hash, object) in &publication.objects {
                if let Some(append) = self.object_append(*hash, object)? {
                    appends.push(append);
                }
            }
            let request = BlobAppendGroup {
                group: AppendGroup {
                    tenant: self.tenant.clone(),
                    appends,
                    meta: envelope(
                        &format!("ekr.occurrence.{}.{attempt}", publication.event.event_id),
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
            match self.atomic(&request) {
                Ok(result) => {
                    return Ok(if result.deduplicated {
                        Appended::AlreadyRecorded
                    } else {
                        Appended::Written
                    });
                }
                Err(StoreError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Conflict)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        let history = self.history()?;
        match history.occurrences.first().map(|o| &o.event.payload) {
            None => Ok(None),
            Some(RevisionPayload::Seeded { seed_hash, .. }) => Ok(Some(
                history
                    .content(*seed_hash, StorageClass::Canonical)?
                    .to_vec(),
            )),
            Some(_) => Err(StoreError::NotSeeded),
        }
    }
    fn fold(&self) -> Result<CanonicalGraph, StoreError> {
        ensure_sync_context()?;
        Ok(self
            .admitted(None, MAX_READ_LIMIT)?
            .ok_or(StoreError::NotSeeded)?
            .graph)
    }
    fn head(&self) -> Result<Option<Root>, StoreError> {
        ensure_sync_context()?;
        Ok(self.admitted(None, MAX_READ_LIMIT)?.map(|state| state.root))
    }
    fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        ensure_sync_context()?;
        Ok(self
            .admitted(Some(revision), 1)?
            .ok_or(StoreError::NotSeeded)?
            .graph)
    }
}
impl<S: AtomicBlobEventStore> Initialize for EventlogStore<S> {
    fn initialize(&self, publication: &Publication) -> Result<Appended, StoreError> {
        ensure_sync_context()?;
        if publication.expected_version != 0
            || !matches!(publication.event.payload, RevisionPayload::Seeded { .. })
        {
            return Err(StoreError::InvalidSeed("invalid-seed-publication".into()));
        }
        self.publish(publication)
    }
}
impl<S: AtomicBlobEventStore> ObjectStore for EventlogStore<S> {
    fn put(
        &self,
        storage_class: StorageClass,
        bytes: &[u8],
        stored_at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        ensure_sync_context()?;
        let hash = ContentHash::of_bytes(bytes);
        let object = crate::PublicationObject {
            storage_class,
            bytes: bytes.to_vec(),
            stored_at,
        };
        for attempt in 0..16 {
            let Some(append) = self.object_append(hash, &object)? else {
                return Ok(self
                    .object(hash)?
                    .ok_or_else(|| StoreError::Document("object-disappeared".into()))?
                    .metadata);
            };
            let request = BlobAppendGroup {
                group: AppendGroup {
                    tenant: self.tenant.clone(),
                    appends: vec![append],
                    meta: envelope(
                        &format!("ekr.object.{hash}.{}.{attempt}", storage_class.name()),
                        hash.to_hex(),
                    ),
                },
                blobs: vec![BlobWrite {
                    digest: hash.to_hex(),
                    bytes: bytes.to_vec(),
                }],
            };
            match self.atomic(&request) {
                Ok(_) => {
                    return Ok(self
                        .object(hash)?
                        .ok_or_else(|| StoreError::Document("object-disappeared".into()))?
                        .metadata);
                }
                Err(StoreError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Conflict)
    }
    fn get(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        ensure_sync_context()?;
        Ok(self.object(*hash)?.map(|o| o.bytes))
    }
}
/// A stream read in progress: what it has accepted so far and where the next slice starts.
///
/// Every slice, whether it came from its own provider call or from a batch, is accepted through
/// [`StreamRead::absorb`], so a batched read and a single one refuse the same envelopes.
struct StreamRead<'s> {
    stream: &'s StreamId,
    events: Vec<RecordedEvent>,
    native_ids: BTreeSet<String>,
    after: u64,
    finished: bool,
}
impl<'s> StreamRead<'s> {
    fn new(stream: &'s StreamId) -> Self {
        Self {
            stream,
            events: Vec::new(),
            native_ids: BTreeSet::new(),
            after: 0,
            finished: false,
        }
    }
    /// Accepts one slice read after `self.after`: each event must be unredacted, this tenant's and
    /// this stream's, the next version and a provider identity not seen before. Stops at the first
    /// event `stop` names or at the end of the stream; refuses a cursor that does not advance.
    fn absorb(
        &mut self,
        tenant: &TenantId,
        slice: StreamSlice,
        stop: &impl Fn(&RecordedEvent) -> bool,
    ) -> Result<(), StoreError> {
        for event in slice.events {
            if event.is_redacted()
                || event.version != self.events.len() as u64 + 1
                || &event.tenant != tenant
                || event.stream_type != self.stream.stream_type()
                || event.stream_id != self.stream.stream_id()
                || !self.native_ids.insert(event.event_id.clone())
            {
                return Err(StoreError::Document("stream-envelope-disagrees".into()));
            }
            let reached = stop(&event);
            self.events.push(event);
            if reached {
                self.finished = true;
                return Ok(());
            }
        }
        if slice.end_of_stream {
            self.finished = true;
            return Ok(());
        }
        if slice.next_version <= self.after {
            return Err(StoreError::Document("stream-cursor-stalled".into()));
        }
        self.after = slice.next_version;
        Ok(())
    }
}
/// What an object's stream says was stored, checked before its blob is read.
///
/// `None` for an object with no stream. Otherwise the first event must be a current
/// `ObjectStored` envelope whose payload is the stored metadata.
fn stored_metadata(events: &[RecordedEvent]) -> Result<Option<ObjectMetadata>, StoreError> {
    let Some(first) = events.first() else {
        return Ok(None);
    };
    if first.name != OBJECT_STORED || first.schema_version != 2 {
        return Err(StoreError::Document(
            "unsupported-object-envelope: legacy inline records require migration".into(),
        ));
    }
    serde_json::from_value(first.data.clone())
        .map(Some)
        .map_err(|e| StoreError::Document(e.to_string()))
}
/// One retained object from its whole stream, the metadata [`stored_metadata`] read from it, and
/// its blob, however the three were read.
///
/// The blob must be present and hash to the address with the recorded length, and every later
/// event must be a retention raise that only strengthens the class. Returns the object at its
/// strongest class and the stream's length.
fn retained_object(
    hash: ContentHash,
    events: &[RecordedEvent],
    mut meta: ObjectMetadata,
    blob: Option<Vec<u8>>,
) -> Result<(RetainedObject, u64), StoreError> {
    let later = events.get(1..).unwrap_or_default();
    let bytes =
        blob.ok_or_else(|| StoreError::Document("object-integrity: native blob missing".into()))?;
    if meta.content_hash != hash
        || meta.byte_len != bytes.len() as u64
        || ContentHash::of_bytes(&bytes) != hash
    {
        return Err(StoreError::Document(
            "object-integrity: address or byte length disagrees".into(),
        ));
    }
    for event in later {
        if event.name != OBJECT_RETENTION_RAISED || event.schema_version != 1 {
            return Err(StoreError::Document(
                "unsupported-retention-envelope".into(),
            ));
        }
        let raised: RetentionRaised = serde_json::from_value(event.data.clone())
            .map_err(|e| StoreError::Document(e.to_string()))?;
        if raised.content_hash != hash
            || raised.to.retention_rank() <= raised.from.retention_rank()
            || raised.from.retention_rank() > meta.storage_class.retention_rank()
        {
            return Err(StoreError::Document(
                "object-integrity: invalid retention metadata".into(),
            ));
        }
        meta.storage_class = meta.storage_class.strongest(raised.to);
    }
    Ok((
        RetainedObject {
            metadata: StoredObject {
                content_hash: hash,
                storage_class: meta.storage_class,
                byte_len: meta.byte_len,
                stored_at: meta.stored_at,
            },
            bytes,
        },
        events.len() as u64,
    ))
}
fn json_error(error: serde_json::Error) -> StoreError {
    StoreError::Document(error.to_string())
}
fn envelope(key: &str, hash: String) -> CommandMeta {
    CommandMeta {
        idempotency_key: key.into(),
        request_hash: hash,
        subject: WRITER.into(),
        actor: WRITER.into(),
        request_id: key.into(),
        trace_id: key.into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}
