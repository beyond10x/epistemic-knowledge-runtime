//! A stage of a store (design § 107, `systems/ekr/domains/store.yaml`): its tenant, its record
//! and the carriers of its declarations.
//!
//! A stage's tenant is the Eventlog tenant a stage of a store is written in, derived from the
//! store's own tenant and the stage id, and carrying a marker no store's tenant may carry. The
//! derivation is deterministic, gives different names for different stage ids and for different
//! store tenants, and is a valid Eventlog tenant for every store tenant: 111 ASCII graphic bytes.
//! Because every derived name carries [`STAGE_TENANT_MARKER`] and a store is never opened under a
//! tenant that carries it ([`admit_store_tenant`], which the kernel's openers apply to every host
//! configuration's tenant), the derivation can produce neither the store's own tenant nor another
//! store's.
//!
//! The stage's record is a private stream per stage in the store's own tenant ([`StageLog`]),
//! outside the revision stream: it holds `ekr.store.StageBegun`, `StageSealed`, `StagePublished`
//! and `StageAbandoned` and no blob, and stays after the stage's tenant is forgotten.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{ContentHash, RevisionId, RevisionNumber, StageId};
use eventlog_core::TenantId;
use serde::{Deserialize, Serialize};

use crate::{Appended, PublicationPreparationV4, StagePublication, StorageClass, StoreError};

/// The marker every stage tenant carries and no store's tenant may carry anywhere.
pub const STAGE_TENANT_MARKER: &str = "ekr.stage:";

/// The tenant of stage `stage` of the store whose tenant is `store`:
/// `ekr.stage:<store>:<stage>`, where `<store>` is the 64 lowercase hex characters of the store
/// tenant's payload address ([`ContentHash::of_bytes`]) and `<stage>` the stage id in its
/// lowercase hyphenated form.
///
/// `stage` is the id begin minted; the caller passes it on and never one of its own choosing.
///
/// # Errors
/// [`StoreError::StageTenantReserved`] for a `store` tenant that carries the marker, which is no
/// store's tenant, and the provider's refusal of a `store` tenant Eventlog does not admit.
pub fn stage_tenant(store: &str, stage: StageId) -> Result<String, StoreError> {
    admit_store_tenant(store)?;
    TenantId::new(store)?;
    let name = format!(
        "{STAGE_TENANT_MARKER}{}:{stage}",
        ContentHash::of_bytes(store.as_bytes()).to_hex(),
    );
    TenantId::new(name.as_str())?;
    Ok(name)
}

/// Admits `tenant` as a store's own tenant: refuses one that carries [`STAGE_TENANT_MARKER`]
/// anywhere, before anything is read, created or written.
///
/// # Errors
/// [`StoreError::StageTenantReserved`], `stage-tenant-reserved` (`ekr.store.StageTenantReserved`).
pub fn admit_store_tenant(tenant: &str) -> Result<(), StoreError> {
    if tenant.contains(STAGE_TENANT_MARKER) {
        Err(StoreError::StageTenantReserved(tenant.to_owned()))
    } else {
        Ok(())
    }
}

/// A store's identity: the Eventlog tenant it is opened with (`ekr.store.StoreTenant`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StoreTenant(pub String);

/// The provider a store is opened on (`ekr.store.ProviderKind`), as the CLI's `--backend` names
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    /// The File provider; it admits no stage.
    File,
    /// The SQLite provider.
    Sqlite,
    /// The hosted PostgreSQL provider.
    Postgres,
}

impl ProviderKind {
    /// The name the CLI's `--backend` gives it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
        }
    }
}

/// A store (`ekr.store.Store`): one Eventlog tenant on one provider. Its head is read from its
/// revision stream, never carried here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    /// The tenant the store is opened with.
    pub tenant: StoreTenant,
    /// The provider it is opened on.
    pub provider: ProviderKind,
}

/// A stage's lifecycle state (`ekr.store.Stage.State`): Begun, then Sealing, then Published;
/// Begun or Sealing may become Abandoned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageState {
    /// Begun and accepting joined writes.
    Begun,
    /// Sealed for publication; no joined write lands.
    Sealing,
    /// Its suffix is in the store.
    Published,
    /// Dropped whole.
    Abandoned,
}

impl StageState {
    /// The name `ekr.store.StageStateConflict` is refused under for a stage in this state.
    #[must_use]
    pub const fn refusal(self) -> &'static str {
        match self {
            Self::Begun => "stage-not-sealed",
            Self::Sealing => "stage-sealed",
            Self::Published => "stage-already-published",
            Self::Abandoned => "stage-already-abandoned",
        }
    }
}

