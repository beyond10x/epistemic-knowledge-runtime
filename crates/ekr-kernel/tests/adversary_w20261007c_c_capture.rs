//! Adversary, wave 2026-10-07c, unit C (`task:postgres-source-copy`, commit 3c96c31e52).
//!
//! Attacks the claim that a store is read once for a preserving copy — one PostgreSQL capture, one
//! SQLite image — and that `CapturedStore::copy_into` reads nothing more of the source, and the
//! claim that an interrupted stage copy never reads as complete.
//!
//! The PostgreSQL halves run where the PostgreSQL tests run: set EKR_TEST_POSTGRES_CONFIG and
//! EKR_TEST_POSTGRES_OWNER to file references; EKR_REQUIRE_POSTGRES=1 makes their absence a
//! failure.
#[path = "support/hosted_store.rs"]
mod controlled;
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{context, seed, SEEDED_AT};
use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, Commit, CommitCommandResult, GraphOperation,
    GraphTransaction, Runtime, StreamReads, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::NodeType;
use ekr_store::{
    Appended, Inventory, ObjectStore, PostgresStore, Publication, PublicationCommandKey,
    PublicationPreparationV1, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
    StoreInventory, StoredObject,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

fn config() -> Option<PostgresConfiguration> {
    let path = match std::env::var_os("EKR_TEST_POSTGRES_CONFIG") {
        Some(path) => path,
        None => {
            assert_ne!(
                std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
                Ok("1"),
                "required real PostgreSQL fixture is missing"
            );
            eprintln!("SKIP real PostgreSQL: EKR_TEST_POSTGRES_CONFIG is unset");
            return None;
        }
    };
    static SCHEMA: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        let owner =
            std::env::var_os("EKR_TEST_POSTGRES_OWNER").expect("owner configuration reference");
        Runtime::postgres_schema(&PostgresConfiguration::read(Path::new(&owner)).unwrap()).unwrap();
    });
    Some(PostgresConfiguration::read(Path::new(&path)).unwrap())
}

fn anchor() -> AuthorityStateV1 {
    let mut anchor = current_fixture::anchor();
    anchor.validation_profile = ValidationProfileV1::schema_evolving(context().validator);
    anchor
}

fn tenant() -> String {
    format!("adversary-c-{}", TypeId::mint())
}

fn stage_id() -> u128 {
    TypeId::mint().as_u128()
}

fn document(at: i64) -> (TransactionId, Vec<u8>) {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Adversary{at}"),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .unwrap();
    (transaction.id, bytes.into_bytes())
}

/// Proposes one transaction and leaves it Proposed: an occurrence that moves no head.
fn propose_only(runtime: &Runtime, at: i64) -> TransactionId {
    let (id, bytes) = document(at);
    runtime
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    id
}

