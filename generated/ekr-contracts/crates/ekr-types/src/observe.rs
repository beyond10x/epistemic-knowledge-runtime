// generated from ekr v1
// model digest 16f0bcba9385e76553b16321dba10f073fcbbf1d53cc29a9a686cf2a907b6146
// contract digest b3aecc34f60f0ce8f8908536c64e38d6497f7a61baffe2b2c753efbc70236b8c
// do not edit: regenerate with `ess synthesize`

//! Observe — `ekr.observe`.
//!
//! The observation layer's own vocabulary: source units, the units of source-side progress this domain names, with their granularity held open; the checkpoint a poll resumes from; the health of one poll; and the idempotency key that makes a second observation of the same source record the same observation. Design § 15, § 54–57; predecessors A8. The observation itself is `ekr.graph.Observation` and is referenced here, not redeclared. Checkpoint state is durable but is not canonical knowledge (§ 55). How units, checkpoints, poll health and observations link to one another remains held open for polling by the remaining decision-blockers, named below where it applies. Redaction (A6), the coverage report and adapter declarations are not modelled yet.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// ObservationIdempotencyKey — `ekr.observe.ObservationIdempotencyKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationIdempotencyKey {
    /// `source` — `String`.
    pub source: String,
    /// `source_native_id` — `Optional<String>`.
    pub source_native_id: Option<String>,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
}

/// ObservationImport — `ekr.observe.ObservationImport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationImport {
    /// `observation` — `ekr.graph.ObservationRecord`.
    pub observation: crate::graph::ObservationRecord,
    /// `key` — `ekr.observe.ObservationIdempotencyKey`.
    pub key: ObservationIdempotencyKey,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// ObservationImportReceipt — `ekr.observe.ObservationImportReceipt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationImportReceipt {
    /// `outcome` — `ekr.observe.ObservationRetentionOutcome`.
    pub outcome: ObservationRetentionOutcome,
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `already_retained` — `Boolean`.
    pub already_retained: bool,
}

/// ObservationRetentionOutcome — `ekr.observe.ObservationRetentionOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationRetentionOutcome {
    /// `Retained`.
    Retained,
    /// `AlreadyRetained`.
    AlreadyRetained,
}

/// PollHealth — `ekr.observe.PollHealth`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollHealth {
    /// `status` — `ekr.observe.PollStatus`.
    pub status: PollStatus,
    /// `checked_through` — `Optional<Timestamp>`.
    pub checked_through: Option<crate::primitives::Timestamp>,
}

/// PollStatus — `ekr.observe.PollStatus`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollStatus {
    /// `Attempt`.
    Attempt,
    /// `Complete`.
    Complete,
    /// `Partial`.
    Partial,
    /// `Failed`.
    Failed,
}

/// The states of `ekr.observe.RetainedObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `RetainedObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedObservationState {
    /// `Recorded`.
    Recorded,
}

/// RetainedObservationRead — `ekr.observe.RetainedObservationRead`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedObservationRead {
    /// `observation` — `ekr.graph.ObservationRecord`.
    pub observation: crate::graph::ObservationRecord,
    /// `key` — `ekr.observe.ObservationIdempotencyKey`.
    pub key: ObservationIdempotencyKey,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// The states of `ekr.observe.SourceCheckpoint`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SourceCheckpoint<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCheckpointState {
    /// `Recorded`.
    Recorded,
}

/// SourceCheckpointId — `ekr.observe.SourceCheckpointId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckpointId(pub crate::primitives::Uuid);

/// SourceRecordObservation — `ekr.observe.SourceRecordObservation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecordObservation {
    /// `key` — `ekr.observe.ObservationIdempotencyKey`.
    pub key: ObservationIdempotencyKey,
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
}

/// The states of `ekr.observe.SourceUnit`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SourceUnit<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceUnitState {
    /// `Present`.
    Present,
}

/// SourceUnitId — `ekr.observe.SourceUnitId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnitId(pub crate::primitives::Uuid);

