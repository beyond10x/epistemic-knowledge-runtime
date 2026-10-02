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
    sync::Arc,
};
use time::OffsetDateTime;
use tokio::runtime::{Builder, Handle, Runtime};

const REVISION_STREAM_TYPE: &str = "ekr.revision";
const REVISION_STREAM_ID: &str = "canonical";
const OBJECT_STREAM_TYPE: &str = "ekr.store.object";
const OBJECT_STORED: &str = "ekr.store.ObjectStored";
const OBJECT_RETENTION_RAISED: &str = "ekr.store.ObjectRetentionRaised";
const WRITER: &str = "ekr.store";
#[path = "inventory.rs"]
mod inventory;
#[path = "preparation.rs"]
mod preparation;
#[path = "read_only.rs"]
mod read_only;
#[path = "replaced.rs"]
mod replaced;
pub use inventory::{InventoriedObject, Inventory, StoreInventory};
pub use preparation::{
    NativeBlobWrite, NativeClaim, NativeCommandMeta, NativeExpected, NativeExpectedKind,
    NativeNewEvent, NativePublicationRequest, NativeStreamAppend, NativeStreamId,
    PublicationCommandKey, PublicationCommandKind, PublicationPreparationV1,
    StagedPublicationObject,
};
pub use read_only::remove_read_only_copies;
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
    /// Objects this handle has already read and verified: stream, metadata and bytes.
    ///
    /// A digest binds one byte sequence in a tenant until the blob is deleted, so a verified read
    /// is not repeated while the object's stream stands still. What can move it is an event on
    /// that stream: a retention raise, and — `systems/ekr/domains/store.yaml`, held bytes — any
    /// operation that withdraws retained bytes (a redaction, a deletion, a retention lowering),
    /// which appends one there; nothing in this runtime withdraws bytes in place without it. A
    /// held object below the strongest class is read again once the log shows such an event
    /// (`log_seen`); a `Canonical` object is never withdrawn (invariant 5) and is not looked for. A
    /// redaction or blob deletion made through the provider directly, outside this runtime, moves
    /// no log position and is not seen until the handle next reads that stream. An object this
    /// handle writes is forgotten, and read again when it is next required. The bytes are held
    /// once, shared with the process's registry of verified bytes (`crate::verified`) for as long
    /// as they are held.
    verified: std::sync::Mutex<BTreeMap<ContentHash, HeldObject>>,
    /// The tenant log position through which this handle has looked for events on the streams of
    /// the objects it holds; `None` while it holds none.
    ///
    /// A raise or a withdrawal appends an event to the object's own stream, so an event the log
    /// published after this position is the only way a held object can have moved. Every held
    /// object was verified when the log had reached at least this position, so looking through
    /// the log from it finds every event appended to a held stream since that object was
    /// verified.
    log_seen: std::sync::Mutex<Option<u64>>,
    /// The revision stream's prefix this handle has read and checked. The stream is append-only,
    /// so a later read fetches only the occurrences after it.
    revisions: std::sync::Mutex<HeldRevisions>,
    /// Whether the retained replay checkpoint is offered to the authority. Off for full replay.
    checkpoints: bool,
    /// Whether it has been offered already: once per handle, on the first head history read.
    checkpoint_offered: std::sync::atomic::AtomicBool,
    /// Payload addresses of the publication preparations this handle authorized.
    ///
    /// What authorizing a preparation reads besides its own bytes is fixed once it is elected —
    /// the revision-stream prefix and the object-stream prefixes it names, and this handle's
    /// tenant, authority and ontology — so bytes read back at one of these addresses are that
    /// authorization's input exactly and are not authorized again. Any other bytes are.
    authorized: std::sync::Mutex<BTreeSet<ContentHash>>,
    /// The newest checkpoint pointer and the pointer stream's length after this handle's last
    /// pointer append, where its next write continues from. Taken by that write, and kept again
    /// only when the write appends: a write that appends nothing from it reads the stream afresh.
    pointer: std::sync::Mutex<Option<(Option<CheckpointWritten>, u64)>>,
    /// For a SQLite store opened for writing: the database file it opened, checked before every
    /// read and write (`replaced.rs`). A file replaced in place is refused as
    /// [`StoreError::Replaced`].
    at_path: Option<replaced::AtPath>,
    /// Set when the store was opened read-only: every write is refused and no checkpoint is
    /// written. Last, so that a File store's private copy is removed after the provider over it.
    read_only: Option<read_only::ReadOnly>,
}
/// One verified object as a handle keeps it: its metadata, and its bytes shared with the process's
/// registry of verified bytes.
struct HeldObject {
    metadata: StoredObject,
    bytes: Arc<Vec<u8>>,
    /// Whether its stream may have an event this handle has not read: set when the log shows one
    /// after the object was verified, or when that cannot be told.
    moved: bool,
}
impl HeldObject {
    /// The object as a history holds it: these very bytes, shared, never a copy of them.
    fn retained(&self) -> RetainedObject {
        RetainedObject {
            metadata: self.metadata.clone(),
            bytes: Arc::clone(&self.bytes),
        }
    }
}
/// A retained object whose blob [`retained_object`] checked against its address. Only this type
/// enters a handle's memo and the registry of verified bytes.
struct CheckedObject(RetainedObject);
/// The prefix of the revision stream one handle has read, each occurrence checked as
/// [`EventlogStore::occurrences`] checks it, with the identities the next occurrence must not
/// repeat.
#[derive(Default)]
struct HeldRevisions {
    occurrences: Vec<RecordedOccurrence>,
    native_ids: BTreeSet<String>,
    event_ids: BTreeSet<ekr_core::EventId>,
    /// The highest tenant log position of an occurrence read: the log had reached it then.
    position: u64,
}
impl HeldRevisions {
    /// Checks the next recorded event of the stream and holds it as an occurrence.
    fn accept(&mut self, recorded: RecordedEvent) -> Result<(), StoreError> {
        crate::verified::count(|work| work.occurrences_read += 1);
        let position = recorded.global_seq;
        let event: RevisionEvent = serde_json::from_value(recorded.data)
            .map_err(|e| StoreError::Document(e.to_string()))?;
        if event.format != RevisionEvent::FORMAT
            || recorded.schema_version != 2
            || recorded.name != event.name()
            || self.event_ids.contains(&event.event_id)
        {
            return Err(StoreError::Document("revision-envelope-disagrees".into()));
        }
        self.event_ids.insert(event.event_id);
        self.native_ids.insert(recorded.event_id.clone());
        self.occurrences.push(RecordedOccurrence {
            version: recorded.version,
            provider_event_id: recorded.event_id,
            event,
        });
        self.position = self.position.max(position);
        Ok(())
    }
}
/// Whether `event` is the occurrence that made committed revision `revision`.
fn makes(event: &RevisionEvent, revision: RevisionNumber) -> bool {
    match event.payload {
        RevisionPayload::Seeded { .. } => revision == RevisionNumber::SEED,
        RevisionPayload::RevisionCommitted { number, .. } => number == revision,
        _ => false,
    }
}
const CHECKPOINT_STREAM_TYPE: &str = "ekr.checkpoint";
const CHECKPOINT_STREAM_ID: &str = "canonical";
const CHECKPOINT_WRITTEN: &str = "ekr.store.CheckpointWritten";
/// Names the retained replay checkpoint: private cache data, never a canonical object.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointWritten {
    /// The retained checkpoint blob's payload address.
    checkpoint_hash: ContentHash,
    /// How many revision-stream occurrences the authority verified.
    covered: u64,
    /// The authority's own binding of that prefix to itself.
    binding: ContentHash,
}
fn checkpoint_key(hash: ContentHash) -> String {
    format!("ekr.private.checkpoint.{hash}")
}
impl EventlogStore<SqliteEventStore> {
    /// Opens the SQLite provider outside an entered async runtime.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, provider failure or
    /// [`StoreError::Replaced`] for a database file that is not the log its connection reads.
    pub fn sqlite(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let tenant = TenantId::new(tenant)?;
        let named = path.to_string_lossy();
        let store =
            waiting_out_the_lock(|| runtime.block_on(SqliteEventStore::open(&named, "ekr")))?;
        Self::at(
            Self::assemble(runtime, store, tenant, ontology.into()),
            path,
        )
    }
    /// Opens an already provisioned SQLite store, creating no database and no tables: a path
    /// holding none is refused and left as it was. A store this process may not write — the
    /// database, its directory or its `-wal` or `-shm` file — is refused as
    /// [`StoreError::ReadOnly`] before anything is opened.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, [`StoreError::NoStore`], [`StoreError::ReadOnly`]
    /// or provider failure, or [`StoreError::Replaced`] as
    /// [`EventlogStore::sqlite`] refuses it.
    pub fn sqlite_existing(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let tenant = TenantId::new(tenant)?;
        holds_something(path)?;
        if let Some(denied) = read_only::sqlite_write_denied(path) {
            return Err(read_only::denied(path, &denied));
        }
        let shown = path.display().to_string();
        let named = path.to_string_lossy();
        let store = waiting_out_the_lock(|| {
            runtime.block_on(SqliteEventStore::open_existing(&named, "ekr"))
        })
        .map_err(|error| match error {
            EventLogError::Invalid(message) if message == SQLITE_NO_OWNER_EVENTS => {
                StoreError::NoStore(shown)
            }
            error => error.into(),
        })?;
        Self::at(
            Self::assemble(runtime, store, tenant, ontology.into()),
            path,
        )
    }
    /// Opens an already provisioned SQLite store for a caller that only reads it: as
    /// [`EventlogStore::sqlite_existing`] where this process may write the store, and otherwise
    /// as [`EventlogStore::sqlite_read_only`].
    /// # Errors
    /// What the open it chose reports.
    pub fn sqlite_reading(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        if read_only::sqlite_write_denied(path).is_some() {
            Self::sqlite_read_only(path, tenant, ontology)
        } else {
            Self::sqlite_existing(path, tenant, ontology)
        }
    }
    /// Opens an already provisioned SQLite store read-only: its database is read through a
    /// read-only connection into an image held in memory (`read_only.rs` says how). It writes
    /// nothing at its path, with one exception SQLite makes: a writer that closes during the open
    /// can leave SQLite to create an empty `-wal` beside the database where this process may write
    /// the directory (`read_only.rs` says when). Every write through the store is refused as
    /// [`StoreError::ReadOnly`] and no replay checkpoint is written.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, [`StoreError::NoStore`] or provider failure.
    pub fn sqlite_read_only(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let tenant = TenantId::new(tenant)?;
        holds_something(path)?;
        let signature = read_only::Signature::sqlite(path);
        let image = read_only::sqlite_image(path, "ekr_events")?
            .ok_or_else(|| StoreError::NoStore(path.display().to_string()))?;
        let store = runtime.block_on(SqliteEventStore::from_image("ekr", image))?;
        let mut opened = Self::assemble(runtime, store, tenant, ontology.into());
        opened.read_only = Some(read_only::ReadOnly {
            path: path.to_owned(),
            signature,
            sqlite: true,
            _copy: None,
        });
        Ok(opened)
    }
}
/// [`StoreError::NoStore`] for a path that holds no store of either provider: nothing at the path
/// after following symlinks, an empty directory or an empty file. It reads and writes nothing
/// else; anything else at the path is left to the provider's own open to accept or refuse.
fn holds_something(path: &Path) -> Result<(), StoreError> {
    let empty = match std::fs::metadata(path) {
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
        Ok(found) if found.is_dir() => std::fs::read_dir(path)
            .map_err(|error| StoreError::Backend(error.to_string()))?
            .next()
            .is_none(),
        Ok(found) => found.is_file() && found.len() == 0,
    };
    if empty {
        Err(StoreError::NoStore(path.display().to_string()))
    } else {
        Ok(())
    }
}
/// What `SqliteEventStore::open_existing` reports for a database that holds no `ekr` owner at all
/// (`Inner::require_existing_schema`, eventlog-sqlite `fe8a0a7`: the events table is checked
/// first). `open` creates every owner table in one `BEGIN IMMEDIATE` transaction, so a reader sees
/// either none of them — an empty database, or one a first seed has not committed its tables to
/// yet — or all of them. A database without the events table therefore holds no store.
const SQLITE_NO_OWNER_EVENTS: &str = "SQLite owner table ekr_events is absent";
/// [`StoreError::NoStore`] for a File-provider directory that holds only what the provider's own
/// open-or-create writes before its `manifest.json` lands (`Journal::open_with_creation`,
/// eventlog-file `fe8a0a7`): `writer.lock`, an empty `events.jsonl` and `.write-*` staging files.
/// Such a directory is a store being created, or one whose creation was killed, and holds no
/// history. A directory without a manifest but with anything else is left to the provider, which
/// refuses it as corrupt.
fn holds_a_file_store(path: &Path) -> Result<(), StoreError> {
    let backend = |error: std::io::Error| StoreError::Backend(error.to_string());
    if !std::fs::metadata(path).map_err(backend)?.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).map_err(backend)? {
        let entry = entry.map_err(backend)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let creating = match name.as_ref() {
            "writer.lock" => true,
            "events.jsonl" => entry.metadata().map_err(backend)?.len() == 0,
            staging => staging.starts_with(".write-"),
        };
        if !creating {
            return Ok(());
        }
    }
    Err(StoreError::NoStore(path.display().to_string()))
}
/// How long a SQLite open keeps starting new attempts while another connection holds the lock.
///
/// `SqliteEventStore::open` runs `PRAGMA journal_mode=WAL`, and on a database another process is
/// converting at the same moment that statement returns `database is locked` at once, without
/// consulting the connection's busy handler (measured: 2 of 48 opens by six concurrent processes
/// on a new database). Every other statement in the open is `BEGIN IMMEDIATE` under that handler.
/// Opening is idempotent, so the open is retried; an attempt is started only inside this window.
const SQLITE_OPEN_RETRY_WINDOW: std::time::Duration = std::time::Duration::from_secs(5);
/// The busy handler every provider connection carries: rusqlite 0.40.2 opens each connection
/// with `sqlite3_busy_timeout(db, 5000)` (`inner_connection.rs`), and eventlog does not change it.
/// One attempt can therefore wait this long inside the provider before it reports the lock.
const SQLITE_BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
/// The true bound on how long a SQLite open waits for a held lock before it reports
/// `database is locked`: the last attempt starts inside [`SQLITE_OPEN_RETRY_WINDOW`] and can wait
/// [`SQLITE_BUSY_TIMEOUT`] inside the provider, plus at most one 100 ms pause. About 10 s.
const SQLITE_OPEN_MAX_WAIT: std::time::Duration = SQLITE_OPEN_RETRY_WINDOW
    .saturating_add(SQLITE_BUSY_TIMEOUT)
    .saturating_add(SQLITE_OPEN_MAX_PAUSE);