/// A stage (`ekr.store.Stage`): begun from a store at its head, written by a run, then published
/// whole or abandoned whole. `published_revisions` are the store's revisions its publication
/// added, read from the store from the first to the last its record names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    /// Minted by begin.
    pub stage_id: StageId,
    /// The store it was begun from, whose tenant holds its record.
    pub store: StoreTenant,
    /// The store's head when it was begun.
    pub base: RevisionNumber,
    /// The identity of that revision.
    pub base_revision: RevisionId,
    /// The store's revisions its publication added, in order; empty until it is published.
    pub published_revisions: Vec<RevisionId>,
}

/// What begin, seal, publish and abandon answer (`ekr.store.StageResult`): the stage record as the
/// command left it. A retry that replays returns the original value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageResult {
    /// The stage.
    pub stage_id: StageId,
    /// Its state after the command.
    pub state: StageState,
    /// The store's head it was begun at.
    pub base: RevisionNumber,
    /// That revision's identity.
    pub base_revision: RevisionId,
    /// The first revision its publication added, absent before publication or where the run
    /// committed nothing.
    pub published_first: Option<RevisionId>,
    /// The last revision its publication added.
    pub published_last: Option<RevisionId>,
    /// How many revision-stream occurrences its publication appended; absent before publication.
    pub occurrences: Option<u64>,
}

/// What a stage's `ekr.store.StagePublished` records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagePublishedRecord {
    /// The store's head after the publication.
    pub head: RevisionNumber,
    /// The first revision it added.
    pub published_first: Option<RevisionId>,
    /// The last revision it added.
    pub published_last: Option<RevisionId>,
    /// How many revision-stream occurrences it appended.
    pub occurrences: u64,
}

/// A stage's record as its stream in the store's tenant holds it, folded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageRecord {
    /// The stage.
    pub stage_id: StageId,
    /// The stage's tenant as begin derived and wrote it; every forgetting takes it from here.
    pub tenant: String,
    /// The store's head it was begun at.
    pub base: RevisionNumber,
    /// That revision's identity.
    pub base_revision: RevisionId,
    /// Its state.
    pub state: StageState,
    /// The record stream's version: how many events it holds.
    pub version: u64,
    /// The record stream's version at which the stage became Sealing, if it did.
    pub sealed_at: Option<u64>,
    /// What its publication recorded, once Published.
    pub published: Option<StagePublishedRecord>,
}

impl StageRecord {
    /// The command result this record answers.
    #[must_use]
    pub fn result(&self) -> StageResult {
        StageResult {
            stage_id: self.stage_id,
            state: self.state,
            base: self.base,
            base_revision: self.base_revision,
            published_first: self
                .published
                .as_ref()
                .and_then(|published| published.published_first),
            published_last: self
                .published
                .as_ref()
                .and_then(|published| published.published_last),
            occurrences: self
                .published
                .as_ref()
                .map(|published| published.occurrences),
        }
    }
}