/// Proposes, validates and commits one transaction: the head moves.
fn extend(runtime: &Runtime, at: i64) -> TransactionId {
    let (id, bytes) = document(at);
    runtime
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = runtime
        .validate(id, runtime.head().unwrap().unwrap().revision, || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let committed = runtime
        .commit(id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{committed:?}"
    );
    id
}

fn populate(runtime: &Runtime) {
    runtime.seed(seed(), || SEEDED_AT).unwrap();
    extend(runtime, 20);
}

/// Every call the kernel makes into a store, by method name.
#[derive(Clone, Default)]
struct Calls(Arc<Mutex<BTreeMap<&'static str, u32>>>);

impl Calls {
    fn hit(&self, name: &'static str) {
        *self.0.lock().unwrap().entry(name).or_default() += 1;
    }
    fn take(&self) -> BTreeMap<&'static str, u32> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

/// A store that records every call made into it and otherwise is the store it wraps.
struct Watched<S> {
    inner: S,
    calls: Calls,
}

impl<S: RevisionLog> RevisionLog for Watched<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.calls.hit("preparation");
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.calls.hit("prepare");
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(&self, prepared: &PublicationPreparationV1) -> Result<Appended, StoreError> {
        self.calls.hit("resume");
        self.inner.resume(prepared)
    }
    fn history(&self) -> Result<RetainedHistory, StoreError> {
        self.calls.hit("history");
        self.inner.history()
    }
    fn replay_history(&self) -> Result<RetainedHistory, StoreError> {
        self.calls.hit("replay_history");
        self.inner.replay_history()
    }
    fn history_at(&self, revision: RevisionNumber) -> Result<RetainedHistory, StoreError> {
        self.calls.hit("history_at");
        self.inner.history_at(revision)
    }
    fn publish(&self, publication: &Publication) -> Result<Appended, StoreError> {
        self.calls.hit("publish");
        self.inner.publish(publication)
    }
    fn checkpoint_covered(&self) -> Result<Option<u64>, StoreError> {
        self.calls.hit("checkpoint_covered");
        self.inner.checkpoint_covered()
    }
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        bytes: Option<&[u8]>,
    ) -> Result<bool, StoreError> {
        self.calls.hit("write_checkpoint");
        self.inner.write_checkpoint(covered, binding, bytes)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        self.calls.hit("seed_bytes");
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.calls.hit("fold");
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<ekr_graph::Root>, StoreError> {
        self.calls.hit("head");
        self.inner.head()
    }
    fn replay(&self, revision: RevisionNumber) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.calls.hit("replay");
        self.inner.replay(revision)
    }
}
impl<S: ObjectStore> ObjectStore for Watched<S> {
    fn put(
        &self,
        class: StorageClass,
        bytes: &[u8],
        at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        self.calls.hit("put");
        self.inner.put(class, bytes, at)
    }
    fn get(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.calls.hit("get");
        self.inner.get(hash)
    }
}
impl<S: Inventory> Inventory for Watched<S> {
    fn inventory(&self) -> Result<StoreInventory, StoreError> {
        self.calls.hit("inventory");
        self.inner.inventory()
    }
    fn is_empty(&self) -> Result<bool, StoreError> {
        self.calls.hit("is_empty");
        self.inner.is_empty()
    }
}

fn postgres_kernel(
    config: &PostgresConfiguration,
    tenant: &str,
    reading: bool,
    full: bool,
) -> Commit<PostgresStore> {
    Commit::over_with_authority(context(), anchor(), |authority| {
        let mut store = PostgresStore::postgres(config, tenant, reading)?;
        store.set_full_replay(full);
        Ok(store.under(authority))
    })
    .unwrap()
}

fn sqlite_kernel(path: &Path, tenant: &str, full: bool) -> Commit<SqliteStore> {
    Commit::over_with_authority(context(), anchor(), |authority| {
        let mut store = SqliteStore::sqlite(path, tenant, None)?;
        store.set_full_replay(full);
        Ok(store.under(authority))
    })
    .unwrap()
}

/// The ids of every transaction a store's verified state holds, of every lifecycle state.
fn transactions<S: RevisionLog + ObjectStore>(kernel: &Commit<S>) -> BTreeSet<TransactionId> {
    kernel.transactions().unwrap().keys().copied().collect()
}

/// What design § 107.1 says a stage keeps of a root: the revision and the ontology, knowledge,
/// evidence and authority roots. Addresses naming the seed envelope or a record are the stage's.
fn kept(root: ekr_graph::Root) -> (RevisionNumber, [ContentHash; 4]) {
    (
        root.revision,
        [
            root.ontology_root,
            root.knowledge_root,
            root.evidence_root,
            root.agent_root,
        ],
    )
}

/// The copy holds the capture and nothing written to the source after it: the captured head,
/// exactly the transactions the source held at the capture — a Proposed one made after it
/// included — and no read of the source between the capture and the end of the copy.
fn copy_after_writes<S, D>(
    source: &Commit<S>,
    calls: &Calls,
    write_after_capture: impl FnOnce() -> (TransactionId, TransactionId),
    stage: &Commit<D>,
) -> (ekr_graph::Root, BTreeSet<TransactionId>)
where
    S: RevisionLog + ObjectStore + Inventory,
    D: RevisionLog + ObjectStore + ekr_store::Initialize + Inventory,
{
    let held = transactions(source);
    let _ = calls.take();
    let captured = source.capture().unwrap();
    assert_eq!(
        calls.take(),
        BTreeMap::from([("inventory", 1)]),
        "a capture is one inventory of the source and nothing else"
    );
    let at = captured.head();
    let (committed, proposed) = write_after_capture();
    captured.copy_into(stage).unwrap();
    assert_eq!(
        calls.take(),
        BTreeMap::new(),
        "copy_into read the source again after its capture"
    );
    assert!(!held.contains(&committed) && !held.contains(&proposed));
    (at, held)
}

/// Design § 107.2 and `CapturedStore::copy_into`'s contract on PostgreSQL: a commit and a proposal
/// written to the source after the capture and before the copy are in no copy of it, and the copy
/// makes no call into the source. `a_commit_racing_the_capture_is_not_in_the_stage` writes its
/// racing commit inside the destination's `initialize`, after any read a copy could make at its
/// start, and compares revisions only, so a copy that captured the source again first, or carried
/// a later Proposed transaction, passes it.
#[test]
fn a_postgres_copy_reads_nothing_of_the_source_after_its_capture() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    populate(&store);
    let calls = Calls::default();
    let source = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(Watched {
            inner: PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            calls: calls.clone(),
        })
    })
    .unwrap();
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    let stage = postgres_kernel(&config, &staged, false, false);
    let (at, held) = copy_after_writes(
        &source,
        &calls,
        || (extend(&store, 30), propose_only(&store, 40)),
        &stage,
    );
    drop(stage);
    assert!(store.head().unwrap().unwrap().revision > at.revision);
    let stage = postgres_kernel(&config, &staged, true, true);
    assert_eq!(
        kept(stage.head().unwrap().unwrap()),
        kept(at),
        "the stage's head"
    );
    assert_eq!(transactions(&stage), held, "the stage's transactions");
}

