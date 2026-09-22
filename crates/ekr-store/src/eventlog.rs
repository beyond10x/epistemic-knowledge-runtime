//! The eventlog-backed store, over the SQLite and file providers.
//!
//! ADR 0001 puts persistence through `beyond10x/eventlog`. This module is the whole of the
//! coupling: the stream coordinates, the envelope every append carries, and the bridge from the
//! port's async surface to the synchronous one this runtime uses.
//!
//! # Why a runtime lives here
//!
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port`. `eventlog-core`'s
//! `EventStore` trait is async — every method returns a `BoxFuture` — and `eventlog-sqlite` wraps a
//! synchronous `rusqlite` behind `tokio::task::spawn_blocking`, so a future driven outside a tokio
//! runtime panics rather than failing. This workspace declared no runtime at all. Rather than
//! colour `ekr-kernel`, the CLI and everything above async to satisfy a dependency's calling
//! convention, the bridge is in the one crate that touches the port: each store owns a
//! current-thread runtime and every public method is synchronous.
//!
//! Constructors and public persistence methods return [`StoreError::RuntimeContext`] on a thread
//! entered into a Tokio runtime, before opening a provider, calling authority or doing I/O. Async
//! consumers must call this synchronous API on a thread outside that runtime. Dropping a store in
//! an entered runtime shuts its owned runtime down without blocking that thread.
//!
//! # Two providers, one implementation
//!
//! [`EventlogStore`] is generic over `EventStore`, so the properties `tests/providers.rs` proves
//! are proved against one body of code rather than two that drifted. [`SqliteStore`] and
//! [`FileStore`] are the two instantiations P1 ships.

use std::path::Path;

use ekr_core::{ContentHash, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionEvent, Root};
use ekr_ontology::Ontology;
use eventlog_core::{
    AppendGroup, AtomicEventStore, CommandMeta, EventLogError, EventStore, Expected, NewEvent,
    StreamAppend, StreamId, TenantId, MAX_READ_LIMIT,
};
use eventlog_file::FileEventStore;
use eventlog_sqlite::SqliteEventStore;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::runtime::{Builder, Handle, Runtime};

use crate::log::{Appended, CommitAuthority, Fold};
use crate::{
    GraphDocument, Initialize, ObjectStore, RevisionLog, StorageClass, StoreError, StoredObject,
};

/// The stream type one tenant's revision lineage lives under.
const REVISION_STREAM_TYPE: &str = "ekr.revision";

/// The one lineage a P1 store holds. Tenancy is one tenant per store, named in the stream
/// coordinate as eventlog requires; the lineage within it is the canonical core's.
const REVISION_STREAM_ID: &str = "canonical";

/// The stream type one content-addressed object lives under, one stream per address.
const OBJECT_STREAM_TYPE: &str = "ekr.store.object";

/// The event an object's stream opens with, exactly once: the bytes themselves.
const OBJECT_STORED: &str = "ekr.store.ObjectStored";

/// The event a later, stronger retention request appends. Carries the class and not the bytes —
/// the bytes are already in the stream and are the object's identity.
const OBJECT_RETENTION_RAISED: &str = "ekr.store.ObjectRetentionRaised";

/// The identity the store puts on every envelope it writes.
///
/// An opaque id and never a person: eventlog refuses an address or a display name on an envelope,
/// because an append-only log outlives every request to erase one. Nothing the store writes is
/// attributable to a person anyway — the agent that proposed a transaction is inside the event,
/// where the runtime's own provenance rules apply to it.
const WRITER: &str = "ekr.store";

/// A store whose revision log lives in a SQLite database.
pub type SqliteStore = EventlogStore<SqliteEventStore>;

/// A store whose revision log lives in a directory of files.
pub type FileStore = EventlogStore<FileEventStore>;

/// A later caller asking that bytes already stored be kept more strongly:
/// `ekr.store.ObjectRetentionRaised`, as `systems/ekr/domains/store.yaml` declares it.
///
/// The three declared fields and no others. The first implementation carried only the new class,
/// so the stored bytes read `{"storage_class":"Canonical"}` and a later reader could say neither
/// which object had been raised nor which way — which made a raise indistinguishable from an
/// original write in a log nobody had the types for. `from` and `to` are both carried because the
/// declaration's own comment says why: so a reader can see the ladder was climbed and not
/// descended.
///
/// `content_hash` is redundant with the stream coordinate, which is that hash, and is carried
/// anyway: an event body that only makes sense beside the stream it was read from is a body that
/// stops making sense the moment anything exports one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetentionRaised {
    /// The object whose retention was raised.
    content_hash: ContentHash,
    /// The class it was recorded as before this request.
    from: StorageClass,
    /// The class it is recorded as after it.
    to: StorageClass,
}

/// One object's record, as the first event of its stream holds it:
/// `ekr.store.ObjectStored`, as `systems/ekr/domains/store.yaml` declares it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectRecord {
    /// The address these bytes have, which is their identity.
    content_hash: ContentHash,
    /// What it is kept for.
    storage_class: StorageClass,
    /// How many bytes it is.
    byte_len: u64,
    /// When it was first stored.
    stored_at: Timestamp,
    /// The payload itself.
    bytes: Vec<u8>,
}

/// An eventlog-backed [`RevisionLog`] and [`ObjectStore`].
///
/// Owns a current-thread tokio runtime, which is what makes every method here synchronous. See the
/// module documentation for why, and for what it costs.
pub struct EventlogStore<S: EventStore> {
    runtime: Option<Runtime>,
    store: S,
    tenant: TenantId,
    /// The schema the folded graph is typed by.
    ///
    /// Constructor compatibility input. The kernel compares its full content with the complete
    /// ontology retained in the seed envelope on every replay.
    ontology: Ontology,
    /// What the fold asks before a commit moves canonical state, or `None`.
    ///
    /// `None` is the honest default and not an oversight: a store is opened with a path, a tenant
    /// and a schema, none of which says anything about who may commit into it, and a fold that
    /// assumed an absent authority meant "anyone" is the defect ADR 0007 repairs. See
    /// [`CommitAuthority`] and [`EventlogStore::under`].
    authority: Option<Box<dyn CommitAuthority>>,
}

impl EventlogStore<SqliteEventStore> {
    /// Opens or creates the SQLite-backed store at `path`.
    ///
    /// # Errors
    ///
    /// [`StoreError::Backend`] when the database cannot be opened or its tables created, or when
    /// `tenant` is not a usable tenant identity.
    /// [`StoreError::RuntimeContext`] when called from an entered Tokio runtime.
    pub fn sqlite(path: &Path, tenant: &str, ontology: Ontology) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let text = path.to_string_lossy().into_owned();
        let store = runtime.block_on(SqliteEventStore::open(&text, "ekr"))?;
        Self::assemble(runtime, store, tenant, ontology)
    }
}

impl EventlogStore<FileEventStore> {
    /// Opens or creates the file-backed store rooted at `path`.
    ///
    /// # Errors
    ///
    /// [`StoreError::Backend`] when the directory cannot be created or its history is unreadable,
    /// or when `tenant` is not a usable tenant identity.
    /// [`StoreError::RuntimeContext`] when called from an entered Tokio runtime.
    pub fn file(path: &Path, tenant: &str, ontology: Ontology) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        std::fs::create_dir_all(path).map_err(|error| {
            StoreError::Backend(format!("creating {}: {error}", path.display()))
        })?;
        let store = runtime.block_on(FileEventStore::open(path))?;
        Self::assemble(runtime, store, tenant, ontology)
    }
}

/// A current-thread runtime, which is the whole of what this crate needs from tokio.
fn new_runtime() -> Result<Runtime, StoreError> {
    ensure_sync_context()?;
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| StoreError::Backend(format!("building a runtime: {error}")))
}

/// Refuse even an entered, idle handle: the public synchronous contract does not depend on how
/// an executor happens to poll its caller.
fn ensure_sync_context() -> Result<(), StoreError> {
    if Handle::try_current().is_ok() {
        Err(StoreError::RuntimeContext)
    } else {
        Ok(())
    }
}

impl<S: EventStore> Drop for EventlogStore<S> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            if Handle::try_current().is_ok() {
                // Every accepted operation completed synchronously before Drop could borrow this
                // store exclusively. Do not try to block the ambient executor during shutdown.
                runtime.shutdown_background();
            }
            // Outside an entered runtime, ordinary Drop waits for owned runtime shutdown.
        }
    }
}

impl<S: EventStore> EventlogStore<S> {
    fn runtime(&self) -> &Runtime {
        self.runtime.as_ref().expect("runtime is owned until drop")
    }

    /// The parts, once the provider is open.
    fn assemble(
        runtime: Runtime,
        store: S,
        tenant: &str,
        ontology: Ontology,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            runtime: Some(runtime),
            store,
            tenant: TenantId::new(tenant)?,
            ontology,
            authority: None,
        })
    }

    /// The same store, folding commits `authority` stands behind.
    ///
    /// `architecture-decision-record:0007-the-commit-path-is-the-kernels`. Its one caller in the
    /// workspace is `ekr-kernel`, which is the only crate that declares this one; a store that
    /// never passes through it folds no commit, which is what
    /// `crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs` reads.
    #[must_use]
    pub fn under(mut self, authority: impl CommitAuthority + 'static) -> Self {
        self.authority = Some(Box::new(authority));
        self
    }

    /// Writes `graph` as a content-addressed object, under [`StorageClass::Canonical`].
    ///
    /// Archival serialization only. Bootstrap publication uses [`Initialize`] with a complete
    /// kernel-admitted envelope; a raw graph document cannot initialize a runtime.
    ///
    /// # Errors
    ///
    /// [`StoreError::Document`] when the graph cannot be serialised, and [`StoreError::Backend`]
    /// when the provider is unavailable.
    /// [`StoreError::RuntimeContext`] when called from an entered Tokio runtime.
    pub fn store_graph(
        &self,
        graph: &CanonicalGraph,
        stored_at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        ensure_sync_context()?;
        let bytes = GraphDocument::of(graph).to_bytes()?;
        self.put(StorageClass::Canonical, &bytes, stored_at)
    }

    /// The lineage's stream.
    fn revision_stream(&self) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            REVISION_STREAM_TYPE,
            REVISION_STREAM_ID,
        )?)
    }

    /// One object's stream. One stream per address, so that a second write of the same bytes meets
    /// a stream that already exists rather than a row it has to search for.
    fn object_stream(&self, content_hash: &ContentHash) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            OBJECT_STREAM_TYPE,
            content_hash.to_hex(),
        )?)
    }

    /// Every event of a stream, in order, read `limit` at a time until the provider says it is
    /// done.
    ///
    /// Paged rather than asked for in one read: `MAX_READ_LIMIT` is the most any single read
    /// returns, and a fold that asked once would silently stop at it.
    fn read_all(
        &self,
        stream: &StreamId,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let mut bodies = Vec::new();
        let mut after = 0u64;
        loop {
            let slice = self
                .runtime()
                .block_on(self.store.read_stream(stream, after, limit))?;
            for recorded in &slice.events {
                if recorded.is_redacted() {
                    return Err(StoreError::Document(format!(
                        "version {} of {} was redacted and no longer folds",
                        recorded.version,
                        stream.stream_id()
                    )));
                }
                bodies.push(recorded.data.clone());
            }
            if slice.end_of_stream {
                return Ok(bodies);
            }
            after = slice.next_version;
        }
    }

    /// Every revision event of the lineage, in order.
    fn revision_events(&self, limit: usize) -> Result<Vec<RevisionEvent>, StoreError> {
        let stream = self.revision_stream()?;
        self.read_all(&stream, limit)?
            .into_iter()
            .map(|body| {
                serde_json::from_value(body)
                    .map_err(|error| StoreError::Document(error.to_string()))
            })
            .collect()
    }

    /// Folds the lineage from `from`, reading `limit` events at a time, or refuses because there
    /// is no state to begin at.
    fn fold_from(
        &self,
        from: RevisionNumber,
        limit: usize,
    ) -> Result<Option<Fold<'_>>, StoreError> {
        if from != RevisionNumber::SEED {
            return Err(StoreError::NoMaterialisedState { requested: from });
        }
        let events = self.revision_events(limit)?;
        let Some((first, rest)) = events.split_first() else {
            return Ok(None);
        };
        let RevisionEvent::Seeded { seed_hash, .. } = first else {
            return Err(StoreError::NotSeeded);
        };
        let bytes = self.retained_seed(seed_hash)?;
        let authority = self
            .authority
            .as_deref()
            .ok_or(StoreError::NoSeedAuthority)?;
        let seed = authority.admit_seed(&bytes, &self.ontology)?;

        let mut fold = Fold::seeded(seed, *seed_hash, self.authority.as_deref());
        for event in rest {
            fold.apply(event)?;
        }
        Ok(Some(fold))
    }

    /// One object's record, folded over its whole stream, or `None` when this store does not hold
    /// it.
    ///
    /// The first event carries the bytes, the instant and the class its first writer asked for;
    /// every later event raises the class. The record a reader gets is therefore the bytes as
    /// written and **the strongest retention anyone ever requested**, which is what
    /// [`ObjectStore::put`] documents and why an object's history is a fold rather than a row.
    fn object_record(
        &self,
        content_hash: &ContentHash,
    ) -> Result<Option<ObjectRecord>, StoreError> {
        let stream = self.object_stream(content_hash)?;
        let mut bodies = self.read_all(&stream, MAX_READ_LIMIT)?.into_iter();
        let Some(first) = bodies.next() else {
            return Ok(None);
        };
        let mut record: ObjectRecord = serde_json::from_value(first)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        if record.content_hash != *content_hash
            || record.byte_len != record.bytes.len() as u64
            || ContentHash::of_bytes(&record.bytes) != *content_hash
        {
            return Err(StoreError::Document(
                "object-integrity: address or byte length disagrees".to_owned(),
            ));
        }
        for body in bodies {
            let raised: RetentionRaised = serde_json::from_value(body)
                .map_err(|error| StoreError::Document(error.to_string()))?;
            if raised.content_hash != *content_hash
                || raised.to.retention_rank() <= raised.from.retention_rank()
                || raised.from.retention_rank() > record.storage_class.retention_rank()
            {
                return Err(StoreError::Document(
                    "object-integrity: invalid retention metadata".to_owned(),
                ));
            }
            // `to` and not `from`: the fold takes the strongest of every request, and `from` is
            // carried so that a reader of the bytes can see the direction, not so that the fold
            // can trust it.
            record.storage_class = record.storage_class.strongest(raised.to);
        }
        Ok(Some(record))
    }

    fn retained_seed(&self, hash: &ContentHash) -> Result<Vec<u8>, StoreError> {
        let record = self
            .object_record(hash)?
            .ok_or(StoreError::SeedNotStored { seed_hash: *hash })?;
        if record.storage_class != StorageClass::Canonical {
            return Err(StoreError::Document(
                "seed-retention: seed is not retained as canonical".to_owned(),
            ));
        }
        Ok(record.bytes)
    }

    /// The first write of some bytes: the one event that carries them.
    fn store_first(
        &self,
        content_hash: ContentHash,
        storage_class: StorageClass,
        bytes: &[u8],
        stored_at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        let record = ObjectRecord {
            content_hash,
            storage_class,
            byte_len: bytes.len() as u64,
            stored_at,
            bytes: bytes.to_vec(),
        };
        let body = serde_json::to_value(&record)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        let stream = self.object_stream(&content_hash)?;
        let new = NewEvent::new(OBJECT_STORED, 1, body)?;
        let meta = envelope(
            &format!("{OBJECT_STREAM_TYPE}.{}", content_hash.to_hex()),
            content_hash.to_hex(),
        );

        match self.runtime().block_on(self.store.append(
            &stream,
            Expected::NoStream,
            std::slice::from_ref(&new),
            &meta,
        )) {
            Ok(_) => Ok(stored(content_hash, &record)),
            // Another writer got there between the read and the append. The object is the bytes,
            // and the bytes are the same, so this call becomes a raise against the record that
            // landed rather than an error and rather than a second object. `Expected::NoStream` is
            // what makes the race detectable at all; this is the one arm in the crate that acts on
            // the provider's own taxonomy rather than flattening it.
            Err(EventLogError::Conflict { .. } | EventLogError::IdempotencyMismatch { .. }) => {
                let landed = self.object_record(&content_hash)?;
                let from = landed.map_or(storage_class, |record| record.storage_class);
                if storage_class.retention_rank() > from.retention_rank() {
                    self.raise_retention(&content_hash, from, storage_class)?;
                }
                self.object_record(&content_hash)?
                    .map(|record| stored(content_hash, &record))
                    .ok_or_else(|| {
                        StoreError::Backend(
                            "an object stream exists and holds no record of the object".to_owned(),
                        )
                    })
            }
            Err(error) => Err(StoreError::from(error)),
        }
    }

    /// The state a completed fold reached, or the refusal that says this store could not evaluate
    /// the lineage at all.
    ///
    /// [`RevisionLog::fold`] and [`RevisionLog::replay`] answer *what canonical state is*, and a
    /// store opened with no [`CommitAuthority`] cannot say: it has a commit in front of it and
    /// nobody to ask about it. Seed admission has already required an authority in `fold_from`;
    /// every reader, including `head`, refuses a seed it cannot admit.
    fn state(folded: Option<Fold<'_>>) -> Result<CanonicalGraph, StoreError> {
        let folded = folded.ok_or(StoreError::NotSeeded)?;
        if let Some(transaction_id) = folded.unauthorised() {
            return Err(StoreError::NoCommitAuthority { transaction_id });
        }
        Ok(folded.into_graph())
    }

    /// Records that someone asked for `storage_class` over bytes already stored.
    ///
    /// Append-only: the earlier record is not rewritten, because an object's history is immutable
    /// like everything else this crate holds (AGENTS.md invariant 5). The key is the object and the
    /// class together, so asking twice for the same class is one raise and the fold is the same
    /// whether the caller retried or not.
    fn raise_retention(
        &self,
        content_hash: &ContentHash,
        from: StorageClass,
        to: StorageClass,
    ) -> Result<(), StoreError> {
        let body = serde_json::to_value(RetentionRaised {
            content_hash: *content_hash,
            from,
            to,
        })
        .map_err(|error| StoreError::Document(error.to_string()))?;
        let stream = self.object_stream(content_hash)?;
        let new = NewEvent::new(OBJECT_RETENTION_RAISED, 1, body)?;
        let key = format!(
            "{OBJECT_STREAM_TYPE}.{}.{}",
            content_hash.to_hex(),
            to.name()
        );
        let meta = envelope(&key, key.clone());
        self.runtime()
            .block_on(
                self.store
                    .append(&stream, Expected::Any, std::slice::from_ref(&new), &meta),
            )
            .map(|_| ())
            .map_err(StoreError::from)
    }
}

impl<S: EventStore> RevisionLog for EventlogStore<S> {
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        ensure_sync_context()?;
        match self.revision_events(1)?.first() {
            None => Ok(None),
            Some(RevisionEvent::Seeded { seed_hash, .. }) => {
                self.retained_seed(seed_hash).map(Some)
            }
            Some(_) => Err(StoreError::NotSeeded),
        }
    }
    fn append(&self, event: &RevisionEvent) -> Result<Appended, StoreError> {
        ensure_sync_context()?;
        let stream = self.revision_stream()?;
        let body =
            serde_json::to_value(event).map_err(|error| StoreError::Document(error.to_string()))?;
        let new = NewEvent::new(event.name(), 1, body.clone())?;
        // The key is derived from the event's own content, so a retry of an interrupted append is
        // the same request and writes the event once.
        //
        // The first implementation derived it from the stream version read just before the append,
        // which is the opposite: a landed append moves that version, so the retry computed a
        // different key and the provider — which dedupes on `(tenant, stream_type, stream_id,
        // idempotency_key)` — saw a new command and wrote the event a second time. Measured by the
        // adversary of wave p1-05 as one `Seeded` appended and retried once making `fold` answer
        // `SeedIsNotFirst`.
        //
        // The consequence is worth stating: **two byte-identical events cannot both be in the log**,
        // because the store cannot tell a second one from a retry of the first. That is right for
        // this vocabulary rather than a limitation of it — every `RevisionEvent` variant carries a
        // minted `RevisionId` or names a `TransactionId`, and each of the six says something that
        // happens once for that id: a lineage is seeded once, a transaction is proposed, validated,
        // rejected and committed once each. Two distinct facts are never byte-identical here.
        let request_hash = eventlog_core::request_hash(&body)?;
        let meta = envelope(
            &format!("{REVISION_STREAM_TYPE}.{request_hash}"),
            request_hash.clone(),
        );
        // `Expected::Any`: design § 72 keeps canonical commits globally ordered above this crate,
        // and an append that carried a version the caller never read would refuse a second writer
        // without preventing anything a first one did.
        // `deduplicated` is the provider's own answer to "had this exact command already been
        // recorded", and it is the only place that question can be answered: by the time the call
        // returns, a written event and a recognised one are the same stream.
        self.runtime()
            .block_on(
                self.store
                    .append(&stream, Expected::Any, std::slice::from_ref(&new), &meta),
            )
            .map(|result| {
                if result.deduplicated {
                    Appended::AlreadyRecorded
                } else {
                    Appended::Written
                }
            })
            .map_err(StoreError::from)
    }

    fn fold(&self) -> Result<CanonicalGraph, StoreError> {
        ensure_sync_context()?;
        Self::state(self.fold_from(RevisionNumber::SEED, MAX_READ_LIMIT)?)
    }

    fn head(&self) -> Result<Option<Root>, StoreError> {
        ensure_sync_context()?;
        Ok(self
            .fold_from(RevisionNumber::SEED, MAX_READ_LIMIT)?
            .map(|folded| folded.head()))
    }

    /// One event per read, which is what makes `replay(SEED) == fold()` a check rather than a
    /// tautology.
    ///
    /// [`RevisionLog::fold`] reads the stream in pages of `MAX_READ_LIMIT`; a replay steps it,
    /// because replaying a lineage means applying its events one at a time from a known
    /// beginning. The two therefore exercise different cursor arithmetic over the same stream, and
    /// `tests/providers.rs` holds them to the same answer: a paging defect that dropped or
    /// repeated an event at a page boundary would show as a disagreement rather than as a fold
    /// that quietly stopped early.
    fn replay(&self, from: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        ensure_sync_context()?;
        Self::state(self.fold_from(from, 1)?)
    }
}