/// The stage record, stage publication and stage tenant operations of a store (design § 107).
///
/// Each runs against the store's own tenant; a handle joined to a stage refuses them. They decide
/// nothing about canonical state: a publication's group is admitted only after the store's
/// injected [`crate::CommitAuthority`] verifies the store's history with the suffix after it.
pub trait StageLog {
    /// The provider this store is on.
    fn provider(&self) -> ProviderKind;
    /// This store's own tenant.
    fn store_tenant(&self) -> String;
    /// The record of `stage` in this store, folded, or `None` when no stage carries the id.
    /// # Errors
    /// Provider failure or a record stream that does not read as a stage's lifecycle.
    fn stage_record(&self, stage: StageId) -> Result<Option<StageRecord>, StoreError>;
    /// Every stage this store has recorded, in every state (`ekr.cli.Stages`).
    /// # Errors
    /// Provider failure or a record that does not read.
    fn stages(&self) -> Result<Vec<StageRecord>, StoreError>;
    /// Writes `ekr.store.StageBegun`: the stream must not exist yet.
    /// # Errors
    /// A stage already carrying the id, a tenant that is not `stage`'s derived tenant, or a
    /// provider failure.
    fn record_stage_begun(
        &self,
        stage: StageId,
        tenant: &str,
        base: RevisionNumber,
        base_revision: RevisionId,
    ) -> Result<StageRecord, StoreError>;
    /// Writes `ekr.store.StageSealed` after `record`, conditionally on the stream at its version.
    /// # Errors
    /// [`StoreError::Conflict`] when the record moved; a stage not Begun; provider failure.
    fn record_stage_sealed(&self, record: &StageRecord) -> Result<StageRecord, StoreError>;
    /// Writes `ekr.store.StageAbandoned` after `record`, conditionally on the stream at its
    /// version: an elected publication group expects the same version, so once this is appended
    /// that group can never land.
    /// # Errors
    /// [`StoreError::Conflict`] when the record moved; a stage neither Begun nor Sealing;
    /// provider failure.
    fn record_stage_abandoned(&self, record: &StageRecord) -> Result<StageRecord, StoreError>;
    /// The stage's elected publication attempt (`ekr.publication-preparation/4`), if any.
    /// # Errors
    /// A preparation that does not read or does not authorize.
    fn stage_preparation(
        &self,
        stage: StageId,
    ) -> Result<Option<PublicationPreparationV4>, StoreError>;
    /// Elects `decision` in the stage's publication slot, or a successor of `previous` after a
    /// definitive conflict of it; an attempt already elected for the same input is returned.
    /// # Errors
    /// Input conflict, a decision the store's authority refuses, or provider failure.
    fn prepare_stage_publication(
        &self,
        input_hash: ContentHash,
        decision: &StagePublication,
        previous: Option<&PublicationPreparationV4>,
    ) -> Result<PublicationPreparationV4, StoreError>;
    /// Appends the exact elected group. A definitive conflict is answered by what moved, read
    /// again in design § 107.4's order.
    /// # Errors
    /// `StageStateConflict` (Abandoned), `StageHeadMoved`, `StageStreamMoved`,
    /// `StageObjectMoved`, an unknown outcome, or provider failure.
    fn resume_stage_publication(
        &self,
        prepared: &PublicationPreparationV4,
    ) -> Result<Appended, StoreError>;
    /// The class this store holds each of `hashes` at, for those it holds.
    /// # Errors
    /// Provider failure or an object that does not verify.
    fn held_classes(
        &self,
        hashes: &BTreeSet<ContentHash>,
    ) -> Result<BTreeMap<ContentHash, StorageClass>, StoreError>;
    /// How many occurrences this store's revision stream holds.
    /// # Errors
    /// Provider failure.
    fn revision_stream_version(&self) -> Result<u64, StoreError>;
    /// Forgets the tenant of a Published or Abandoned stage with the provider's `forget_tenant`
    /// (held bytes, rule 2): the tenant its `StageBegun` names, only where that is the tenant
    /// derived from this store's tenant and the stage id, and never this store's own.
    /// # Errors
    /// `StageNotFound`, `StageStateConflict` for a stage neither Published nor Abandoned,
    /// `stage-tenant-not-derived`, `StageUnsupportedProvider` on File, or provider failure.
    fn forget_stage_tenant(&self, stage: StageId) -> Result<(), StoreError>;
}

/// A point in a stage command where a test interleaves another writer, or an interruption.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagePoint {
    /// A write through a handle joined to a stage has passed its check and not yet appended.
    JoinedWrite,
    /// Seal has checked the stage and the store and not yet written `StageSealed`.
    SealChecked,
    /// Seal has written `StageSealed`; publish has not yet read the stage.
    Sealed,
    /// Publish has captured the stage and derived its suffix, and not yet elected an attempt.
    PublishCaptured,
    /// Publish has captured the stage and elected its attempt, and not yet appended the group.
    PublishElected,
    /// The group is appended; the stage's tenant is not yet forgotten.
    PublishAppended,
    /// Abandon has read the record and not yet written `StageAbandoned`.
    AbandonRead,
    /// Abandon has written `StageAbandoned` and not yet forgotten the tenant.
    AbandonRecorded,
}

type Hook = Box<dyn FnMut(StagePoint) -> Result<(), StoreError>>;

thread_local! {
    static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
}

/// Restores the calling thread's previous hook when dropped.
#[doc(hidden)]
pub struct StageHookGuard(Option<Hook>);

impl Drop for StageHookGuard {
    fn drop(&mut self) {
        let previous = self.0.take();
        HOOK.with(|hook| *hook.borrow_mut() = previous);
    }
}

/// Test instrumentation: runs `hook` at each [`StagePoint`] the calling thread reaches until the
/// guard drops. An `Err` it returns is what the command at that point returns, as an interruption
/// there would leave it. The hook is not run again while it runs, so another writer it drives
/// reaches no point.
#[doc(hidden)]
pub fn on_stage_point(
    hook: impl FnMut(StagePoint) -> Result<(), StoreError> + 'static,
) -> StageHookGuard {
    let previous = HOOK.with(|held| held.borrow_mut().replace(Box::new(hook)));
    StageHookGuard(previous)
}

/// Runs the calling thread's hook at `point`, if one is set.
///
/// # Errors
/// Whatever the hook returns.
#[doc(hidden)]
pub fn reached(point: StagePoint) -> Result<(), StoreError> {
    let Some(mut hook) = HOOK.with(|held| held.borrow_mut().take()) else {
        return Ok(());
    };
    let result = hook(point);
    HOOK.with(|held| {
        let mut held = held.borrow_mut();
        if held.is_none() {
            *held = Some(hook);
        }
    });
    result
}