/// The same on SQLite, from a live source handle rather than an image: a copy that read the
/// source again would see the writes made after the capture.
#[test]
fn a_sqlite_copy_reads_nothing_of_the_source_after_its_capture() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.db");
    let tenant = tenant();
    let store = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    populate(&store);
    let calls = Calls::default();
    let source = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(Watched {
            inner: SqliteStore::sqlite(&path, &tenant, None)?.under(authority),
            calls: calls.clone(),
        })
    })
    .unwrap();
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    let stage = sqlite_kernel(&path, &staged, false);
    let (at, held) = copy_after_writes(
        &source,
        &calls,
        || (extend(&store, 30), propose_only(&store, 40)),
        &stage,
    );
    drop(stage);
    assert!(store.head().unwrap().unwrap().revision > at.revision);
    let stage = sqlite_kernel(&path, &staged, true);
    assert_eq!(
        kept(stage.head().unwrap().unwrap()),
        kept(at),
        "the stage's head"
    );
    assert_eq!(transactions(&stage), held, "the stage's transactions");
}

/// COORDINATOR § 2, run as the mutant: `postgres_runtime_reopens_seed_and_committed_history` and
/// `a_postgres_store_is_copied_into_a_stage_under_one_capture` hold "one provider capture and no
/// stream or feed read" with `stream_reads() == StreamReads { captures: 1, .. }`. This case makes
/// the one capture and then a second, complete read of the source's tenant log after it — what
/// an inventory that also took its event count or its preparations from the change feed would
/// do — and asks that predicate whether it sees the second read.
#[test]
fn the_one_capture_predicate_sees_a_feed_read_after_the_capture() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    populate(&store);
    let raw = PostgresStore::postgres(&config, &tenant, false).unwrap();
    let _ = ekr_kernel::stream_reads();
    raw.inventory().unwrap();
    let fed = raw.published_events().unwrap();
    assert!(!fed.is_empty(), "the feed read returned the tenant's log");
    assert_ne!(
        ekr_kernel::stream_reads(),
        StreamReads {
            captures: 1,
            ..StreamReads::default()
        },
        "a complete feed read of the source after its capture leaves the unit's \
         one-capture-and-no-feed-read predicate true"
    );
}

/// What `Commit::capture` answers for a stage tenant, once on the checkpoint path and once in
/// full: its refusal, or a line saying it was captured.
fn captures<S: RevisionLog + ObjectStore + Inventory>(
    open: impl Fn(bool) -> Commit<S>,
) -> Vec<String> {
    [false, true]
        .into_iter()
        .map(|full| {
            let stage = open(full);
            let _ = stage.head();
            match stage.capture() {
                Ok(_) => format!("full={full}: the interrupted stage was captured"),
                Err(error) => format!("full={full}: {error}"),
            }
        })
        .collect()
}

/// An interrupted stage copy cannot be laundered into a complete one: a capture of the
/// interrupted stage, which is what a stage of a stage or a migration of it would copy from,
/// refuses `migrate-incomplete` like every other read, on a fresh writing handle.
fn interrupted_stage_cannot_be_captured<S, D>(
    captured: &ekr_kernel::CapturedStore<'_, S>,
    store_tenant: &str,
    open: impl Fn(&str, ekr_kernel::KernelAuthority) -> D,
    capture: impl Fn(&str) -> Vec<String>,
) where
    S: RevisionLog + ObjectStore + Inventory,
    D: RevisionLog + ObjectStore + ekr_store::Initialize + Inventory,
{
    for (after_seed, completion) in [(true, false), (false, true)] {
        let staged = ekr_store::stage_tenant(store_tenant, stage_id()).unwrap();
        let into = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(controlled::Controlled {
                inner: open(&staged, authority),
                before_initialize: Box::new(|| {}),
                interrupt_after_seed: after_seed,
                interrupt_completion: completion,
            })
        })
        .unwrap();
        captured.copy_into(&into).unwrap_err();
        drop(into);
        for refused in capture(&staged) {
            assert!(
                refused.contains("migrate-incomplete"),
                "after_seed={after_seed} completion={completion}: {refused}"
            );
        }
    }
}

#[test]
fn an_interrupted_stage_copy_cannot_be_captured_on_sqlite() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.db");
    let tenant = tenant();
    let store = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    populate(&store);
    let source = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    let captured = source.capture().unwrap();
    interrupted_stage_cannot_be_captured(
        &captured,
        &tenant,
        |staged, authority| {
            SqliteStore::sqlite(&path, staged, None)
                .unwrap()
                .under(authority)
        },
        |staged| captures(|full| sqlite_kernel(&path, staged, full)),
    );
}

#[test]
fn an_interrupted_stage_copy_cannot_be_captured_on_postgres() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    populate(&store);
    let source = postgres_kernel(&config, &tenant, false, false);
    let captured = source.capture().unwrap();
    interrupted_stage_cannot_be_captured(
        &captured,
        &tenant,
        |staged, authority| {
            PostgresStore::postgres(&config, staged, false)
                .unwrap()
                .under(authority)
        },
        |staged| captures(|full| postgres_kernel(&config, staged, false, full)),
    );
}