impl<S: AtomicEventStore> Initialize for EventlogStore<S> {
    fn initialize(&self, bytes: &[u8], at: Timestamp) -> Result<(), StoreError> {
        ensure_sync_context()?;
        let authority = self
            .authority
            .as_deref()
            .ok_or(StoreError::NoSeedAuthority)?;
        authority.admit_seed(bytes, &self.ontology)?;
        let hash = ContentHash::of_bytes(bytes);
        let revision_id = RevisionId::mint();
        let seed = RevisionEvent::Seeded {
            revision_id,
            seed_hash: hash,
        };
        let body =
            serde_json::to_value(&seed).map_err(|error| StoreError::Document(error.to_string()))?;
        for attempt in 0..16 {
            if !self.revision_events(1)?.is_empty() {
                return Err(StoreError::AlreadySeeded);
            }
            // The lineage expectation remains NoStream in every retry. The second stream's
            // content-addressed race may change the object append, never that expectation.
            let mut appends = vec![StreamAppend {
                stream: self.revision_stream()?,
                expected: Expected::NoStream,
                events: vec![NewEvent::new(seed.name(), 1, body.clone())?],
            }];
            let object = match self.object_record(&hash)? {
                Some(record) => {
                    if record.bytes != bytes {
                        return Err(StoreError::Document(
                            "object-integrity: content-address collision".to_owned(),
                        ));
                    }
                    if record.storage_class == StorageClass::Canonical {
                        None
                    } else {
                        let raised = RetentionRaised {
                            content_hash: hash,
                            from: record.storage_class,
                            to: StorageClass::Canonical,
                        };
                        Some((
                            Expected::Any,
                            NewEvent::new(
                                OBJECT_RETENTION_RAISED,
                                1,
                                serde_json::to_value(raised)
                                    .map_err(|error| StoreError::Document(error.to_string()))?,
                            )?,
                        ))
                    }
                }
                None => {
                    let record = ObjectRecord {
                        content_hash: hash,
                        storage_class: StorageClass::Canonical,
                        byte_len: bytes.len() as u64,
                        stored_at: at,
                        bytes: bytes.to_vec(),
                    };
                    Some((
                        Expected::NoStream,
                        NewEvent::new(
                            OBJECT_STORED,
                            1,
                            serde_json::to_value(record)
                                .map_err(|error| StoreError::Document(error.to_string()))?,
                        )?,
                    ))
                }
            };
            if let Some((expected, event)) = object {
                appends.push(StreamAppend {
                    stream: self.object_stream(&hash)?,
                    expected,
                    events: vec![event],
                });
            }
            let request = AppendGroup {
                tenant: self.tenant.clone(),
                appends,
                meta: envelope(&format!("ekr.seed.{revision_id}.{attempt}"), hash.to_hex()),
            };
            match self.runtime().block_on(self.store.append_group(&request)) {
                Ok(result) if !result.deduplicated => return Ok(()),
                Ok(_) => return Err(StoreError::AlreadySeeded),
                Err(EventLogError::Conflict { .. }) => {
                    if !self.revision_events(1)?.is_empty() {
                        return Err(StoreError::AlreadySeeded);
                    }
                }
                Err(error) => return Err(StoreError::from(error)),
            }
        }
        Err(StoreError::Backend(
            "seed object contention exceeded retry limit".to_owned(),
        ))
    }
}

