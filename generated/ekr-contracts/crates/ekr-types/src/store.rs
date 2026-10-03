// generated from ekr v1
// model digest 147770d5ae58c39107c86a474a7d5a2a01b4a84701f3864bbd0d4874663d73ad
// contract digest b79e26b7335246ec2103e09912236dba575c2b8e1118ee2a0238152e64c2b4e6
// do not edit: regenerate with `ess synthesize`

//! Store — `ekr.store`.
//!
//! Persistence through eventlog. A committed revision is an event; the graph is a fold; snapshots are eventlog snapshots; bytes are content-addressed and carry a storage class that decides their retention. Design § 34, § 37, § 57. Reclamation (§ 38–39) arrives in P6 with the commands that move an object toward deletion.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// NativeBlobWrite — `ekr.store.NativeBlobWrite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBlobWrite {
    /// `digest` — `String`.
    pub digest: String,
    /// `bytes` — `Bytes`.
    pub bytes: Vec<u8>,
}

/// NativeClaim — `ekr.store.NativeClaim`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClaim {
    /// `scope` — `String`.
    pub scope: String,
    /// `key` — `String`.
    pub key: String,
    /// `digest` — `String`.
    pub digest: String,
}

/// NativeCommandMeta — `ekr.store.NativeCommandMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCommandMeta {
    /// `idempotency_key` — `String`.
    pub idempotency_key: String,
    /// `request_hash` — `String`.
    pub request_hash: String,
    /// `subject` — `String`.
    pub subject: String,
    /// `actor` — `String`.
    pub actor: String,
    /// `request_id` — `String`.
    pub request_id: String,
    /// `trace_id` — `String`.
    pub trace_id: String,
    /// `causation_id` — `Optional<String>`.
    pub causation_id: Option<String>,
    /// `causation_depth` — `Integer`.
    pub causation_depth: i64,
    /// `occurred_at_unix_nanos` — `String`.
    pub occurred_at_unix_nanos: String,
    /// `occurred_at_offset_seconds` — `Integer`.
    pub occurred_at_offset_seconds: i64,
    /// `claim` — `Optional<ekr.store.NativeClaim>`.
    pub claim: Option<NativeClaim>,
}

/// NativeExpected — `ekr.store.NativeExpected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpected {
    /// `kind` — `ekr.store.NativeExpectedKind`.
    pub kind: NativeExpectedKind,
    /// `version` — `Optional<Integer>`.
    pub version: Option<i64>,
}

/// NativeExpectedKind — `ekr.store.NativeExpectedKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExpectedKind {
    /// `Any`.
    Any,
    /// `NoStream`.
    NoStream,
    /// `Exact`.
    Exact,
}

/// NativeNewEvent — `ekr.store.NativeNewEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNewEvent {
    /// `name` — `String`.
    pub name: String,
    /// `schema_version` — `Integer`.
    pub schema_version: i64,
    /// `data` — `Bytes`.
    pub data: Vec<u8>,
}

/// NativePublicationRequest — `ekr.store.NativePublicationRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePublicationRequest {
    /// `tenant` — `String`.
    pub tenant: String,
    /// `appends` — `List<ekr.store.NativeStreamAppend>`.
    pub appends: Vec<NativeStreamAppend>,
    /// `meta` — `ekr.store.NativeCommandMeta`.
    pub meta: NativeCommandMeta,
    /// `blobs` — `List<ekr.store.NativeBlobWrite>`.
    pub blobs: Vec<NativeBlobWrite>,
}

/// NativeStreamAppend — `ekr.store.NativeStreamAppend`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStreamAppend {
    /// `stream` — `ekr.store.NativeStreamId`.
    pub stream: NativeStreamId,
    /// `expected` — `ekr.store.NativeExpected`.
    pub expected: NativeExpected,
    /// `events` — `List<ekr.store.NativeNewEvent>`.
    pub events: Vec<NativeNewEvent>,
}

/// NativeStreamId — `ekr.store.NativeStreamId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStreamId {
    /// `tenant` — `String`.
    pub tenant: String,
    /// `stream_type` — `String`.
    pub stream_type: String,
    /// `stream_id` — `String`.
    pub stream_id: String,
}

