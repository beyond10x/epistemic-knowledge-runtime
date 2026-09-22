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
    EventStore, Expected, NewEvent, RecordedEvent, StreamAppend, StreamId, TenantId,
    MAX_READ_LIMIT,
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
        let store = runtime.block_on(SqliteEventStore::open(&path.to_string_lossy(), "ekr"))?;
        Self::assemble(runtime, store, tenant, ontology.into())
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
    fn read_all(&self, stream: &StreamId, limit: usize) -> Result<Vec<RecordedEvent>, StoreError> {
        self.read_until(stream, limit, |_| false)
    }
    fn read_until(
        &self,
        stream: &StreamId,
        limit: usize,
        stop: impl Fn(&RecordedEvent) -> bool,
    ) -> Result<Vec<RecordedEvent>, StoreError> {
        let mut result = Vec::new();
        let mut native_ids = BTreeSet::new();
        let mut after = 0;
        loop {
            let slice = self
                .runtime()
                .block_on(self.store.read_stream(stream, after, limit))?;
            for event in slice.events {
                if event.is_redacted()
                    || event.version != result.len() as u64 + 1
                    || event.tenant != self.tenant
                    || event.stream_type != stream.stream_type()
                    || event.stream_id != stream.stream_id()
                    || !native_ids.insert(event.event_id.clone())
                {
                    return Err(StoreError::Document("stream-envelope-disagrees".into()));
                }
                let reached = stop(&event);
                result.push(event);
                if reached {
                    return Ok(result);
                }
            }
            if slice.end_of_stream {
                return Ok(result);
            }
            if slice.next_version <= after {
                return Err(StoreError::Document("stream-cursor-stalled".into()));
            }
            after = slice.next_version;
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
        let Some((first, later)) = events.split_first() else {
            return Ok(None);
        };
        if first.name != OBJECT_STORED || first.schema_version != 2 {
            return Err(StoreError::Document(
                "unsupported-object-envelope: legacy inline records require migration".into(),
            ));
        }
        let mut meta: ObjectMetadata = serde_json::from_value(first.data.clone())
            .map_err(|e| StoreError::Document(e.to_string()))?;
        let bytes = self
            .runtime()
            .block_on(self.store.get_blob(&self.tenant, &hash.to_hex()))?
            .ok_or_else(|| StoreError::Document("object-integrity: native blob missing".into()))?;
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
        Ok(Some((
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
        )))
    }
    fn load_object(
        &self,
        history: &mut RetainedHistory,
        hash: ContentHash,
    ) -> Result<(), StoreError> {
        if let std::collections::btree_map::Entry::Vacant(entry) = history.objects.entry(hash) {
            entry.insert(
                self.object(hash)?
                    .ok_or_else(|| StoreError::Document("required-object-missing".into()))?,
            );
        }
        Ok(())
    }
    fn load_history(
        &self,
        limit: usize,
        selected: Option<RevisionNumber>,
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
        for hash in required {
            self.load_object(&mut history, hash)?;
        }
        if !history.occurrences.is_empty() {
            for hash in self.authority()?.required_objects(&history)? {
                self.load_object(&mut history, hash)?;
            }
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
            for hash in self.authority()?.required_objects(&history)? {
                self.load_object(&mut history, hash)?;
            }
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