/// The longest pause between two attempts.
const SQLITE_OPEN_MAX_PAUSE: std::time::Duration = std::time::Duration::from_millis(100);
/// The provider's rendering of `SQLITE_BUSY` (`sqlite3_errstr`).
const SQLITE_BUSY: &str = "database is locked";
/// Retries `open` while it reports a held SQLite lock; see [`SQLITE_OPEN_MAX_WAIT`] for the bound.
fn waiting_out_the_lock<T>(
    mut open: impl FnMut() -> Result<T, EventLogError>,
) -> Result<T, EventLogError> {
    let window = std::time::Instant::now() + SQLITE_OPEN_RETRY_WINDOW;
    let mut pause = std::time::Duration::from_millis(5);
    loop {
        match open() {
            Err(EventLogError::Backend(message))
                if message == SQLITE_BUSY && std::time::Instant::now() + pause < window =>
            {
                std::thread::sleep(pause);
                pause = (pause * 2).min(SQLITE_OPEN_MAX_PAUSE);
            }
            Err(EventLogError::Backend(message)) if message == SQLITE_BUSY => {
                return Err(EventLogError::Backend(format!(
                    "{SQLITE_BUSY}: still held when the retry window closed; a SQLite open waits \
                     at most {SQLITE_OPEN_MAX_WAIT:?}"
                )));
            }
            result => return result,
        }
    }
}
/// What the File provider reports when the history at its root is no longer the one a handle
/// observed: the journal no longer extends what the handle saw (eventlog-file `fe8a0a7`, `lib.rs`,
/// `capture.rs` and `inline_admin.rs`, each as `EventLogError::Backend`). The provider names the
/// condition by no variant of its own, so this is the one place its text is read.
const FILE_DIVERGED: &str = "file history diverged from this handle's observed history";
/// Whether a provider error is its refusal of a diverged history: [`StoreError::Diverged`].
pub(crate) fn diverged(error: &EventLogError) -> bool {
    matches!(error, EventLogError::Backend(message) if message.contains(FILE_DIVERGED))
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
        let tenant = TenantId::new(tenant)?;
        std::fs::create_dir_all(path).map_err(|e| StoreError::Backend(e.to_string()))?;
        let store = runtime.block_on(FileEventStore::open(path))?;
        Ok(Self::assemble(runtime, store, tenant, ontology.into()))
    }
    /// Opens an already provisioned File store, creating no directory, lock or manifest: a path
    /// holding none is refused and left as it was. A store this process may not write — its
    /// directory, `writer.lock`, `events.jsonl` or `blobs` directory — is refused as
    /// [`StoreError::ReadOnly`] before anything is opened.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, [`StoreError::NoStore`], [`StoreError::ReadOnly`]
    /// or provider failure.
    pub fn file_existing(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let tenant = TenantId::new(tenant)?;
        holds_something(path)?;
        holds_a_file_store(path)?;
        if let Some(denied) = read_only::file_write_denied(path) {
            return Err(read_only::denied(path, &denied));
        }
        let store = runtime.block_on(FileEventStore::open_existing(path))?;
        Ok(Self::assemble(runtime, store, tenant, ontology.into()))
    }
    /// Opens an already provisioned File store for a caller that only reads it: as
    /// [`EventlogStore::file_existing`] where this process may write the store, and otherwise as
    /// [`EventlogStore::file_read_only`].
    /// # Errors
    /// What the open it chose reports.
    pub fn file_reading(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        if read_only::file_write_denied(path).is_some() {
            Self::file_read_only(path, tenant, ontology)
        } else {
            Self::file_existing(path, tenant, ontology)
        }
    }
    /// Opens an already provisioned File store read-only, writing nothing at its path: the store
    /// is copied into a private temporary directory under a shared lock and the provider opened
    /// over the copy (`read_only.rs` says how), every write through the store is refused as
    /// [`StoreError::ReadOnly`] and no replay checkpoint is written. The copy is removed when the
    /// store drops.
    /// # Errors
    /// Runtime-context refusal, invalid tenant, [`StoreError::NoStore`] or provider failure.
    pub fn file_read_only(
        path: &Path,
        tenant: &str,
        ontology: impl Into<Option<Ontology>>,
    ) -> Result<Self, StoreError> {
        let runtime = new_runtime()?;
        let tenant = TenantId::new(tenant)?;
        holds_something(path)?;
        holds_a_file_store(path)?;
        let signature = read_only::Signature::file(path);
        let copy = read_only::copy_file_store(path)?;
        let store = runtime.block_on(FileEventStore::open_existing(copy.path()))?;
        let mut opened = Self::assemble(runtime, store, tenant, ontology.into());
        opened.read_only = Some(read_only::ReadOnly {
            path: path.to_owned(),
            signature,
            sqlite: false,
            _copy: Some(copy),
        });
        Ok(opened)
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
    fn assemble(runtime: Runtime, store: S, tenant: TenantId, ontology: Option<Ontology>) -> Self {
        Self {
            runtime: Some(runtime),
            store,
            tenant,
            ontology,
            authority: None,
            verified: std::sync::Mutex::default(),
            log_seen: std::sync::Mutex::default(),
            revisions: std::sync::Mutex::default(),
            checkpoints: true,
            checkpoint_offered: std::sync::atomic::AtomicBool::new(false),
            authorized: std::sync::Mutex::default(),
            pointer: std::sync::Mutex::default(),
            at_path: None,
            read_only: None,
        }
    }
    /// What every read and write through this store does first: refuses a thread entered into an
    /// async runtime, and a SQLite database replaced in place since this handle opened it.
    fn entered(&self) -> Result<(), StoreError> {
        ensure_sync_context()?;
        let Some(at_path) = &self.at_path else {
            return Ok(());
        };
        at_path.check(|position| {
            let page = self.runtime().block_on(self.store.read_feed(
                &self.tenant,
                position.saturating_sub(1),
                1,
            ))?;
            Ok(page
                .events
                .into_iter()
                .next()
                .filter(|event| event.global_seq == position)
                .map(|event| event.event_id))
        })
    }
    /// `opened`, recording the database file at `path` it opened and checking it once.
    fn at(mut opened: Self, path: &Path) -> Result<Self, StoreError> {
        opened.at_path = Some(replaced::AtPath::opened(path, opened.tenant.as_str())?);
        opened.entered()?;
        Ok(opened)
    }
    /// Replays every history read in full from the seed: the retained replay checkpoint is not
    /// offered to the authority. Checkpoints are still written.
    pub fn set_full_replay(&mut self, full: bool) {
        self.checkpoints = !full;
    }
    /// Whether this store was opened read-only ([`EventlogStore::file_read_only`],
    /// [`EventlogStore::sqlite_read_only`]): every write through it is refused.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.read_only.is_some()
    }
    /// Whether this store was opened read-only and the files at its path have changed since it
    /// read them — another process committed, say. A long-lived reader then opens the store
    /// again; a store opened for writing reads the provider itself and is never stale.
    #[must_use]
    pub fn source_changed(&self) -> bool {
        self.read_only
            .as_ref()
            .is_some_and(read_only::ReadOnly::changed)
    }
    fn checkpoint_stream(&self) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            CHECKPOINT_STREAM_TYPE,
            CHECKPOINT_STREAM_ID,
        )?)
    }
    /// The newest retained checkpoint pointer and its stream length.
    fn checkpoint_pointer(&self) -> Result<(Option<CheckpointWritten>, u64), StoreError> {
        let events = self.read_all(&self.checkpoint_stream()?, MAX_READ_LIMIT)?;
        let length = events.len() as u64;
        // A newest pointer this store cannot read names no checkpoint: it is cache data, and the
        // next write appends after it.
        let pointer = events
            .into_iter()
            .last()
            .filter(|last| last.name == CHECKPOINT_WRITTEN && last.schema_version == 1)
            .and_then(|last| serde_json::from_value(last.data).ok());
        Ok((pointer, length))
    }
    /// The head root the authority vouches for without a replay: when the newest checkpoint
    /// pointer records that it verified every occurrence the revision stream holds, the head is
    /// the root the head revision's retained record carries. Reads the revision stream, the
    /// pointer and that one record. `None` whenever anything is missing or disagrees.
    fn checkpointed_head(&self) -> Result<Option<Root>, StoreError> {
        let Ok(authority) = self.authority() else {
            return Ok(None);
        };
        if !self.checkpoints {
            return Ok(None);
        }
        let Ok((Some(pointer), _)) = self.checkpoint_pointer() else {
            return Ok(None);
        };
        let occurrences = self.occurrences(MAX_READ_LIMIT, None)?;
        if pointer.covered != occurrences.len() as u64 {
            return Ok(None);
        }
        let Some(record) = occurrences.iter().rev().find_map(|held| {
            matches!(
                held.event.payload,
                RevisionPayload::Seeded { .. } | RevisionPayload::RevisionCommitted { .. }
            )
            .then_some(held.event.record_hash)
        }) else {
            return Ok(None);
        };
        let mut history = RetainedHistory {
            occurrences,
            objects: BTreeMap::new(),
        };
        if self.load_object(&mut history, record).is_err() {
            return Ok(None);
        }
        Ok(authority
            .checkpointed_head(&history, pointer.binding)
            .unwrap_or(None))
    }
    /// The newest retained checkpoint's bytes, if its blob is still held and hashes to its name.
    fn read_checkpoint(&self) -> Result<Option<Vec<u8>>, StoreError> {
        let (Some(pointer), _) = self.checkpoint_pointer()? else {
            return Ok(None);
        };
        let bytes = self.runtime().block_on(
            self.store
                .get_blob(&self.tenant, &checkpoint_key(pointer.checkpoint_hash)),
        )?;
        Ok(bytes.filter(|bytes| ContentHash::of_bytes(bytes) == pointer.checkpoint_hash))
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
        self.entered()?;
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
        self.read_on(&mut read, limit, &stop)?;
        Ok(read.events)
    }
    /// Reads until `read` is finished, every slice through [`StreamRead::absorb`].
    fn read_on(
        &self,
        read: &mut StreamRead<'_>,
        limit: usize,
        stop: &impl Fn(&RecordedEvent) -> bool,
    ) -> Result<(), StoreError> {
        while !read.finished {
            match read.stream.stream_type() {
                REVISION_STREAM_TYPE => {
                    crate::verified::count_stream_read(|reads| reads.revision += 1)
                }
                CHECKPOINT_STREAM_TYPE => {
                    crate::verified::count_stream_read(|reads| reads.checkpoint += 1);
                }
                OBJECT_STREAM_TYPE => {
                    crate::verified::count_stream_read(|reads| reads.object += 1);
                }
                _ => {}
            }
            let slice =
                self.runtime()
                    .block_on(self.store.read_stream(read.stream, read.after, limit))?;
            read.absorb(&self.tenant, slice, stop)?;
        }
        Ok(())
    }
    /// The revision stream from its start through the occurrence that made `selected`, or all of
    /// it: what this handle already holds, and from the provider only what follows that.
    ///
    /// Each occurrence is checked once, when it is first read, by [`StreamRead::absorb`] and
    /// [`HeldRevisions::accept`] against every occurrence before it, which are the checks a read
    /// of the whole stream applies. Held occurrences are not fetched again: the stream is
    /// append-only. An occurrence that fails a check is not held, so every later read that
    /// reaches it refuses it again.
    ///
    /// Every read, including one answered wholly from the held prefix, first asks the provider for
    /// the last held occurrence again ([`Self::still_holds`]); a read that goes on past the held
    /// prefix asks for it in the call that reads on ([`Self::still_holds_reading_on`]). That is
    /// the provider call through which it refuses a history that diverged from what this handle
    /// observed (the file provider's own check), so a read at an earlier revision refuses where a
    /// head read does. A provider that answers with a different occurrence there no longer has the
    /// held prefix, and the handle drops everything it holds and reads from the start, as a new
    /// handle would.
    fn occurrences(
        &self,
        limit: usize,
        selected: Option<RevisionNumber>,
    ) -> Result<Vec<RecordedOccurrence>, StoreError> {
        let selects =
            |event: &RevisionEvent| selected.is_some_and(|revision| makes(event, revision));
        let mut held = self
            .revisions
            .lock()
            .map_err(|_| StoreError::Document("held-revisions-poisoned".into()))?;
        let stream = self.revision_stream()?;
        let found = |held: &HeldRevisions| {
            held.occurrences
                .iter()
                .position(|occurrence| selects(&occurrence.event))
        };
        // When the read goes on past what is held, the provider call that confirms the last held
        // occurrence also returns what follows it; otherwise it is made on its own.
        let mut after_held = None;
        if held.occurrences.is_empty() || found(&held).is_some() {
            self.confirm_held(&stream, &mut held)?;
        } else {
            match self.still_holds_reading_on(&stream, &held, limit)? {
                Some(rest) => after_held = Some(rest),
                None => self.forget_everything(&mut held),
            }
        }
        if found(&held).is_none() {
            let events = {
                let stop = |record: &RecordedEvent| {
                    selected.is_some()
                        && serde_json::from_value::<RevisionEvent>(record.data.clone())
                            .is_ok_and(|event| selects(&event))
                };
                let mut read =
                    StreamRead::resumed(&stream, held.occurrences.len() as u64, &held.native_ids);
                // An empty slice that is not the end of the stream carries no cursor past the
                // held prefix; the read then goes on from the prefix itself.
                if let Some(rest) =
                    after_held.filter(|rest| !rest.events.is_empty() || rest.end_of_stream)
                {
                    read.absorb(&self.tenant, rest, &stop)?;
                }
                self.read_on(&mut read, limit, &stop)?;
                read.events
            };
            for recorded in events {
                held.accept(recorded)?;
            }
        }
        let end = found(&held).map_or(held.occurrences.len(), |at| at + 1);
        Ok(held.occurrences[..end].to_vec())
    }
    /// Asks the provider whether this handle's held state still describes its store, before
    /// anything held answers a read: the provider's own refusal of a diverged history is returned
    /// as it is, and a store that no longer has the held prefix makes the handle drop everything
    /// it holds, objects included.
    fn confirm_held(&self, stream: &StreamId, held: &mut HeldRevisions) -> Result<(), StoreError> {
        if !self.still_holds(stream, held)? {
            self.forget_everything(held);
        }
        Ok(())
    }
    /// Whether the provider still has the last occurrence `held` holds, as it was read: one
    /// single-event read. A provider refusal is returned as it is.
    ///
    /// Made even when no occurrence is held but objects are, because the call is what gives the
    /// provider the chance to refuse. A handle that holds nothing at all makes none: nothing it
    /// holds could answer, and its first read is the provider's own.
    fn still_holds(&self, stream: &StreamId, held: &HeldRevisions) -> Result<bool, StoreError> {
        let holds_objects = self.verified.lock().map_or(true, |memo| !memo.is_empty());
        if held.occurrences.is_empty() && !holds_objects {
            return Ok(true);
        }
        let after = held.occurrences.last().map_or(0, |last| last.version - 1);
        crate::verified::count_stream_read(|reads| reads.revision += 1);
        let slice = self
            .runtime()
            .block_on(self.store.read_stream(stream, after, 1))?;
        let Some(last) = held.occurrences.last() else {
            return Ok(true);
        };
        Ok(slice
            .events
            .first()
            .is_some_and(|event| self.is_held(stream, last, event)))
    }
    /// Whether the provider's `event` is the held occurrence `last`, as it was read.
    fn is_held(&self, stream: &StreamId, last: &RecordedOccurrence, event: &RecordedEvent) -> bool {
        !event.is_redacted()
            && event.version == last.version
            && event.event_id == last.provider_event_id
            && event.tenant == self.tenant
            && event.stream_type == stream.stream_type()
            && event.stream_id == stream.stream_id()
    }
    /// [`Self::still_holds`] for a handle that holds at least one occurrence, in the same provider
    /// call as the first slice of what follows it: one read from the last held occurrence on,
    /// `limit` events long. The rest of that slice, after the confirmed occurrence, when the
    /// provider still has it as it was read; `None` when it does not. A provider refusal is
    /// returned as it is.
    fn still_holds_reading_on(
        &self,
        stream: &StreamId,
        held: &HeldRevisions,
        limit: usize,
    ) -> Result<Option<StreamSlice>, StoreError> {
        let Some(last) = held.occurrences.last() else {
            return Ok(None);
        };
        crate::verified::count_stream_read(|reads| reads.revision += 1);
        let mut slice =
            self.runtime()
                .block_on(self.store.read_stream(stream, last.version - 1, limit))?;
        if !slice
            .events
            .first()
            .is_some_and(|event| self.is_held(stream, last, event))
        {
            return Ok(None);
        }
        slice.events.remove(0);
        Ok(Some(slice))
    }
    fn object(&self, hash: ContentHash) -> Result<Option<RetainedObject>, StoreError> {
        Ok(self
            .object_versioned(hash)?
            .map(|(CheckedObject(object), _)| object))
    }
    fn object_versioned(
        &self,
        hash: ContentHash,
    ) -> Result<Option<(CheckedObject, u64)>, StoreError> {
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
    ///
    /// An object this handle already verified is not fetched again, unless its held class is below
    /// the strongest and its stream has moved since this handle verified it: a raise appends an
    /// event to that stream, and so does any operation that withdraws retained bytes
    /// (`systems/ekr/domains/store.yaml`, held bytes), so the handle first looks through the log
    /// for such events ([`Self::look_through_log`]), and a moved object's stream is read and
    /// checked again in its place in the batch, with its held bytes standing in for its blob. A
    /// stream is judged before the bytes ([`retained_object`]), so a moved stream this store does
    /// not read is refused as a fresh handle refuses it. A class is never refused from a stale
    /// memo.
    fn required(&self, hashes: &[ContentHash]) -> Result<Vec<RetainedObject>, StoreError> {
        let mut known = self.verified_objects(hashes)?;
        let below: Vec<ContentHash> = known
            .iter()
            .filter(|(_, object)| object.metadata.storage_class != StorageClass::Canonical)
            .map(|(hash, _)| *hash)
            .collect();
        let as_of = self.look_through_log(!below.is_empty())?;
        let raisable: BTreeMap<ContentHash, RetainedObject> = self
            .moved(&below)?
            .into_iter()
            .filter_map(|hash| known.remove(&hash).map(|object| (hash, object)))
            .collect();
        let unknown: Vec<ContentHash> = hashes
            .iter()
            .filter(|hash| !known.contains_key(hash))
            .copied()
            .collect();
        let (read, refusal) = self.read_required(&unknown, raisable)?;
        let (refreshed, read): (Vec<_>, Vec<_>) = read.into_iter().partition(|(_, _, held)| *held);
        let read: Vec<_> = read
            .into_iter()
            .map(|(hash, object, _)| (hash, object))
            .collect();
        let refreshed: Vec<_> = refreshed
            .into_iter()
            .map(|(hash, object, _)| (hash, object))
            .collect();
        let read = self.remember_verified(read, as_of)?;
        self.refresh_verified(&refreshed, as_of)?;
        let mut read = read
            .into_iter()
            .chain(refreshed)
            .map(|(hash, CheckedObject(object))| (hash, object))
            .collect::<BTreeMap<_, _>>();
        let mut objects = Vec::with_capacity(hashes.len());
        for hash in hashes {
            match known.remove(hash).or_else(|| read.remove(hash)) {
                Some(object) => objects.push(object),
                None => break,
            }
        }
        match refusal {
            Some(error) => Err(error),
            None => Ok(objects),
        }
    }
    /// [`Self::required`] for objects this handle has not verified yet, and for those in `held`
    /// whose class may have been raised: their streams in one batch, then the blobs of those not in
    /// `held` in one batch, checked in order up to the first refusal. Each result says whether it
    /// was held.
    #[allow(clippy::type_complexity)]
    fn read_required(
        &self,
        hashes: &[ContentHash],
        mut held: BTreeMap<ContentHash, RetainedObject>,
    ) -> Result<(Vec<(ContentHash, CheckedObject, bool)>, Option<StoreError>), StoreError> {
        if hashes.is_empty() {
            return Ok((Vec::new(), None));
        }
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
        let reads: Vec<Read> = stored
            .iter()
            .filter(|(hash, _, _)| !held.contains_key(hash))
            .map(|(hash, _, _)| Read::Blob {
                tenant: self.tenant.clone(),
                digest: hash.to_hex(),
            })
            .collect();
        let mut blobs = if reads.is_empty() {
            Vec::new()
        } else {
            self.batch(&reads)?
        }
        .into_iter();
        for (hash, events, meta) in stored {
            let (checked, was_held) = match held.remove(&hash) {
                // Bytes this handle verified against `hash` when it first read them.
                Some(object) => (
                    checked_object(hash, &events, meta, object.bytes, |_| true)?.0,
                    true,
                ),
                None => {
                    let Some(ReadResult::Blob(blob)) = blobs.next() else {
                        return Err(StoreError::Document("batch-read-disagrees".into()));
                    };
                    (retained_object(hash, &events, meta, blob)?.0, false)
                }
            };
            objects.push((hash, checked, was_held));
        }
        Ok((objects, refusal))
    }
    /// The objects of `hashes` this handle already read and verified.
    fn verified_objects(
        &self,
        hashes: &[ContentHash],
    ) -> Result<BTreeMap<ContentHash, RetainedObject>, StoreError> {
        let held = self
            .verified
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        Ok(hashes
            .iter()
            .filter_map(|hash| held.get(hash).map(|object| (*hash, object.retained())))
            .collect())
    }
    /// Those of `hashes` this handle holds whose streams may have moved since it verified them.
    fn moved(&self, hashes: &[ContentHash]) -> Result<Vec<ContentHash>, StoreError> {
        let held = self
            .verified
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        Ok(hashes
            .iter()
            .filter(|hash| held.get(hash).is_some_and(|object| object.moved))
            .copied()
            .collect())
    }
    /// The log position at or after which the objects this handle verifies next are verified.
    ///
    /// When `look`, it first looks through the tenant log from the position this handle has
    /// looked through to the log's end, marks every held object whose stream has an event there
    /// as moved, and moves the position to that end: one provider read when the log has not
    /// moved, and one per page of what it has. A log that cannot be read that way, or reads back
    /// out of order, marks every held object moved, so each is read again as it was before this
    /// handle looked at the log. A handle that holds no object yet starts from the newest
    /// revision occurrence it has read, which the log had reached before anything it verifies
    /// next was read.
    fn look_through_log(&self, look: bool) -> Result<u64, StoreError> {
        let start = self
            .revisions
            .lock()
            .map_err(|_| StoreError::Document("held-revisions-poisoned".into()))?
            .position;
        let mut seen = self
            .log_seen
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        let Some(after) = *seen else {
            *seen = Some(start);
            return Ok(start);
        };
        if !look {
            return Ok(after);
        }
        let found = self.object_streams_after(after);
        let mut held = self
            .verified
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        match found {
            Some((end, streams)) => {
                for hash in streams {
                    if let Some(object) = held.get_mut(&hash) {
                        object.moved = true;
                    }
                }
                *seen = Some(end);
                Ok(end)
            }
            None => {
                for object in held.values_mut() {
                    object.moved = true;
                }
                Ok(after)
            }
        }
    }
    /// The last position of the tenant log after `after`, or `after` when there is none, and every
    /// object stream with an event after `after`. `None` when the provider fails or its feed does
    /// not ascend.
    fn object_streams_after(&self, after: u64) -> Option<(u64, BTreeSet<ContentHash>)> {
        let mut streams = BTreeSet::new();
        let mut end = after;
        loop {
            crate::verified::count_stream_read(|reads| reads.feed += 1);
            let page = self
                .runtime()
                .block_on(self.store.read_feed(&self.tenant, end, MAX_READ_LIMIT))
                .ok()?;
            for event in &page.events {
                if event.tenant != self.tenant || event.global_seq <= end {
                    return None;
                }
                end = event.global_seq;
                if event.stream_type == OBJECT_STREAM_TYPE {
                    if let Ok(hash) = event.stream_id.parse::<ContentHash>() {
                        streams.insert(hash);
                    }
                }
            }
            if !page.has_more {
                return Some((end, streams));
            }
            if page.events.is_empty() {
                return None;
            }
        }
    }
    /// Keeps objects [`retained_object`] checked, and registers their bytes as verified for the
    /// process while this handle keeps them. Returns them holding the kept bytes, so that the read
    /// that loaded them shares them with every later read, as later reads share them.
    ///
    /// `as_of` is what [`Self::look_through_log`] returned before they were read. When this handle
    /// has looked further through the log since, an event it passed over may be on one of their
    /// streams, so they are kept as moved.
    fn remember_verified(
        &self,
        read: Vec<(ContentHash, CheckedObject)>,
        as_of: u64,
    ) -> Result<Vec<(ContentHash, CheckedObject)>, StoreError> {
        let seen = self
            .log_seen
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        let moved = *seen != Some(as_of);
        let mut held = self
            .verified
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        Ok(read
            .into_iter()
            .map(|(hash, CheckedObject(mut object))| {
                object.bytes = crate::verified::register(hash, object.bytes);
                held.insert(
                    hash,
                    HeldObject {
                        metadata: object.metadata.clone(),
                        bytes: Arc::clone(&object.bytes),
                        moved,
                    },
                );
                (hash, CheckedObject(object))
            })
            .collect())
    }
    /// Records the retention class a stream re-read found for objects this handle holds, read
    /// after [`Self::look_through_log`] returned `as_of`, as [`Self::remember_verified`] does.
    fn refresh_verified(
        &self,
        read: &[(ContentHash, CheckedObject)],
        as_of: u64,
    ) -> Result<(), StoreError> {
        let seen = self
            .log_seen
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        let moved = *seen != Some(as_of);
        let mut held = self
            .verified
            .lock()
            .map_err(|_| StoreError::Document("verified-object-memo-poisoned".into()))?;
        for (hash, CheckedObject(object)) in read {
            if let Some(kept) = held.get_mut(hash) {
                kept.metadata = object.metadata.clone();
                kept.moved = moved;
            }
        }
        Ok(())
    }
    /// Drops everything this handle holds: the revision prefix and every verified object.
    fn forget_everything(&self, revisions: &mut HeldRevisions) {
        *revisions = HeldRevisions::default();
        let seen = self.log_seen.lock();
        if let Ok(mut held) = self.verified.lock() {
            held.clear();
        }
        if let Ok(mut seen) = seen {
            *seen = None;
        }
    }
    /// Forgets the verified objects a write of this handle may have moved: their retention.
    fn forget_verified(&self, digests: impl IntoIterator<Item = String>) {
        if let Ok(mut held) = self.verified.lock() {
            for digest in digests {
                if let Ok(hash) = digest.parse::<ContentHash>() {
                    held.remove(&hash);
                }
            }
        }
    }
    /// One provider batch, answered read for read.
    fn batch(&self, reads: &[Read]) -> Result<Vec<ReadResult>, StoreError> {
        let objects = reads
            .iter()
            .filter(|read| {
                matches!(read, Read::Stream { stream, .. } if stream.stream_type() == OBJECT_STREAM_TYPE)
            })
            .count() as u64;
        crate::verified::count_stream_read(|counted| counted.object += objects);
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
            if selected.is_none() {
                self.offer_checkpoint(&history)?;
            }
            let required = required_objects(&history)?;
            self.load_objects(&mut history, required)?;
            if let Ok(authority) = self.authority() {
                let wanted = authority.objects_if_held(&history)?;
                self.load_present(&mut history, wanted)?;
            }
        }
        Ok(history)
    }
    /// Everything the authority asks of `history` beyond its records and seed envelope: its
    /// required objects, then each object it reads if held that the store holds.
    fn load_authority_objects(&self, history: &mut RetainedHistory) -> Result<(), StoreError> {
        let authority = self.authority()?;
        let required = authority.required_objects(history)?;
        self.load_objects(history, required)?;
        let wanted = authority.objects_if_held(history)?;
        self.load_present(history, wanted)
    }
    /// Loads each object of `hashes` the history does not hold yet and the store holds an object
    /// for, as [`Self::load_objects`] loads it; one with no object stream is left out.
    fn load_present(
        &self,
        history: &mut RetainedHistory,
        hashes: impl IntoIterator<Item = ContentHash>,
    ) -> Result<(), StoreError> {
        let wanted: Vec<ContentHash> = hashes
            .into_iter()
            .filter(|hash| !history.objects.contains_key(hash))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if wanted.is_empty() {
            return Ok(());
        }
        let held = self.verified_objects(&wanted)?;
        let unknown: Vec<ContentHash> = wanted
            .iter()
            .filter(|hash| !held.contains_key(hash))
            .copied()
            .collect();
        let mut present: Vec<ContentHash> = held.into_keys().collect();
        if !unknown.is_empty() {
            let reads = unknown
                .iter()
                .map(|hash| {
                    Ok(Read::Stream {
                        stream: self.object_stream(*hash)?,
                        after_version: 0,
                        limit: 1,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            for (hash, slice) in unknown.iter().zip(self.batch(&reads)?) {
                let ReadResult::Stream(slice) = slice else {
                    return Err(StoreError::Document("batch-read-disagrees".into()));
                };
                if !slice.events.is_empty() {
                    present.push(*hash);
                }
            }
        }
        self.load_objects(history, present)
    }
    /// Offers the retained replay checkpoint to the authority once per handle, before the first
    /// head replay. A checkpoint the authority refuses, or one that cannot be read, is ignored:
    /// the history is then replayed in full, exactly as if there were none.
    fn offer_checkpoint(&self, history: &RetainedHistory) -> Result<(), StoreError> {
        use std::sync::atomic::Ordering;
        let Ok(authority) = self.authority() else {
            return Ok(());
        };
        if !self.checkpoints || self.checkpoint_offered.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Ok(Some(checkpoint)) = self.read_checkpoint() {
            let _ = authority.restore(history, &checkpoint);
        }
        Ok(())
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
        let (expected, event) = if let Some((CheckedObject(held), version)) =
            self.object_versioned(hash)?
        {
            if *held.bytes != object.bytes {
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
        // Every publication, object, preparation and checkpoint write reaches the provider here:
        // on a store opened read-only it is refused before anything is written, to the copy too.
        if let Some(read_only) = &self.read_only {
            return Err(read_only.refusal());
        }
        // Retry an uncertain outcome with exactly the same group and bindings, never a new identity
        // or an object cleanup. Native receipt lookup precedes blob revalidation on such a retry.
        self.forget_verified(request.blobs.iter().map(|blob| blob.digest.clone()));
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
        self.entered()?;
        let history = self.load_history(1, Some(revision))?;
        if !history.occurrences.is_empty() {
            self.authority()?
                .verify(&history, self.ontology.as_ref(), Some(revision))?;
        }
        Ok(history)
    }
    fn history(&self) -> Result<RetainedHistory, StoreError> {
        self.entered()?;
        let history = self.load_history(MAX_READ_LIMIT, None)?;
        if !history.occurrences.is_empty() {
            self.authority()?
                .verify(&history, self.ontology.as_ref(), None)?;
        }
        Ok(history)
    }
    fn publish(&self, publication: &Publication) -> Result<Appended, StoreError> {
        self.entered()?;
        for attempt in 0..16 {
            let mut history = self.load_history(MAX_READ_LIMIT, None)?;
            self.authority()?
                .verify(&history, self.ontology.as_ref(), None)?;
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
                    bytes: Arc::new(object.bytes.clone()),
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
            self.load_authority_objects(&mut history)?;
            self.authority()?
                .verify(&history, self.ontology.as_ref(), None)?;
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
        self.entered()?;
        Ok(self
            .admitted(None, MAX_READ_LIMIT)?
            .ok_or(StoreError::NotSeeded)?
            .graph)
    }
    fn head(&self) -> Result<Option<Root>, StoreError> {
        self.entered()?;
        if let Some(root) = self.checkpointed_head()? {
            return Ok(Some(root));
        }
        // No pointer names every occurrence — after a proposal or a validation none does (design
        // § 99) — so the head is replayed, and only its root is asked for.
        let history = self.load_history(MAX_READ_LIMIT, None)?;
        if history.occurrences.is_empty() {
            return Ok(None);
        }
        self.authority()?
            .replay_root(&history, self.ontology.as_ref(), None)
    }
    fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        self.entered()?;
        Ok(self
            .admitted(Some(revision), 1)?
            .ok_or(StoreError::NotSeeded)?
            .graph)
    }
    /// Appends the pointer event and, with `checkpoint`, publishes the checkpoint blob in the same
    /// atomic group, then deletes the blob of the checkpoint it replaces: one checkpoint is
    /// retained at a time. Without `checkpoint` the pointer names the retained one again.
    ///
    /// The store neither reads nor judges what a checkpoint or a binding says; the kernel admits
    /// one or not when it next replays. A pointer that loses a race with another writer is
    /// dropped, which leaves the store as it was and is not an error: a checkpoint is a cache.
    /// Whether the pointer stands afterwards is the answer, so that a writer knows whether its
    /// checkpoint is the retained one.
    fn checkpoint_covered(&self) -> Result<Option<u64>, StoreError> {
        self.entered()?;
        Ok(self.checkpoint_pointer()?.0.map(|pointer| pointer.covered))
    }
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        checkpoint: Option<&[u8]>,
    ) -> Result<bool, StoreError> {
        self.entered()?;
        // A checkpoint is a cache of verified work: a store opened read-only keeps none, and a
        // read on it is answered exactly as without one.
        if self.read_only.is_some() {
            return Ok(false);
        }
        // This handle's own last append names the newest pointer and the stream's length, unless
        // another handle has written since. Only an append at that length proves the record still
        // holds: the conditional append succeeds. Any other outcome from the record — a lost
        // append, or nothing to write, which is what the record says and may not be what the
        // stream holds — is decided again from the stream as read, as a handle without the record
        // decides it.
        let remembered = self.pointer.lock().ok().and_then(|mut held| held.take());
        let written = match remembered {
            Some((previous, length)) => {
                match self.append_pointer(covered, binding, checkpoint, previous, length)? {
                    PointerWrite::Appended(appended) => PointerWrite::Appended(appended),
                    PointerWrite::Current | PointerWrite::Nothing => {
                        let (previous, length) = self.checkpoint_pointer()?;
                        self.append_pointer(covered, binding, checkpoint, previous, length)?
                    }
                }
            }
            None => {
                let (previous, length) = self.checkpoint_pointer()?;
                self.append_pointer(covered, binding, checkpoint, previous, length)?
            }
        };
        Ok(match written {
            PointerWrite::Appended(appended) => {
                if let Ok(mut held) = self.pointer.lock() {
                    *held = Some(appended);
                }
                true
            }
            PointerWrite::Current => true,
            PointerWrite::Nothing => false,
        })
    }
}
/// What one conditional pointer append did.
enum PointerWrite {
    /// Appended: the pointer and the stream's length after it.
    Appended((Option<CheckpointWritten>, u64)),
    /// Nothing appended, because the newest pointer already is this one.
    Current,
    /// Nothing appended: no retained checkpoint to name, or the append lost to another writer.
    Nothing,
}
impl<S: AtomicBlobEventStore> EventlogStore<S> {
    /// [`RevisionLog::write_checkpoint`] after `previous`, the newest pointer of a stream `length`
    /// long: the pointer it appended and the stream's length after it, or why it appended none.
    fn append_pointer(
        &self,
        covered: u64,
        binding: ContentHash,
        checkpoint: Option<&[u8]>,
        previous: Option<CheckpointWritten>,
        length: u64,
    ) -> Result<PointerWrite, StoreError> {
        let hash = match (checkpoint, &previous) {
            (Some(bytes), _) => ContentHash::of_bytes(bytes),
            (None, Some(previous)) => previous.checkpoint_hash,
            (None, None) => return Ok(PointerWrite::Nothing),
        };
        if previous.as_ref().is_some_and(|previous| {
            previous.checkpoint_hash == hash
                && previous.covered == covered
                && previous.binding == binding
        }) {
            return Ok(PointerWrite::Current);
        }
        let pointer = CheckpointWritten {
            checkpoint_hash: hash,
            covered,
            binding,
        };
        let replaced = previous
            .map(|previous| previous.checkpoint_hash)
            .filter(|previous| *previous != hash && checkpoint.is_some());
        let request = BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends: vec![StreamAppend {
                    stream: self.checkpoint_stream()?,
                    expected: if length == 0 {
                        Expected::NoStream
                    } else {
                        Expected::Exact(length)
                    },
                    events: vec![NewEvent::new(
                        CHECKPOINT_WRITTEN,
                        1,
                        serde_json::to_value(&pointer).map_err(json_error)?,
                    )?],
                }],
                meta: envelope(
                    &format!("ekr.checkpoint.{length}.{covered}.{binding}.{hash}"),
                    ContentHash::of_bytes(format!("{covered}.{binding}.{hash}").as_bytes())
                        .to_hex(),
                ),
            },
            blobs: checkpoint
                .map(|bytes| BlobWrite {
                    digest: checkpoint_key(hash),
                    bytes: bytes.to_vec(),
                })
                .into_iter()
                .collect(),
        };
        let written = if request.blobs.is_empty() {
            self.runtime()
                .block_on(eventlog_core::AtomicEventStore::append_group(
                    &self.store,
                    &request.group,
                ))
                .map(|_| ())
                .map_err(StoreError::from)
        } else {
            self.atomic(&request).map(|_| ())
        };
        match written {
            Ok(()) => {}
            Err(StoreError::Conflict) => return Ok(PointerWrite::Nothing),
            Err(error) => return Err(error),
        }
        if let Some(replaced) = replaced {
            self.runtime().block_on(
                self.store
                    .delete_blob(&self.tenant, &checkpoint_key(replaced)),
            )?;
        }
        Ok(PointerWrite::Appended((Some(pointer), length + 1)))
    }
}
impl<S: AtomicBlobEventStore> Initialize for EventlogStore<S> {
    fn initialize(&self, publication: &Publication) -> Result<Appended, StoreError> {
        self.entered()?;
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
        self.entered()?;
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
    /// An object this handle already verified is answered from what it holds, once the provider
    /// has confirmed the handle's held state and the memo's stream check has passed, as a history
    /// read does; any other is read and checked, and then held.
    fn get(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.entered()?;
        {
            let mut held = self
                .revisions
                .lock()
                .map_err(|_| StoreError::Document("held-revisions-poisoned".into()))?;
            self.confirm_held(&self.revision_stream()?, &mut held)?;
        }
        // A held object is answered after the memo's stream check, as a history load answers it:
        // a withdrawal recorded on its stream since it was verified is seen here too.
        if self.verified_objects(&[*hash])?.contains_key(hash) {
            return Ok(self
                .required(&[*hash])?
                .into_iter()
                .next()
                .map(|held| held.bytes.to_vec()));
        }
        let as_of = self.look_through_log(false)?;
        let Some((checked, _)) = self.object_versioned(*hash)? else {
            return Ok(None);
        };
        let read = self.remember_verified(vec![(*hash, checked)], as_of)?;
        Ok(read
            .into_iter()
            .next()
            .map(|(_, CheckedObject(object))| object.bytes.to_vec()))
    }
}
/// A stream read in progress: what it has accepted so far and where the next slice starts.
///
/// Every slice, whether it came from its own provider call or from a batch, is accepted through
/// [`StreamRead::absorb`], so a batched read and a single one refuse the same envelopes.
struct StreamRead<'s> {
    stream: &'s StreamId,
    events: Vec<RecordedEvent>,
    /// Provider identities of the events before `base`, which a new event must not repeat.
    held_ids: Option<&'s BTreeSet<String>>,
    native_ids: BTreeSet<String>,
    /// How many events of the stream were accepted before this read began.
    base: u64,
    after: u64,
    finished: bool,
}
impl<'s> StreamRead<'s> {
    fn new(stream: &'s StreamId) -> Self {
        Self {
            stream,
            events: Vec::new(),
            held_ids: None,
            native_ids: BTreeSet::new(),
            base: 0,
            after: 0,
            finished: false,
        }
    }
    /// A read that continues after the first `base` events of `stream`, already accepted with the
    /// provider identities `held_ids`.
    fn resumed(stream: &'s StreamId, base: u64, held_ids: &'s BTreeSet<String>) -> Self {
        Self {
            held_ids: Some(held_ids),
            base,
            after: base,
            ..Self::new(stream)
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
                || event.version != self.base + self.events.len() as u64 + 1
                || &event.tenant != tenant
                || event.stream_type != self.stream.stream_type()
                || event.stream_id != self.stream.stream_id()
                || self
                    .held_ids
                    .is_some_and(|held| held.contains(&event.event_id))
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
/// The stream is judged first: every later event must be a retention raise that only strengthens
/// the class. Then the blob must be present and hash to the address with the recorded length.
/// Returns the object at its strongest class and the stream's length. A stream that records what
/// this store does not read — a withdrawal, say — is refused before the blob is looked at, so a
/// handle holding the bytes and one that reads them refuse it alike.
fn retained_object(
    hash: ContentHash,
    events: &[RecordedEvent],
    meta: ObjectMetadata,
    blob: Option<Vec<u8>>,
) -> Result<(CheckedObject, u64), StoreError> {
    let class = recorded_class(hash, events, &meta)?;
    let bytes =
        blob.ok_or_else(|| StoreError::Document("object-integrity: native blob missing".into()))?;
    crate::verified::count(|work| work.blobs_read += 1);
    checked_bytes(hash, events, meta, class, bytes, |bytes| {
        crate::verified::addresses(hash, bytes)
    })
}
/// [`retained_object`]'s checks of a stream, its metadata and `bytes`, however the bytes were
/// obtained; `addressed` says whether they are the payload `hash` addresses. Bytes read from the
/// provider move into the shared allocation every later read holds; held bytes stay where they are.
fn checked_object(
    hash: ContentHash,
    events: &[RecordedEvent],
    meta: ObjectMetadata,
    bytes: impl Into<Arc<Vec<u8>>>,
    addressed: impl FnOnce(&[u8]) -> bool,
) -> Result<(CheckedObject, u64), StoreError> {
    let class = recorded_class(hash, events, &meta)?;
    checked_bytes(hash, events, meta, class, bytes, addressed)
}
/// The strongest class an object's stream records: every event after its first must be a
/// retention raise that only strengthens the class.
fn recorded_class(
    hash: ContentHash,
    events: &[RecordedEvent],
    meta: &ObjectMetadata,
) -> Result<StorageClass, StoreError> {
    let mut class = meta.storage_class;
    for event in events.get(1..).unwrap_or_default() {
        if event.name != OBJECT_RETENTION_RAISED || event.schema_version != 1 {
            return Err(StoreError::Document(
                "unsupported-retention-envelope".into(),
            ));
        }
        let raised: RetentionRaised = serde_json::from_value(event.data.clone())
            .map_err(|e| StoreError::Document(e.to_string()))?;
        if raised.content_hash != hash
            || raised.to.retention_rank() <= raised.from.retention_rank()
            || raised.from.retention_rank() > class.retention_rank()
        {
            return Err(StoreError::Document(
                "object-integrity: invalid retention metadata".into(),
            ));
        }
        class = class.strongest(raised.to);
    }
    Ok(class)
}
/// The object at `class`, once `bytes` are the payload its metadata records at `hash`.
fn checked_bytes(
    hash: ContentHash,
    events: &[RecordedEvent],
    meta: ObjectMetadata,
    class: StorageClass,
    bytes: impl Into<Arc<Vec<u8>>>,
    addressed: impl FnOnce(&[u8]) -> bool,
) -> Result<(CheckedObject, u64), StoreError> {
    let bytes = bytes.into();
    if meta.content_hash != hash || meta.byte_len != bytes.len() as u64 || !addressed(&bytes) {
        return Err(StoreError::Document(
            "object-integrity: address or byte length disagrees".into(),
        ));
    }
    Ok((
        CheckedObject(RetainedObject {
            metadata: StoredObject {
                content_hash: hash,
                storage_class: class,
                byte_len: meta.byte_len,
                stored_at: meta.stored_at,
            },
            bytes,
        }),
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

#[cfg(test)]
mod lock_wait {
    use super::{
        waiting_out_the_lock, EventLogError, SQLITE_BUSY, SQLITE_BUSY_TIMEOUT,
        SQLITE_OPEN_MAX_WAIT, SQLITE_OPEN_RETRY_WINDOW,
    };
    use std::time::Instant;

    /// A lock that is never released: every attempt starts inside the retry window, the last
    /// report comes back inside it, and so an attempt that also waits out the provider's own busy
    /// handler ends inside [`SQLITE_OPEN_MAX_WAIT`]. Any other error is returned at once.
    #[test]
    fn a_held_lock_is_retried_only_inside_the_window_and_the_bound_is_its_sum() {
        let start = Instant::now();
        let mut attempts = Vec::new();
        let result: Result<(), _> = waiting_out_the_lock(|| {
            attempts.push(start.elapsed());
            Err(EventLogError::Backend(SQLITE_BUSY.to_owned()))
        });
        let elapsed = start.elapsed();
        let Err(EventLogError::Backend(reported)) = result else {
            panic!("{result:?}")
        };
        assert!(
            reported.starts_with(SQLITE_BUSY)
                && reported.ends_with(&format!("{SQLITE_OPEN_MAX_WAIT:?}")),
            "{reported}"
        );
        assert!(attempts.len() > 1, "{attempts:?}");
        let last = *attempts.last().unwrap();
        assert!(last < SQLITE_OPEN_RETRY_WINDOW, "{last:?}");
        assert!(elapsed < SQLITE_OPEN_RETRY_WINDOW, "{elapsed:?}");
        assert!(last + SQLITE_BUSY_TIMEOUT < SQLITE_OPEN_MAX_WAIT);

        let mut calls = 0;
        let other: Result<(), _> = waiting_out_the_lock(|| {
            calls += 1;
            Err(EventLogError::Backend("disk I/O error".to_owned()))
        });
        assert!(other.is_err());
        assert_eq!(calls, 1, "only the lock is retried");
    }
}