/// Publication — `ekr.store.Publication`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    /// `event` — `ekr.kernel.RevisionEventV3`.
    pub event: crate::kernel::RevisionEventV3,
    /// `objects` — `Map<String, ekr.store.PublicationObject>`.
    pub objects: std::collections::BTreeMap<String, PublicationObject>,
    /// `expected_version` — `Integer`.
    pub expected_version: i64,
}

/// PublicationCommandKey — `ekr.store.PublicationCommandKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationCommandKey {
    /// `kind` — `ekr.store.PublicationCommandKind`.
    pub kind: PublicationCommandKind,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `predecessor_event_id` — `Optional<ekr.kernel.EventId>`.
    pub predecessor_event_id: Option<crate::kernel::EventId>,
    /// `predecessor_record_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub predecessor_record_hash: Option<crate::kernel::ContentHash>,
}

/// PublicationCommandKind — `ekr.store.PublicationCommandKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationCommandKind {
    /// `Bootstrap`.
    Bootstrap,
    /// `Propose`.
    Propose,
    /// `Validate`.
    Validate,
    /// `Commit`.
    Commit,
    /// `UpgradeAuthority`.
    UpgradeAuthority,
}

/// PublicationObject — `ekr.store.PublicationObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationObject {
    /// `storage_class` — `ekr.store.StorageClass`.
    pub storage_class: StorageClass,
    /// `stored_at` — `Timestamp`.
    pub stored_at: crate::primitives::Timestamp,
    /// `bytes` — `Bytes`.
    pub bytes: Vec<u8>,
}

/// PublicationPreparationFormatV1 — `ekr.store.PublicationPreparationFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationPreparationFormatV1 {
    /// `EkrPublicationPreparation1`.
    EkrPublicationPreparation1,
    /// `EkrPublicationPreparation2`.
    EkrPublicationPreparation2,
    /// `EkrPublicationPreparation3`.
    EkrPublicationPreparation3,
    /// `EkrPublicationPreparation4`.
    EkrPublicationPreparation4,
}

/// PublicationPreparationV1 — `ekr.store.PublicationPreparationV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationPreparationV1 {
    /// `format` — `ekr.store.PublicationPreparationFormatV1`.
    pub format: PublicationPreparationFormatV1,
    /// `command_key` — `ekr.store.PublicationCommandKey`.
    pub command_key: PublicationCommandKey,
    /// `input_hash` — `ekr.kernel.ContentHash`.
    pub input_hash: crate::kernel::ContentHash,
    /// `decision` — `ekr.store.Publication`.
    pub decision: Publication,
    /// `attempt_number` — `Integer`.
    pub attempt_number: i64,
    /// `previous_attempt_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub previous_attempt_hash: Option<crate::kernel::ContentHash>,
    /// `native_request` — `ekr.store.NativePublicationRequest`.
    pub native_request: NativePublicationRequest,
    /// `native_fingerprint` — `String`.
    pub native_fingerprint: String,
}

/// PublicationResolution — `ekr.store.PublicationResolution`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationResolution {
    /// `Written`.
    Written,
    /// `AlreadyRecorded`.
    AlreadyRecorded,
    /// `Conflict`.
    Conflict,
    /// `UnknownCommit`.
    UnknownCommit,
}

/// StagedPublicationObject — `ekr.store.StagedPublicationObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedPublicationObject {
    /// `storage_class` — `ekr.store.StorageClass`.
    pub storage_class: StorageClass,
    /// `stored_at` — `Timestamp`.
    pub stored_at: crate::primitives::Timestamp,
    /// `byte_len` — `Integer`.
    pub byte_len: i64,
}

/// StorageClass — `ekr.store.StorageClass`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    /// `Canonical`.
    Canonical,
    /// `Provenance`.
    Provenance,
    /// `Incubating`.
    Incubating,
    /// `Cache`.
    Cache,
    /// `Ephemeral`.
    Ephemeral,
}

/// The states of `ekr.store.StoredObject`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `StoredObject<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredObjectState {
    /// `Stored`.
    Stored,
}