/// What RetainedObservation — `ekr.observe.RetainedObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`RetainedObservation<S>`], and at a boundary by [`RetainedObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedObservationData {
    /// The identity: `observation_id` — `ekr.graph.ObservationId`.
    ///
    /// Carries `observation`: `ekr.observe.RetainedObservation` references one `ekr.graph.Observation`.
    pub observation_id: crate::graph::ObservationId,
    /// `key` — `ekr.observe.ObservationIdempotencyKey`.
    pub key: ObservationIdempotencyKey,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `bytes`: `ekr.observe.RetainedObservation` references one `ekr.store.StoredObject`.
    pub content_hash: crate::kernel::ContentHash,
    /// `retained_at` — `Timestamp`.
    pub retained_at: crate::primitives::Timestamp,
}

/// The states of `ekr.observe.RetainedObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](retained_observation_state::Marker), so [`RetainedObservation<S>`](RetainedObservation) can only ever rest in a real state.
pub mod retained_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `RetainedObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RetainedObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::RetainedObservationState = super::RetainedObservationState::Recorded;
    }
}

/// RetainedObservation — `ekr.observe.RetainedObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RetainedObservationSnapshot`]
/// and [`RetainedObservationSnapshot::refine`].
pub struct RetainedObservation<S: retained_observation_state::Marker> {
    data: RetainedObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: retained_observation_state::Marker> RetainedObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RetainedObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RetainedObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RetainedObservationData {
        self.data
    }
}

impl RetainedObservation<retained_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: RetainedObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.observe.RetainedObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RetainedObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RetainedObservationState,
    /// What it holds.
    pub data: RetainedObservationData,
}

/// An `RetainedObservation` in whichever declared state it was found.
pub enum AnyRetainedObservation {
    /// Resting in `Recorded`.
    Recorded(RetainedObservation<retained_observation_state::Recorded>),
}

impl RetainedObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RetainedObservationState` cannot spell one.
    pub fn refine(self) -> AnyRetainedObservation {
        match self.state {
            RetainedObservationState::Recorded => AnyRetainedObservation::Recorded(RetainedObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRetainedObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> RetainedObservationState {
        match self {
            Self::Recorded(_) => RetainedObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RetainedObservationSnapshot {
        match self {
            Self::Recorded(instance) => RetainedObservationSnapshot {
                state: RetainedObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What SourceCheckpoint — `ekr.observe.SourceCheckpoint` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SourceCheckpoint<S>`], and at a boundary by [`SourceCheckpointSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckpointData {
    /// The identity: `source_checkpoint_id` — `ekr.observe.SourceCheckpointId`.
    pub source_checkpoint_id: SourceCheckpointId,
    /// `cursor` — `String`.
    pub cursor: String,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// The states of `ekr.observe.SourceCheckpoint`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](source_checkpoint_state::Marker), so [`SourceCheckpoint<S>`](SourceCheckpoint) can only ever rest in a real state.
pub mod source_checkpoint_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SourceCheckpoint`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SourceCheckpointState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SourceCheckpointState = super::SourceCheckpointState::Recorded;
    }
}

/// SourceCheckpoint — `ekr.observe.SourceCheckpoint` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SourceCheckpointSnapshot`]
/// and [`SourceCheckpointSnapshot::refine`].
pub struct SourceCheckpoint<S: source_checkpoint_state::Marker> {
    data: SourceCheckpointData,
    state: core::marker::PhantomData<S>,
}

impl<S: source_checkpoint_state::Marker> SourceCheckpoint<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SourceCheckpointState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SourceCheckpointData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SourceCheckpointData {
        self.data
    }
}

impl SourceCheckpoint<source_checkpoint_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SourceCheckpointData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.observe.SourceCheckpoint` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SourceCheckpointSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckpointSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SourceCheckpointState,
    /// What it holds.
    pub data: SourceCheckpointData,
}

/// An `SourceCheckpoint` in whichever declared state it was found.
pub enum AnySourceCheckpoint {
    /// Resting in `Recorded`.
    Recorded(SourceCheckpoint<source_checkpoint_state::Recorded>),
}

impl SourceCheckpointSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SourceCheckpointState` cannot spell one.
    pub fn refine(self) -> AnySourceCheckpoint {
        match self.state {
            SourceCheckpointState::Recorded => AnySourceCheckpoint::Recorded(SourceCheckpoint {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySourceCheckpoint {
    /// The state, as the runtime value.
    pub fn state(&self) -> SourceCheckpointState {
        match self {
            Self::Recorded(_) => SourceCheckpointState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SourceCheckpointSnapshot {
        match self {
            Self::Recorded(instance) => SourceCheckpointSnapshot {
                state: SourceCheckpointState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What SourceUnit — `ekr.observe.SourceUnit` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SourceUnit<S>`], and at a boundary by [`SourceUnitSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnitData {
    /// The identity: `source_unit_id` — `ekr.observe.SourceUnitId`.
    pub source_unit_id: SourceUnitId,
    /// `source` — `String`.
    pub source: String,
    /// `source_native_id` — `Optional<String>`.
    pub source_native_id: Option<String>,
}

/// The states of `ekr.observe.SourceUnit`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](source_unit_state::Marker), so [`SourceUnit<S>`](SourceUnit) can only ever rest in a real state.
pub mod source_unit_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Present {}
    }

    /// A declared state of `SourceUnit`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SourceUnitState;
    }

    /// `Present`. Where a new instance starts.
    pub struct Present;

    impl Marker for Present {
        const STATE: super::SourceUnitState = super::SourceUnitState::Present;
    }
}

/// SourceUnit — `ekr.observe.SourceUnit` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Present`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SourceUnitSnapshot`]
/// and [`SourceUnitSnapshot::refine`].
pub struct SourceUnit<S: source_unit_state::Marker> {
    data: SourceUnitData,
    state: core::marker::PhantomData<S>,
}

impl<S: source_unit_state::Marker> SourceUnit<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SourceUnitState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SourceUnitData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SourceUnitData {
        self.data
    }
}

impl SourceUnit<source_unit_state::Present> {
    /// A new instance, resting in `Present` — the only state the lifecycle starts one in.
    pub fn new(data: SourceUnitData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.observe.SourceUnit` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SourceUnitSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnitSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SourceUnitState,
    /// What it holds.
    pub data: SourceUnitData,
}

/// An `SourceUnit` in whichever declared state it was found.
pub enum AnySourceUnit {
    /// Resting in `Present`.
    Present(SourceUnit<source_unit_state::Present>),
}

impl SourceUnitSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SourceUnitState` cannot spell one.
    pub fn refine(self) -> AnySourceUnit {
        match self.state {
            SourceUnitState::Present => AnySourceUnit::Present(SourceUnit {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySourceUnit {
    /// The state, as the runtime value.
    pub fn state(&self) -> SourceUnitState {
        match self {
            Self::Present(_) => SourceUnitState::Present,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SourceUnitSnapshot {
        match self {
            Self::Present(instance) => SourceUnitSnapshot {
                state: SourceUnitState::Present,
                data: instance.into_data(),
            },
        }
    }
}

/// ImportObservation — the input of `ekr.observe.ImportObservation`.
///
/// Everything it can result in is [`ImportObservationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportObservation {
    /// `document` — `ekr.observe.ObservationImport`.
    pub document: ObservationImport,
}

/// Actual typed response of `ekr.observe.ImportObservation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportObservationResponse {
    /// `receipt` — `ekr.observe.ObservationImportReceipt`.
    pub receipt: ObservationImportReceipt,
}

/// Everything `ekr.observe.ImportObservation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportObservationOutcome {
    /// `answered` — otherwise.
    ///
    /// Retain the observation and exact bytes atomically through the store abstraction, before interpretation. A matching key returns its original identity without another record. It does not commit a canonical revision.
    Answered {
        /// The `ekr.observe.ImportObservationResult` this outcome publishes.
        import_observation_result: ImportObservationResult,
    },
    /// `refused` — externally decided (Payload digest, source key coherence, immutable identity and idempotency are checked against retained records; a reused identity with other bytes is refused.).
    Refused {
        /// Why it was refused: `ekr.observe.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ListObservations — the input of `ekr.observe.ListObservations`.
///
/// Everything it can result in is [`ListObservationsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListObservations {
}

/// Actual typed response of `ekr.observe.ListObservations`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListObservationsResponse {
    /// `observations` — `List<ekr.graph.ObservationRecord>`.
    pub observations: Vec<crate::graph::ObservationRecord>,
}

/// Everything `ekr.observe.ListObservations` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListObservationsOutcome {
    /// `listed` — otherwise.
    ///
    /// Read retained observations independently of canonical revisions and interpretations, in identity order.
    Listed {
        /// The `ekr.observe.ObservationsListed` this outcome publishes.
        observations_listed: ObservationsListed,
    },
}

/// ShowObservation — the input of `ekr.observe.ShowObservation`.
///
/// Everything it can result in is [`ShowObservationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowObservation {
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
}

/// Actual typed response of `ekr.observe.ShowObservation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowObservationResponse {
    /// `retained` — `ekr.observe.RetainedObservationRead`.
    pub retained: RetainedObservationRead,
}

/// Everything `ekr.observe.ShowObservation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowObservationOutcome {
    /// `shown` — otherwise.
    ///
    /// Return exact retained bytes and their source metadata after reopen, including observations with no accepted interpretation.
    Shown {
        /// The `ekr.observe.ObservationShown` this outcome publishes.
        observation_shown: ObservationShown,
    },
    /// `refused` — externally decided (An unknown observation or missing/mismatched retained bytes is refused before answering.).
    Refused {
        /// Why it was refused: `ekr.observe.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ImportObservationResult — the event `ekr.observe.ImportObservationResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportObservationResult {
    /// `receipt` — `ekr.observe.ObservationImportReceipt`.
    pub receipt: ObservationImportReceipt,
}

/// ObservationShown — the event `ekr.observe.ObservationShown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationShown {
    /// `retained` — `ekr.observe.RetainedObservationRead`.
    pub retained: RetainedObservationRead,
}

/// ObservationsListed — the event `ekr.observe.ObservationsListed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationsListed {
    /// `observations` — `List<ekr.graph.ObservationRecord>`.
    pub observations: Vec<crate::graph::ObservationRecord>,
}

/// The declared error `ekr.observe.KnowledgeRefused`.
///
/// A named deterministic refusal; no unreported write occurred. Partial application is a typed report, never this refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRefused {
    /// `code` — `String`.
    pub code: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// RetainedObservationRecords — one row of the view `ekr.observe.RetainedObservationRecords`.
///
/// Projects `ekr.observe.RetainedObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedObservationRecords {
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
    /// `state` — `ekr.observe.RetainedObservation.State`.
    pub state: RetainedObservationState,
    /// `key` — `ekr.observe.ObservationIdempotencyKey`.
    pub key: ObservationIdempotencyKey,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `retained_at` — `Timestamp`.
    pub retained_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `ekr.observe.ImportObservation` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.observe.ImportObservation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.observe.ImportObservationResult`; `refused` externally decided (Payload digest, source key coherence, immutable identity and idempotency are checked against retained records; a reused identity with other bytes is refused.), error `ekr.observe.KnowledgeRefused`.
    pub trait ImportObservationBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.observe.ImportObservation`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn import_observation(&mut self, input: super::ImportObservation) -> Result<super::ImportObservationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.observe.ListObservations` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.observe.ListObservations` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `listed` otherwise, emits `ekr.observe.ObservationsListed`.
    pub trait ListObservationsBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.observe.ListObservations`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn list_observations(&mut self, input: super::ListObservations) -> Result<super::ListObservationsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.observe.ShowObservation` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.observe.ShowObservation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `shown` otherwise, emits `ekr.observe.ObservationShown`; `refused` externally decided (An unknown observation or missing/mismatched retained bytes is refused before answering.), error `ekr.observe.KnowledgeRefused`.
    pub trait ShowObservationBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.observe.ShowObservation`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn show_observation(&mut self, input: super::ShowObservation) -> Result<super::ShowObservationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.observe.RetainedObservationRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RetainedObservationRecordsQuery {
        /// Serves `ekr.observe.RetainedObservationRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn retained_observation_records(&self) -> Result<Vec<super::RetainedObservationRecords>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl ImportObservationBehavior for Unimplemented {
        fn import_observation(&mut self, _input: super::ImportObservation) -> Result<super::ImportObservationOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.observe.ImportObservation" })
        }
    }

    impl ListObservationsBehavior for Unimplemented {
        fn list_observations(&mut self, _input: super::ListObservations) -> Result<super::ListObservationsOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.observe.ListObservations" })
        }
    }

    impl ShowObservationBehavior for Unimplemented {
        fn show_observation(&mut self, _input: super::ShowObservation) -> Result<super::ShowObservationOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.observe.ShowObservation" })
        }
    }
}