impl<S: EventStore> ObjectStore for EventlogStore<S> {
    fn put(
        &self,
        storage_class: StorageClass,
        bytes: &[u8],
        stored_at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        ensure_sync_context()?;
        let content_hash = ContentHash::of_bytes(bytes);
        match self.object_record(&content_hash)? {
            // Already stored. The bytes and the instant do not move; the class may only rise.
            Some(record)
                if storage_class.retention_rank() <= record.storage_class.retention_rank() =>
            {
                Ok(stored(content_hash, &record))
            }
            Some(record) => {
                self.raise_retention(&content_hash, record.storage_class, storage_class)?;
                Ok(stored(
                    content_hash,
                    &ObjectRecord {
                        storage_class: record.storage_class.strongest(storage_class),
                        ..record
                    },
                ))
            }
            None => self.store_first(content_hash, storage_class, bytes, stored_at),
        }
    }

    fn get(&self, content_hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        ensure_sync_context()?;
        Ok(self.object_record(content_hash)?.map(|record| record.bytes))
    }
}

/// The record, as the object it describes.
fn stored(content_hash: ContentHash, record: &ObjectRecord) -> StoredObject {
    StoredObject {
        content_hash,
        storage_class: record.storage_class,
        byte_len: record.byte_len,
        stored_at: record.stored_at,
    }
}

/// The envelope the store puts on an append.
///
/// `occurred_at` is the Unix epoch rather than a clock reading, and that is the honest value: this
/// runtime has no clock — every timestamp it holds is one a caller supplied — and eventlog stamps
/// its own `recorded_at` and never orders by this field. A fabricated instant here would be a
/// number a reader could mistake for an observation.
fn envelope(key: &str, request_hash: String) -> CommandMeta {
    CommandMeta {
        idempotency_key: key.to_owned(),
        request_hash,
        subject: WRITER.to_owned(),
        actor: WRITER.to_owned(),
        request_id: key.to_owned(),
        trace_id: key.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}