/// What StoredObject — `ekr.store.StoredObject` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`StoredObject<S>`], and at a boundary by [`StoredObjectSnapshot::state`].
///
/// Every value satisfies `byte_len >= 0` — checked by [`StoredObjectData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObjectData {
    /// The identity: `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `storage_class` — `ekr.store.StorageClass`.
    pub storage_class: StorageClass,
    /// `byte_len` — `Integer`.
    pub byte_len: i64,
    /// `stored_at` — `Timestamp`.
    pub stored_at: crate::primitives::Timestamp,
}

impl StoredObjectData {
    /// The first declared invariant of `ekr.store.StoredObject` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.byte_len)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("byte_len >= 0");
        }
        None
    }
}

/// The states of `ekr.store.StoredObject`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](stored_object_state::Marker), so [`StoredObject<S>`](StoredObject) can only ever rest in a real state.
pub mod stored_object_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Stored {}
    }

    /// A declared state of `StoredObject`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::StoredObjectState;
    }

    /// `Stored`. Where a new instance starts.
    pub struct Stored;

    impl Marker for Stored {
        const STATE: super::StoredObjectState = super::StoredObjectState::Stored;
    }
}

/// StoredObject — `ekr.store.StoredObject` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Stored`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`StoredObjectSnapshot`]
/// and [`StoredObjectSnapshot::refine`].
pub struct StoredObject<S: stored_object_state::Marker> {
    data: StoredObjectData,
    state: core::marker::PhantomData<S>,
}

impl<S: stored_object_state::Marker> StoredObject<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> StoredObjectState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &StoredObjectData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> StoredObjectData {
        self.data
    }
}

impl StoredObject<stored_object_state::Stored> {
    /// A new instance, resting in `Stored` — the only state the lifecycle starts one in.
    pub fn new(data: StoredObjectData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.store.StoredObject` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`StoredObjectSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObjectSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: StoredObjectState,
    /// What it holds.
    pub data: StoredObjectData,
}

/// An `StoredObject` in whichever declared state it was found.
pub enum AnyStoredObject {
    /// Resting in `Stored`.
    Stored(StoredObject<stored_object_state::Stored>),
}

impl StoredObjectSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `StoredObjectState` cannot spell one.
    pub fn refine(self) -> AnyStoredObject {
        match self.state {
            StoredObjectState::Stored => AnyStoredObject::Stored(StoredObject {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyStoredObject {
    /// The state, as the runtime value.
    pub fn state(&self) -> StoredObjectState {
        match self {
            Self::Stored(_) => StoredObjectState::Stored,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> StoredObjectSnapshot {
        match self {
            Self::Stored(instance) => StoredObjectSnapshot {
                state: StoredObjectState::Stored,
                data: instance.into_data(),
            },
        }
    }
}

/// CheckpointWritten — the event `ekr.store.CheckpointWritten`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointWritten {
    /// `checkpoint_hash` — `ekr.kernel.ContentHash`.
    pub checkpoint_hash: crate::kernel::ContentHash,
    /// `covered` — `Integer`.
    pub covered: i64,
    /// `binding` — `ekr.kernel.ContentHash`.
    pub binding: crate::kernel::ContentHash,
}

/// ObjectRetentionRaised — the event `ekr.store.ObjectRetentionRaised`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRetentionRaised {
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `from` — `ekr.store.StorageClass`.
    pub from: StorageClass,
    /// `to` — `ekr.store.StorageClass`.
    pub to: StorageClass,
}

/// ObjectStored — the event `ekr.store.ObjectStored`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStored {
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `storage_class` — `ekr.store.StorageClass`.
    pub storage_class: StorageClass,
    /// `byte_len` — `Integer`.
    pub byte_len: i64,
    /// `stored_at` — `Timestamp`.
    pub stored_at: crate::primitives::Timestamp,
}

/// PublicationPrepared — the event `ekr.store.PublicationPrepared`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationPrepared {
    /// `preparation_hash` — `ekr.kernel.ContentHash`.
    pub preparation_hash: crate::kernel::ContentHash,
    /// `attempt_number` — `Integer`.
    pub attempt_number: i64,
    /// `previous_attempt_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub previous_attempt_hash: Option<crate::kernel::ContentHash>,
}

/// The declared error `ekr.store.StoreReplaced`.
///
/// The SQLite database file at the store's path is no longer the one this handle opened; nothing was answered or written through the handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreReplaced {
    /// `path` — `String`.
    pub path: String,
    /// `reason` — `String`.
    pub reason: String,
}
