//! A run staged and published whole (design § 107, `story:a-run-is-staged-and-published-whole`).
//!
//! Where each case runs is design § 107.10's: `a_stage_tenant_is_never_a_store_tenant` on File
//! and SQLite, `a_stage_is_refused_on_the_file_provider` on File, and every other case on SQLite
//! — and each on PostgreSQL where the PostgreSQL tests run: set EKR_TEST_POSTGRES_CONFIG and
//! EKR_TEST_POSTGRES_OWNER to file references; EKR_REQUIRE_POSTGRES=1 makes their absence a
//! failure. Run them with `--test-threads=1` there.
//!
//! Interleavings are made with `ekr_store::on_stage_point`, which runs another writer, or an
//! interruption, at a named point of a stage command on the calling thread. No case reads a clock.
#[path = "support/hosted_store.rs"]
#[allow(dead_code)]
mod controlled;
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::{ContentHash, RevisionNumber, StageId, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, Commit, CommitCommandResult, CommitError,
    GraphOperation, GraphTransaction, KernelAuthority, PersistenceError, Runtime, TransactionState,
    ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::NodeType;
use ekr_store::{
    Initialize, Inventory, ObjectStore, PostgresStore, ProviderKind, PublishedEvent, RevisionLog,
    SqliteStore, StageLog, StagePoint, StageResult, StageState, StorageClass, StoreError,
    STAGE_TENANT_MARKER,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The hosted PostgreSQL configuration, with its owner schema applied once, or `None` where the
/// PostgreSQL tests do not run.
fn postgres() -> Option<PostgresConfiguration> {
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
        Runtime::postgres_schema(
            &PostgresConfiguration::read(std::path::Path::new(&owner)).unwrap(),
        )
        .unwrap();
    });
    Some(PostgresConfiguration::read(std::path::Path::new(&path)).unwrap())
}

/// `opened` was refused as `stage-tenant-reserved`, and is reported as that refusal.
fn refused(what: &str, opened: Result<Runtime, PersistenceError>) {
    let error = opened.err().unwrap_or_else(|| panic!("{what} opened"));
    assert!(
        matches!(error, PersistenceError::StageTenantReserved(_)),
        "{what}: {error}"
    );
    assert!(
        error.to_string().starts_with("stage-tenant-reserved: "),
        "{what}: {error}"
    );
}

/// Design § 107.10, unit C: the derivation never yields a store's tenant — every name it gives
/// carries the stage marker and fits an Eventlog tenant for every store tenant, a 512-byte one
/// included — and a host configuration whose tenant carries the marker opens no store, on any
/// provider, and writes nothing. A stage id is minted as begin mints it (`ekr.store.StageId`).
#[test]
fn a_stage_tenant_is_never_a_store_tenant() {
    let minted = ekr_core::StageId::mint();
    let id = minted;
    let other = ekr_core::StageId::mint();
    let longest = "t".repeat(512);
    let stores = [
        "hosted-store",
        "a",
        "a tenant with spaces",
        "colons:and/slashes",
        "~",
        longest.as_str(),
    ];
    let mut derived = BTreeSet::new();
    for store in stores {
        let staged = ekr_store::stage_tenant(store, id).unwrap();
        assert_ne!(staged, store);
        assert!(staged.starts_with(STAGE_TENANT_MARKER), "{staged}");
        assert!(
            staged.ends_with(&format!(":{minted}")),
            "the name reads the stage id: {staged}"
        );
        assert_eq!(
            staged.len(),
            STAGE_TENANT_MARKER.len() + 64 + 1 + 36,
            "a fixed length whatever the store tenant's: {staged}"
        );
        assert!(
            staged.len() <= 512 && staged.bytes().all(|b| b.is_ascii_graphic() || b == b' '),
            "an Eventlog tenant: {staged}"
        );
        assert_eq!(staged, ekr_store::stage_tenant(store, id).unwrap());
        assert_ne!(staged, ekr_store::stage_tenant(store, other).unwrap());
        assert!(derived.insert(staged), "another store tenant, another name");
    }
    // A store tenant carrying the marker has no stage.
    assert!(ekr_store::stage_tenant(&format!("x{STAGE_TENANT_MARKER}y"), id).is_err());

    // A host configuration whose tenant carries the marker opens no store, on any provider, and
    // writes nothing: so no store's tenant is ever a stage's.
    let directory = tempfile::tempdir().unwrap();
    let file_store = directory.path().join("file-store");
    let sqlite_store = directory.path().join("store.db");
    Runtime::file(&file_store, "hosted-store", context(), anchor())
        .unwrap()
        .seed(seed(), || SEEDED_AT)
        .unwrap();
    Runtime::sqlite(&sqlite_store, "hosted-store", context(), anchor())
        .unwrap()
        .seed(seed(), || SEEDED_AT)
        .unwrap();
    let staged = derived.first().unwrap().clone();
    let reserved = [
        staged.clone(),
        STAGE_TENANT_MARKER.to_owned(),
        format!("team {STAGE_TENANT_MARKER} tenant"),
    ];
    for tenant in &reserved {
        let new_file = directory.path().join("new-file-store");
        let new_sqlite = directory.path().join("new.db");
        refused(
            "file",
            Runtime::file(&new_file, tenant, context(), anchor()),
        );
        refused(
            "sqlite",
            Runtime::sqlite(&new_sqlite, tenant, context(), anchor()),
        );
        assert!(
            !new_file.exists() && !new_sqlite.exists(),
            "nothing created"
        );
        refused(
            "file_existing",
            Runtime::file_existing(&file_store, tenant, context(), anchor()),
        );
        refused(
            "file_reading",
            Runtime::file_reading(&file_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_existing",
            Runtime::sqlite_existing(&sqlite_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_reading",
            Runtime::sqlite_reading(&sqlite_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_snapshot",
            Runtime::sqlite_snapshot(&sqlite_store, tenant, context(), anchor()),
        );
    }
    // The stage's own tenant is an Eventlog tenant the store level opens: the derivation, not a
    // host configuration, is how a stage's tenant is reached.
    Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(SqliteStore::sqlite(&sqlite_store, &staged, None)?.under(authority))
    })
    .unwrap();

    let Some(config) = postgres() else { return };
    let derived = ekr_store::stage_tenant(&format!("hosted-{}", TypeId::mint()), id).unwrap();
    for tenant in [derived, format!("team {STAGE_TENANT_MARKER} tenant")] {
        for reading in [false, true] {
            refused(
                "postgres",
                Runtime::postgres(&config, &tenant, context(), anchor(), reading),
            );
        }
    }
}

// Unit P (`task:stage-suffix-publication`), design § 107.10.

/// The host anchor these stores are seeded under: the schema-evolving profile, so a run's
/// transactions can define node types.
fn evolving() -> AuthorityStateV1 {
    let mut anchor = anchor();
    anchor.validation_profile = ValidationProfileV1::schema_evolving(context().validator);
    anchor
}

/// A store under test: a SQLite database, or a tenant of the hosted PostgreSQL schema.
struct Fixture {
    directory: tempfile::TempDir,
    config: Option<PostgresConfiguration>,
    tenant: String,
}

/// A provider handle on one tenant of a fixture's store, with no kernel authority: it counts,
/// reads and stores objects, and decides nothing.
enum Raw {
    Sqlite(SqliteStore),
    Postgres(PostgresStore),
}

impl Raw {
    fn log(&self) -> &dyn StageLog {
        match self {
            Self::Sqlite(store) => store,
            Self::Postgres(store) => store,
        }
    }
    /// Every event the tenant holds: the provider log on SQLite, one capture on PostgreSQL.
    fn events(&self) -> usize {
        match self {
            Self::Sqlite(store) => store.published_events().unwrap().len(),
            Self::Postgres(store) => store.inventory().unwrap().events,
        }
    }
    fn published(&self) -> Vec<PublishedEvent> {
        match self {
            Self::Sqlite(store) => store.published_events().unwrap(),
            Self::Postgres(store) => store.published_events().unwrap(),
        }
    }
    fn put(&self, class: StorageClass, bytes: &[u8]) {
        match self {
            Self::Sqlite(store) => store.put(class, bytes, Timestamp::EPOCH).unwrap(),
            Self::Postgres(store) => store.put(class, bytes, Timestamp::EPOCH).unwrap(),
        };
    }
    fn get(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        match self {
            Self::Sqlite(store) => store.get(hash).unwrap(),
            Self::Postgres(store) => store.get(hash).unwrap(),
        }
    }
}

impl Fixture {
    fn sqlite() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            config: None,
            tenant: "staged-store".into(),
        }
    }
    fn postgres(config: PostgresConfiguration) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            config: Some(config),
            tenant: format!("staged-{}", TypeId::mint()),
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("store.db")
    }
    fn open(&self) -> Runtime {
        open_at(&self.path(), self.config.as_ref(), &self.tenant)
    }
    /// What opens this fixture's store again, for a writer a hook runs.
    fn opener(&self) -> impl Fn() -> Runtime + 'static {
        let (path, config, tenant) = (self.path(), self.config.clone(), self.tenant.clone());
        move || open_at(&path, config.as_ref(), &tenant)
    }
    fn raw(&self, tenant: &str) -> Raw {
        match &self.config {
            None => Raw::Sqlite(SqliteStore::sqlite(&self.path(), tenant, None).unwrap()),
            Some(config) => Raw::Postgres(PostgresStore::postgres(config, tenant, false).unwrap()),
        }
    }
    /// How many events `tenant` of this store holds.
    fn holds(&self, tenant: &str) -> usize {
        self.raw(tenant).events()
    }
    /// The tenant of `stage` of this store.
    fn stage_tenant(&self, stage: StageId) -> String {
        ekr_store::stage_tenant(&self.tenant, stage).unwrap()
    }
    /// The store, seeded and committed once.
    fn seeded(&self) -> Runtime {
        let store = self.open();
        store.seed(seed(), || SEEDED_AT).unwrap();
        commit(&store);
        store
    }
}

fn open_at(path: &Path, config: Option<&PostgresConfiguration>, tenant: &str) -> Runtime {
    match config {
        None => Runtime::sqlite(path, tenant, context(), evolving()),
        Some(config) => Runtime::postgres(config, tenant, context(), evolving(), false),
    }
    .unwrap()
}

/// Runs `case` on SQLite, and on PostgreSQL where the PostgreSQL tests run.
fn each(case: impl Fn(&Fixture)) {
    case(&Fixture::sqlite());
    if let Some(config) = postgres() {
        case(&Fixture::postgres(config));
    }
}

/// The next instant of the run's commands: later than every one before it. A counter, not a clock.
fn at() -> i64 {
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_000);
    NEXT.fetch_add(10, std::sync::atomic::Ordering::Relaxed)
}

/// A transaction defining one new node type.
fn define() -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Staged{}", TypeId::mint()),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    }
}

fn document(transaction: &GraphTransaction) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction,
    })
    .unwrap()
    .into_bytes()
}

/// Proposes `transaction` through `runtime`.
fn try_propose(runtime: &Runtime, transaction: &GraphTransaction) -> Result<(), CommitError> {
    let at = at();
    runtime
        .propose(&document(transaction), context().operator, || {
            Timestamp::from_millis(at)
        })
        .map(|_| ())
}

/// A proposal that stays Proposed.
fn propose(runtime: &Runtime) -> TransactionId {
    let transaction = define();
    try_propose(runtime, &transaction).unwrap();
    transaction.id
}

/// A transaction proposed, validated and committed: three occurrences, one revision.
fn commit(runtime: &Runtime) -> TransactionId {
    commit_transaction(runtime, &define())
}

/// A transaction adding evidence, whose payload a commit publishes as a Provenance object, and an
/// assertion on the seed's first node citing it.
fn evidence(payload: &[u8]) -> GraphTransaction {
    let id = ekr_core::EvidenceId::mint();
    let assertion = ekr_graph::Assertion {
        id: ekr_core::AssertionId::mint(),
        root_id: current_fixture::id(0x02),
        subject: ekr_graph::Subject::Node(current_fixture::id(0x10)),
        predicate: ekr_graph::Predicate::Property(current_fixture::id(0x06)),
        object: ekr_graph::Object::Value(ekr_ontology::Value::String("staged".into())),
        evidence: BTreeSet::from([id]),
        proposed_by: context().operator,
        assessment: ekr_graph::Assessment::Proposed,
        lifecycle: ekr_graph::AssertionLifecycle::Active,
        valid_time: ekr_graph::TemporalRange::UNBOUNDED,
        transaction_time: ekr_graph::TransactionTime::since(Timestamp::EPOCH),
    };
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::AddEvidence(Box::new(ekr_kernel::EvidenceAddition {
                evidence: ekr_graph::Evidence {
                    id,
                    source: ekr_graph::EvidenceSource::HumanStatement {
                        identity: Some("operator".into()),
                    },
                    content_hash: ContentHash::of_bytes(payload),
                    extracted_by: context().operator,
                    observed_at: Timestamp::from_millis(5),
                    confidence: ekr_graph::Confidence::CERTAIN,
                },
                payload: payload.to_vec(),
            })),
            GraphOperation::AddAssertion(Box::new(assertion)),
        ],
        evidence: BTreeSet::from([id]),
        schema_version: None,
    }
}

/// `transaction` proposed, validated and committed through `runtime`.
fn commit_transaction(runtime: &Runtime, transaction: &GraphTransaction) -> TransactionId {
    try_propose(runtime, transaction).unwrap();
    let id = transaction.id;
    let at = at();
    let verdict = runtime
        .validate(id, runtime.head().unwrap().unwrap().revision, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let at = at + 1;
    let committed = runtime
        .commit(id, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{committed:?}"
    );
    id
}

/// The store refusal inside `result`.
fn refusal<T: std::fmt::Debug>(result: Result<T, CommitError>) -> StoreError {
    match result {
        Ok(admitted) => panic!("admitted: {admitted:?}"),
        Err(CommitError::Store(error)) => error,
        Err(other) => panic!("not a store refusal: {other}"),
    }
}

/// `error` is refused under `name`.
fn named(error: &StoreError, name: &str) {
    assert!(
        error.to_string().starts_with(&format!("{name}:")),
        "expected {name}: {error}"
    );
}

/// What a hook returns for a command's refusal.
fn store_error(error: CommitError) -> StoreError {
    match error {
        CommitError::Store(error) => error,
        other => StoreError::Backend(other.to_string()),
    }
}

/// The store's revision stream as its provider log holds it.
fn revision_stream(runtime: &Runtime) -> Vec<PublishedEvent> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.stream_type == "ekr.revision")
        .collect()
}

/// The `ekr.store.PublicationPrepared` selections the store holds: one per elected attempt.
fn elections(runtime: &Runtime) -> usize {
    runtime
        .published_events()
        .unwrap()
        .iter()
        .filter(|event| event.name == "ekr.store.PublicationPrepared")
        .count()
}

/// A revision event as the log holds it.
fn occurrence(event: &PublishedEvent) -> ekr_graph::RevisionEvent {
    serde_json::from_value(event.data.clone()).unwrap()
}

/// The identities and roots of `root` a preserving copy keeps: the number and the ontology,
/// knowledge, evidence and authority roots.
fn roots(root: &ekr_graph::Root) -> [String; 5] {
    [
        root.revision.to_string(),
        root.ontology_root.to_string(),
        root.knowledge_root.to_string(),
        root.evidence_root.to_string(),
        root.agent_root.to_string(),
    ]
}

/// The state the store's record holds for `stage`.
fn state(store: &Runtime, stage: StageId) -> StageState {
    store
        .stages()
        .unwrap()
        .into_iter()
        .find(|listed| listed.stage.stage_id == stage)
        .expect("the store records the stage")
        .state
}

/// A stage begun on `store` and a run of one commit in it.
fn staged(store: &Runtime) -> StageResult {
    let begun = store.begin_stage().unwrap();
    commit(&store.join_stage(begun.stage_id).unwrap());
    begun
}

#[test]
fn a_stage_begins_at_the_head_and_the_store_is_unchanged() {
    each(|fixture| {
        let store = fixture.seeded();
        let before = store.head().unwrap().unwrap();
        let stream = revision_stream(&store);
        let begun = store.begin_stage().unwrap();
        assert_eq!(begun.state, StageState::Begun);
        assert_eq!(begun.base, before.revision);
        assert_eq!(
            (
                begun.published_first,
                begun.published_last,
                begun.occurrences
            ),
            (None, None, None)
        );
        assert_eq!(
            store.head().unwrap(),
            Some(before),
            "the head does not move"
        );
        assert_eq!(revision_stream(&store), stream, "nor the revision stream");
        let joined = store.join_stage(begun.stage_id).unwrap();
        assert_eq!(joined.joined_stage(), Some(begun.stage_id));
        assert_eq!(
            roots(&joined.head().unwrap().unwrap()),
            roots(&before),
            "the stage is the store at its head"
        );
        assert!(fixture.holds(&fixture.stage_tenant(begun.stage_id)) > 0);
        let listed = store.stages().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].state, StageState::Begun);
        assert_eq!(listed[0].stage.base, before.revision);
        assert_eq!(listed[0].stage.base_revision, begun.base_revision);
        assert_eq!(listed[0].stage.store.0, fixture.tenant);
    });
}

#[test]
fn a_stage_is_refused_on_the_file_provider() {
    let directory = tempfile::tempdir().unwrap();
    let store = Runtime::file(
        &directory.path().join("file-store"),
        "staged-store",
        context(),
        evolving(),
    )
    .unwrap();
    store.seed(seed(), || SEEDED_AT).unwrap();
    let events = store.published_events().unwrap();
    let error = refusal(store.begin_stage());
    assert_eq!(
        error,
        StoreError::StageUnsupportedProvider(ProviderKind::File)
    );
    named(&error, "stage-unsupported-provider");
    let stage = StageId::mint();
    let head = RevisionNumber::SEED;
    for refused in [
        store.seal_stage(stage, head),
        store.publish_stage(stage, head),
        store.seal_and_publish_stage(stage, head),
        store.abandon_stage(stage),
    ] {
        named(&refusal(refused), "stage-unsupported-provider");
    }
    named(
        &refusal(store.join_stage(stage).map(|_| ())),
        "stage-unsupported-provider",
    );
    assert_eq!(
        store.published_events().unwrap(),
        events,
        "nothing was recorded or copied"
    );
}

#[test]
fn a_published_stage_lands_whole_and_the_store_replays_in_full() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        let payload = format!("a statement the run retains {}", TypeId::mint()).into_bytes();
        let run = [
            commit(&joined),
            commit_transaction(&joined, &evidence(&payload)),
            propose(&joined),
        ];
        let staged = joined.head().unwrap().unwrap();
        let states = joined.transaction_states(run).unwrap();
        drop(joined);
        // The run's validation receipts and commit receipts name the stage's own lineage (its seed
        // envelope, its prior records): the publication derives them again for the store's.
        let stage_records: Vec<ContentHash> = fixture
            .raw(&fixture.stage_tenant(begun.stage_id))
            .published()
            .iter()
            .filter(|event| {
                event.stream_type == "ekr.revision"
                    && matches!(
                        event.name.as_str(),
                        "ekr.kernel.TransactionValidated" | "ekr.kernel.RevisionCommitted"
                    )
            })
            .map(|event| occurrence(event).record_hash)
            .collect();
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.state, StageState::Published);
        assert!(
            stage_records.len() >= 4,
            "the run's two validations and two commits"
        );
        for record in &stage_records {
            assert!(
                fixture.open().content(record).unwrap().is_none(),
                "no record of the stage's lineage enters the store (adversary F3): {record}"
            );
        }
        assert_eq!(published.occurrences, Some(7), "two commits and a proposal");
        let mut replayed = fixture.open();
        replayed.set_full_replay(true);
        let head = replayed.head().unwrap().unwrap();
        assert_eq!(
            roots(&head),
            roots(&staged),
            "ekr head is the stage's last revision, and replay from the seed reaches it"
        );
        assert_eq!(head.revision, RevisionNumber::new(begun.base.get() + 2));
        replayed.snapshot().unwrap();
        assert_eq!(replayed.transaction_states(run).unwrap(), states);
        assert_eq!(
            replayed.content(&ContentHash::of_bytes(&payload)).unwrap(),
            Some(payload.clone()),
            "the evidence payload the run's commit added is the store's"
        );
        assert_eq!(
            fixture
                .raw(&fixture.tenant)
                .log()
                .held_classes(&BTreeSet::from([ContentHash::of_bytes(&payload)]))
                .unwrap()
                .get(&ContentHash::of_bytes(&payload)),
            Some(&StorageClass::Provenance)
        );
        let added: Vec<_> = revision_stream(&replayed)
            .iter()
            .filter_map(|event| match occurrence(event).payload {
                ekr_graph::RevisionPayload::RevisionCommitted {
                    revision_id,
                    number,
                    ..
                } if number > begun.base => Some(revision_id),
                _ => None,
            })
            .collect();
        assert_eq!(
            (published.published_first, published.published_last),
            (added.first().copied(), added.last().copied())
        );
        assert_eq!(added.len(), 2);
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
    });
}

#[test]
fn a_publish_whose_expected_head_is_not_the_head_changes_nothing() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let events = store.published_events().unwrap();
        let wrong = RevisionNumber::new(begun.base.get() + 1);
        let error = refusal(store.seal_and_publish_stage(begun.stage_id, wrong));
        named(&error, "stage-head-moved");
        assert_eq!(
            error,
            StoreError::StageHeadMoved {
                stage_id: begun.stage_id,
                expected: wrong,
                base: begun.base,
                current: begun.base,
            }
        );
        named(
            &refusal(store.publish_stage(begun.stage_id, begun.base)),
            "stage-not-sealed",
        );
        assert_eq!(store.published_events().unwrap(), events, "nothing written");
        assert_eq!(state(&store, begun.stage_id), StageState::Begun);
        store.seal_stage(begun.stage_id, begun.base).unwrap();
        let events = store.published_events().unwrap();
        named(
            &refusal(store.publish_stage(begun.stage_id, wrong)),
            "stage-head-moved",
        );
        assert_eq!(store.published_events().unwrap(), events, "nothing written");
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
    });
}

#[test]
fn a_publish_after_the_store_moved_since_the_base_is_refused_by_name() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        commit(&store);
        let moved = store.head().unwrap().unwrap().revision;
        let events = store.published_events().unwrap();
        for expected in [begun.base, moved] {
            assert_eq!(
                refusal(store.seal_and_publish_stage(begun.stage_id, expected)),
                StoreError::StageHeadMoved {
                    stage_id: begun.stage_id,
                    expected,
                    base: begun.base,
                    current: moved,
                },
                "never rebased"
            );
        }
        assert_eq!(store.published_events().unwrap(), events, "nothing written");
        assert_eq!(state(&store, begun.stage_id), StageState::Begun);
    });
}

#[test]
fn a_head_that_moves_after_the_seal_leaves_the_stage_sealing_and_the_store_unchanged() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        assert_eq!(
            store.seal_stage(begun.stage_id, begun.base).unwrap().state,
            StageState::Sealing
        );
        commit(&store);
        let events = store.published_events().unwrap();
        named(
            &refusal(store.publish_stage(begun.stage_id, begun.base)),
            "stage-head-moved",
        );
        assert_eq!(store.published_events().unwrap(), events, "nothing written");
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);

        // A commit landing between the capture and the append: the group is refused whole.
        let second = staged(&store);
        let stream = revision_stream(&store).len();
        let open = fixture.opener();
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected {
                commit(&open());
            }
            Ok(())
        });
        let error = refusal(store.seal_and_publish_stage(second.stage_id, second.base));
        drop(hook);
        named(&error, "stage-head-moved");
        assert_eq!(
            revision_stream(&store).len(),
            stream + 3,
            "the racing commit alone"
        );
        assert_eq!(state(&store, second.stage_id), StageState::Sealing);
        assert_eq!(
            store.abandon_stage(second.stage_id).unwrap().state,
            StageState::Abandoned
        );
    });
}

#[test]
fn a_write_joined_to_a_sealed_stage_is_refused_by_name() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        commit(&joined);
        store.seal_stage(begun.stage_id, begun.base).unwrap();
        let tenant = fixture.stage_tenant(begun.stage_id);
        let held = fixture.holds(&tenant);
        named(&refusal(try_propose(&joined, &define())), "stage-sealed");
        named(
            &refusal(store.join_stage(begun.stage_id).map(|_| ())),
            "stage-sealed",
        );
        assert_eq!(fixture.holds(&tenant), held, "nothing was written");

        // The seal between a write's check and its append (adversary F6): the write lands and is
        // refused by name, never reported successful.
        let other = store.begin_stage().unwrap();
        let joined = store.join_stage(other.stage_id).unwrap();
        let (open, stage, base) = (fixture.opener(), other.stage_id, other.base);
        let mut sealed = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::JoinedWrite && !sealed {
                sealed = true;
                open().seal_stage(stage, base).map_err(store_error)?;
            }
            Ok(())
        });
        let error = refusal(try_propose(&joined, &define()));
        drop(hook);
        named(&error, "stage-write-landed");
        assert!(
            matches!(
                error,
                StoreError::StageWriteLanded {
                    state: StageState::Sealing,
                    ref event_ids,
                    ..
                } if event_ids.is_empty()
            ),
            "{error}"
        );
        // What landed is the write's election, which the seal did not see: the stage holds a
        // decision elected and never published, so it cannot be published and is abandoned.
        named(
            &refusal(store.publish_stage(other.stage_id, other.base)),
            "unresolved-preparation",
        );
        store.abandon_stage(other.stage_id).unwrap();
        assert_eq!(fixture.holds(&fixture.stage_tenant(other.stage_id)), 0);
    });
}

#[test]
fn a_write_reported_successful_is_in_the_publication() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        let committed = [commit(&joined), commit(&joined)];
        let proposed = propose(&joined);
        // The whole publication runs between a further write's check and its append (adversary
        // F6): the write lands in a tenant already forgotten and is refused by name.
        let (open, stage, base) = (fixture.opener(), begun.stage_id, begun.base);
        let mut ran = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::JoinedWrite && !ran {
                ran = true;
                open()
                    .seal_and_publish_stage(stage, base)
                    .map_err(store_error)?;
            }
            Ok(())
        });
        let late = define();
        let error = refusal(try_propose(&joined, &late));
        drop(hook);
        named(&error, "stage-write-landed");
        assert!(
            matches!(
                error,
                StoreError::StageWriteLanded {
                    state: StageState::Published,
                    ..
                }
            ),
            "{error}"
        );
        let states = fixture
            .open()
            .transaction_states([committed[0], committed[1], proposed, late.id])
            .unwrap();
        assert_eq!(
            states.get(&committed[0]),
            Some(&TransactionState::Committed)
        );
        assert_eq!(
            states.get(&committed[1]),
            Some(&TransactionState::Committed)
        );
        assert_eq!(states.get(&proposed), Some(&TransactionState::Proposed));
        assert_eq!(
            states.get(&late.id),
            None,
            "the refused write is not published"
        );
        assert_eq!(
            fixture.holds(&fixture.stage_tenant(begun.stage_id)),
            0,
            "and left nothing in the stage's tenant (adversary F1)"
        );
    });
}

#[test]
fn a_publish_retried_after_an_unknown_outcome_adopts_its_publication() {
    each(|fixture| {
        let store = fixture.seeded();
        // The group landed and its answer was lost.
        let begun = staged(&store);
        let hook = ekr_store::on_stage_point(|point| {
            if point == StagePoint::PublishAppended {
                Err(StoreError::UnknownCommit)
            } else {
                Ok(())
            }
        });
        assert_eq!(
            refusal(store.seal_and_publish_stage(begun.stage_id, begun.base)),
            StoreError::UnknownCommit
        );
        drop(hook);
        let stream = revision_stream(&store);
        let retried = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(retried.state, StageState::Published);
        assert_eq!(retried.occurrences, Some(3));
        assert_eq!(revision_stream(&store), stream, "the retry appends nothing");
        assert_eq!(
            store
                .seal_and_publish_stage(begun.stage_id, begun.base)
                .unwrap(),
            retried,
            "every retry returns the original result"
        );
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);

        // The answer was lost before the group was appended: the retry appends the elected
        // attempt itself, once.
        let second = staged(&store);
        let hook = ekr_store::on_stage_point(|point| {
            if point == StagePoint::PublishElected {
                Err(StoreError::UnknownCommit)
            } else {
                Ok(())
            }
        });
        assert_eq!(
            refusal(store.seal_and_publish_stage(second.stage_id, second.base)),
            StoreError::UnknownCommit
        );
        drop(hook);
        let (stream, elected) = (revision_stream(&store).len(), elections(&store));
        let published = store
            .seal_and_publish_stage(second.stage_id, second.base)
            .unwrap();
        assert_eq!(published.state, StageState::Published);
        assert_eq!(
            revision_stream(&store).len(),
            stream + 3,
            "the suffix, once"
        );
        assert_eq!(
            elections(&store),
            elected,
            "the elected attempt, not another"
        );
    });
}

#[test]
fn a_publish_interrupted_after_the_seal_is_finished_by_its_retry() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let stream = revision_stream(&store);
        let hook = ekr_store::on_stage_point(|point| {
            if point == StagePoint::Sealed {
                Err(StoreError::Backend("interrupted after the seal".into()))
            } else {
                Ok(())
            }
        });
        assert_eq!(
            refusal(store.seal_and_publish_stage(begun.stage_id, begun.base)),
            StoreError::Backend("interrupted after the seal".into())
        );
        drop(hook);
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
        assert_eq!(revision_stream(&store), stream);
        named(
            &refusal(store.join_stage(begun.stage_id).map(|_| ())),
            "stage-sealed",
        );
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.state, StageState::Published);
        assert_eq!(published.occurrences, Some(3));
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
    });
}

#[test]
fn a_publish_interrupted_after_the_append_is_finished_by_its_retry() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let hook = ekr_store::on_stage_point(|point| {
            if point == StagePoint::PublishAppended {
                Err(StoreError::Backend("interrupted after the append".into()))
            } else {
                Ok(())
            }
        });
        refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        let tenant = fixture.stage_tenant(begun.stage_id);
        assert_eq!(state(&store, begun.stage_id), StageState::Published);
        assert!(
            fixture.holds(&tenant) > 0,
            "the tenant is not forgotten yet"
        );
        let stream = revision_stream(&store);
        named(
            &refusal(store.join_stage(begun.stage_id).map(|_| ())),
            "stage-already-published",
        );
        named(
            &refusal(store.abandon_stage(begun.stage_id)),
            "stage-already-published",
        );
        named(
            &refusal(
                store.seal_and_publish_stage(
                    begun.stage_id,
                    RevisionNumber::new(begun.base.get() + 1),
                ),
            ),
            "stage-already-published",
        );
        let retried = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(retried.state, StageState::Published);
        assert_eq!(retried.occurrences, Some(3));
        assert_eq!(revision_stream(&store), stream, "nothing appended twice");
        assert_eq!(fixture.holds(&tenant), 0, "the retry forgets the tenant");
    });
}

#[test]
fn a_stage_with_an_unresolved_preparation_is_not_sealed() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        // The run's proposal is elected and interrupted before its publication.
        let mut writes = 0;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::JoinedWrite {
                writes += 1;
                if writes == 2 {
                    return Err(StoreError::Backend("interrupted after the election".into()));
                }
            }
            Ok(())
        });
        let transaction = define();
        let bytes = document(&transaction);
        let at = at();
        joined
            .propose(&bytes, context().operator, || Timestamp::from_millis(at))
            .unwrap_err();
        drop(hook);
        let error = refusal(store.seal_stage(begun.stage_id, begun.base));
        named(&error, "unresolved-preparation");
        assert!(matches!(error, StoreError::UnresolvedPreparation(_)));
        named(
            &refusal(store.seal_and_publish_stage(begun.stage_id, begun.base)),
            "unresolved-preparation",
        );
        assert_eq!(state(&store, begun.stage_id), StageState::Begun);
        // The command that elected it resolves it; then the stage seals and publishes.
        joined
            .propose(&bytes, context().operator, || Timestamp::from_millis(at))
            .unwrap();
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.occurrences, Some(1));
        assert_eq!(
            fixture
                .open()
                .transaction_states([transaction.id])
                .unwrap()
                .get(&transaction.id),
            Some(&TransactionState::Proposed)
        );
    });
}

/// The revision identity of `runtime`'s head.
fn head_revision(runtime: &Runtime) -> ekr_core::RevisionId {
    revision_stream(runtime)
        .iter()
        .rev()
        .find_map(|event| match occurrence(event).payload {
            ekr_graph::RevisionPayload::Seeded { revision_id, .. }
            | ekr_graph::RevisionPayload::RevisionCommitted { revision_id, .. } => {
                Some(revision_id)
            }
            _ => None,
        })
        .unwrap()
}

/// Copies the store `source` opens into the stage tenant `destination` opens, through a
/// destination that refuses the copy's completion receipt: a begin interrupted inside its copy.
fn interrupted_copy<S, D>(
    source: impl FnOnce(KernelAuthority) -> Result<S, StoreError>,
    destination: impl FnOnce(KernelAuthority) -> Result<D, StoreError>,
) where
    S: RevisionLog + ObjectStore + Inventory,
    D: RevisionLog + ObjectStore + Initialize + Inventory,
{
    let source = Commit::over_with_authority(context(), evolving(), source).unwrap();
    let captured = source.capture().unwrap();
    let destination = Commit::over_with_authority(context(), evolving(), |authority| {
        Ok(controlled::Controlled {
            inner: destination(authority)?,
            before_initialize: Box::new(|| {}),
            interrupt_after_seed: false,
            interrupt_completion: true,
        })
    })
    .unwrap();
    assert!(
        captured.copy_into(&destination).is_err(),
        "the copy is interrupted"
    );
}

#[test]
fn an_incomplete_stage_refuses_seal_and_is_abandoned() {
    each(|fixture| {
        let store = fixture.seeded();
        let head = store.head().unwrap().unwrap().revision;
        let raw = fixture.raw(&fixture.tenant);
        // A begin interrupted after its record, before the copy wrote anything.
        let stage = StageId::mint();
        raw.log()
            .record_stage_begun(
                stage,
                &fixture.stage_tenant(stage),
                head,
                head_revision(&store),
            )
            .unwrap();
        named(&refusal(store.seal_stage(stage, head)), "stage-incomplete");
        named(
            &refusal(store.join_stage(stage).map(|_| ())),
            "stage-incomplete",
        );
        assert_eq!(
            store.abandon_stage(stage).unwrap().state,
            StageState::Abandoned
        );

        // A begin interrupted inside the copy, before its completion receipt.
        let stage = StageId::mint();
        let tenant = fixture.stage_tenant(stage);
        raw.log()
            .record_stage_begun(stage, &tenant, head, head_revision(&store))
            .unwrap();
        match &fixture.config {
            None => interrupted_copy(
                |authority| {
                    Ok(
                        SqliteStore::sqlite_read_only(&fixture.path(), &fixture.tenant, None)?
                            .under(authority),
                    )
                },
                |authority| {
                    Ok(SqliteStore::sqlite(&fixture.path(), &tenant, None)?.under(authority))
                },
            ),
            Some(config) => interrupted_copy(
                |authority| {
                    Ok(PostgresStore::postgres(config, &fixture.tenant, false)?.under(authority))
                },
                |authority| Ok(PostgresStore::postgres(config, &tenant, false)?.under(authority)),
            ),
        }
        assert!(
            fixture.holds(&tenant) > 0,
            "the copy wrote part of the stage"
        );
        named(&refusal(store.seal_stage(stage, head)), "stage-incomplete");
        named(
            &refusal(store.join_stage(stage).map(|_| ())),
            "stage-incomplete",
        );
        assert_eq!(
            store.abandon_stage(stage).unwrap().state,
            StageState::Abandoned
        );
        assert_eq!(fixture.holds(&tenant), 0);
    });
}

#[test]
fn an_abandoned_stage_leaves_the_head_and_its_tenant_holds_nothing() {
    each(|fixture| {
        let store = fixture.seeded();
        let before = store.head().unwrap();
        let stream = revision_stream(&store);
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        commit(&joined);
        propose(&joined);
        let tenant = fixture.stage_tenant(begun.stage_id);
        assert!(fixture.holds(&tenant) > 0);
        let abandoned = store.abandon_stage(begun.stage_id).unwrap();
        assert_eq!(abandoned.state, StageState::Abandoned);
        assert_eq!(store.head().unwrap(), before, "ekr head is where it was");
        assert_eq!(revision_stream(&store), stream);
        assert_eq!(
            fixture.holds(&tenant),
            0,
            "the stage's tenant holds nothing"
        );
        assert_eq!(state(&store, begun.stage_id), StageState::Abandoned);
        for refused in [
            store.seal_stage(begun.stage_id, begun.base),
            store.publish_stage(begun.stage_id, begun.base),
        ] {
            named(&refusal(refused), "stage-already-abandoned");
        }
        assert_eq!(store.abandon_stage(begun.stage_id).unwrap(), abandoned);
    });
}

#[test]
fn a_sealed_stage_can_be_abandoned() {
    each(|fixture| {
        let store = fixture.seeded();
        let before = store.head().unwrap();
        let begun = staged(&store);
        store.seal_stage(begun.stage_id, begun.base).unwrap();
        let abandoned = store.abandon_stage(begun.stage_id).unwrap();
        assert_eq!(abandoned.state, StageState::Abandoned);
        assert_eq!(store.head().unwrap(), before);
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
        named(
            &refusal(store.publish_stage(begun.stage_id, begun.base)),
            "stage-already-abandoned",
        );
    });
}

#[test]
fn an_abandon_interrupted_after_its_record_is_finished_by_its_retry() {
    each(|fixture| {
        let store = fixture.seeded();
        for seal_first in [false, true] {
            let begun = staged(&store);
            if seal_first {
                store.seal_stage(begun.stage_id, begun.base).unwrap();
            }
            let hook = ekr_store::on_stage_point(|point| {
                if point == StagePoint::AbandonRecorded {
                    Err(StoreError::Backend("interrupted after the record".into()))
                } else {
                    Ok(())
                }
            });
            refusal(store.abandon_stage(begun.stage_id));
            drop(hook);
            let tenant = fixture.stage_tenant(begun.stage_id);
            assert_eq!(state(&store, begun.stage_id), StageState::Abandoned);
            assert!(
                fixture.holds(&tenant) > 0,
                "from Begun or Sealing: {seal_first}"
            );
            named(
                &refusal(store.join_stage(begun.stage_id).map(|_| ())),
                "stage-already-abandoned",
            );
            let retried = store.abandon_stage(begun.stage_id).unwrap();
            assert_eq!(retried.state, StageState::Abandoned);
            assert_eq!(fixture.holds(&tenant), 0);
        }
    });
}

#[test]
fn a_published_stage_tenant_holds_nothing_and_its_record_stays() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        commit(&joined);
        commit(&joined);
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
        for reader in [&store, &fixture.open()] {
            let listed = reader.stages().unwrap();
            assert_eq!(listed.len(), 1, "the record stays");
            assert_eq!(listed[0].state, StageState::Published);
            let revisions = &listed[0].stage.published_revisions;
            assert_eq!(revisions.len(), 2);
            assert_eq!(
                (revisions.first().copied(), revisions.last().copied()),
                (published.published_first, published.published_last)
            );
        }
    });
}

#[test]
fn publish_and_abandon_of_one_stage_cannot_both_succeed() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        store.seal_stage(begun.stage_id, begun.base).unwrap();
        // Abandon reads Sealing; the publication appends; abandon's conditional append is refused
        // (adversary F8).
        let published = std::rc::Rc::new(std::cell::RefCell::new(None));
        let (open, stage, base, outcome) = (
            fixture.opener(),
            begun.stage_id,
            begun.base,
            std::rc::Rc::clone(&published),
        );
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::AbandonRead && outcome.borrow().is_none() {
                *outcome.borrow_mut() =
                    Some(open().publish_stage(stage, base).map_err(store_error));
            }
            Ok(())
        });
        let abandoned = store.abandon_stage(begun.stage_id);
        drop(hook);
        let published = published.borrow_mut().take().expect("the publication ran");
        assert_eq!(published.unwrap().state, StageState::Published);
        named(&refusal(abandoned), "stage-already-published");
        assert_eq!(state(&store, begun.stage_id), StageState::Published);
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
    });
}

/// Appends a `StageBegun` that names `tenant`, past the store's own refusal of a tenant that is
/// not the stage's: what a damaged or forged record would hold. SQLite only, through the provider.
fn forge_begun(path: &Path, store: &str, stage: StageId, tenant: &str, base: RevisionNumber) {
    use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider = eventlog_sqlite::SqliteEventStore::open(&path.to_string_lossy(), "ekr")
            .await
            .unwrap();
        let stream = StreamId::new(
            TenantId::new(store).unwrap(),
            "ekr.stage",
            stage.to_string(),
        )
        .unwrap();
        let event = NewEvent::new(
            "ekr.store.StageBegun",
            1,
            serde_json::json!({
                "stage_id": stage,
                "tenant": tenant,
                "base": base,
                "base_revision": ekr_core::RevisionId::mint(),
            }),
        )
        .unwrap();
        let key = format!("forged.{stage}");
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: ContentHash::of_bytes(key.as_bytes()).to_hex(),
            subject: "forger".into(),
            actor: "forger".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        provider
            .append(&stream, Expected::NoStream, &[event], &meta)
            .await
            .unwrap();
    });
}

#[test]
fn the_store_tenant_is_never_forgotten() {
    each(|fixture| {
        let store = fixture.seeded();
        // Another store at the same location: another tenant of the same database or schema.
        let other = format!("another-{}", TypeId::mint());
        let neighbour = open_at(&fixture.path(), fixture.config.as_ref(), &other);
        neighbour.seed(seed(), || SEEDED_AT).unwrap();
        let head = store.head().unwrap().unwrap().revision;
        let raw = fixture.raw(&fixture.tenant);
        // The store's own record refuses a tenant that is not the stage's derived one.
        let stage = StageId::mint();
        for tenant in [
            fixture.tenant.clone(),
            other.clone(),
            "unmarked-tenant".to_owned(),
            fixture.stage_tenant(StageId::mint()),
            ekr_store::stage_tenant(&other, stage).unwrap(),
        ] {
            let error = raw
                .log()
                .record_stage_begun(stage, &tenant, head, head_revision(&store))
                .unwrap_err();
            assert!(
                error.to_string().contains("stage-tenant-not-derived"),
                "{tenant}: {error}"
            );
        }
        // A record naming the store's own tenant, or another store's, written past that refusal:
        // abandon records the stage and refuses to forget, and both stores are as they were. The
        // record is forged through the SQLite provider; this crate's tests carry no PostgreSQL
        // provider of their own to forge one with.
        if fixture.config.is_some() {
            return;
        }
        // Also a marked name: another live stage of this store, and a live stage of the other
        // store. Each carries the marker, and neither is the forged stage's own derived tenant.
        let ours_staged = staged(&store);
        let theirs_staged = staged(&neighbour);
        let live = [
            fixture.stage_tenant(ours_staged.stage_id),
            ekr_store::stage_tenant(&other, theirs_staged.stage_id).unwrap(),
        ];
        let held = live.clone().map(|tenant| fixture.holds(&tenant));
        for victim in [
            fixture.tenant.clone(),
            other.clone(),
            live[0].clone(),
            live[1].clone(),
        ] {
            let stage = StageId::mint();
            forge_begun(&fixture.path(), &fixture.tenant, stage, &victim, head);
            let (ours, theirs) = (revision_stream(&store), revision_stream(&neighbour));
            let error = refusal(store.abandon_stage(stage));
            assert!(
                error
                    .to_string()
                    .starts_with("a stored document could not be read: stage-tenant-not-derived"),
                "{victim}: {error}"
            );
            assert_eq!(
                revision_stream(&store),
                ours,
                "the store's tenant is not forgotten"
            );
            assert_eq!(revision_stream(&neighbour), theirs, "nor another store's");
            assert_eq!(
                live.clone().map(|tenant| fixture.holds(&tenant)),
                held,
                "nor another stage's"
            );
            store.head().unwrap().unwrap();
            neighbour.head().unwrap().unwrap();
        }
    });
}

#[test]
fn a_reader_of_the_store_never_sees_part_of_a_suffix() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let joined = store.join_stage(begun.stage_id).unwrap();
        commit(&joined);
        commit(&joined);
        propose(&joined);
        let staged = joined.head().unwrap().unwrap();
        drop(joined);
        let reader = fixture.open();
        let base = reader.head().unwrap().unwrap();
        let stream = revision_stream(&store);
        // A fault inside the group (adversary F10): an object it stores after its revision
        // appends is stored by another writer first, so the group's conditional append of that
        // object fails and with it every entry before it.
        let stage_raw = fixture.raw(&fixture.stage_tenant(begun.stage_id));
        let proposal = stage_raw
            .published()
            .iter()
            .rev()
            .find(|event| event.name == "ekr.kernel.TransactionProposed")
            .map(occurrence)
            .unwrap();
        let bytes = stage_raw.get(&proposal.record_hash).unwrap();
        drop(stage_raw);
        let (path, config, tenant) = (
            fixture.path(),
            fixture.config.clone(),
            fixture.tenant.clone(),
        );
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected {
                match &config {
                    None => Raw::Sqlite(SqliteStore::sqlite(&path, &tenant, None)?),
                    Some(config) => Raw::Postgres(PostgresStore::postgres(config, &tenant, false)?),
                }
                .put(StorageClass::Cache, &bytes);
            }
            Ok(())
        });
        let error = refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        assert_eq!(
            error,
            StoreError::StageObjectMoved {
                stage_id: begun.stage_id,
                content_hash: proposal.record_hash,
            }
        );
        assert_eq!(revision_stream(&store), stream, "none of the suffix");
        assert_eq!(reader.head().unwrap(), Some(base), "a reader sees the base");
        assert_eq!(fixture.open().head().unwrap(), Some(base));
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.occurrences, Some(7));
        assert_eq!(
            revision_stream(&store).len(),
            stream.len() + 7,
            "all of it at once"
        );
        assert_eq!(roots(&reader.head().unwrap().unwrap()), roots(&staged));
    });
}

#[test]
fn a_handle_joined_to_a_published_or_abandoned_stage_refuses() {
    each(|fixture| {
        let store = fixture.seeded();
        for publish in [true, false] {
            let begun = store.begin_stage().unwrap();
            let joined = store.join_stage(begun.stage_id).unwrap();
            commit(&joined);
            // Opened while Begun, and read through while Begun (adversary F7).
            let reader = store.join_stage(begun.stage_id).unwrap();
            reader.head().unwrap().unwrap();
            let name = if publish {
                store
                    .seal_and_publish_stage(begun.stage_id, begun.base)
                    .unwrap();
                "stage-already-published"
            } else {
                store.abandon_stage(begun.stage_id).unwrap();
                "stage-already-abandoned"
            };
            named(&reader.head().unwrap_err(), name);
            named(&reader.snapshot().unwrap_err(), name);
            named(&refusal(try_propose(&joined, &define())), name);
            assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
        }
    });
}

#[test]
fn a_proposal_made_in_the_store_during_the_run_does_not_refuse_the_publication_and_precedes_the_suffix(
) {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = store.begin_stage().unwrap();
        let run = commit(&store.join_stage(begun.stage_id).unwrap());
        let foreign = propose(&store);
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.occurrences, Some(3));
        let proposed: Vec<TransactionId> = revision_stream(&store)
            .iter()
            .filter_map(|event| match occurrence(event).payload {
                ekr_graph::RevisionPayload::TransactionProposed { transaction_id, .. } => {
                    Some(transaction_id)
                }
                _ => None,
            })
            .collect();
        let at = |id: TransactionId| proposed.iter().position(|held| *held == id).unwrap();
        assert!(at(foreign) < at(run), "the run's occurrences follow theirs");
        let states = fixture.open().transaction_states([run, foreign]).unwrap();
        assert_eq!(states.get(&run), Some(&TransactionState::Committed));
        assert_eq!(states.get(&foreign), Some(&TransactionState::Proposed));
    });
}

#[test]
fn a_proposal_landing_between_capture_and_append_is_refused_stage_stream_moved_and_the_retry_publishes(
) {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let captured = revision_stream(&store).len() as u64;
        let open = fixture.opener();
        let mut proposed = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected && !proposed {
                proposed = true;
                propose(&open());
            }
            Ok(())
        });
        let error = refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        named(&error, "stage-stream-moved");
        assert_eq!(
            error,
            StoreError::StageStreamMoved {
                stage_id: begun.stage_id,
                captured,
                current: captured + 1,
            }
        );
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
        assert_eq!(revision_stream(&store).len() as u64, captured + 1);
        let elected = elections(&store);
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.state, StageState::Published);
        assert_eq!(
            elections(&store),
            elected + 1,
            "the slot's successor attempt"
        );
        assert_eq!(revision_stream(&store).len() as u64, captured + 4);
    });
}

#[test]
fn an_object_stored_in_the_store_after_the_capture_is_refused_stage_object_moved_and_the_retry_publishes(
) {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let stage_raw = fixture.raw(&fixture.stage_tenant(begun.stage_id));
        let proposal = stage_raw
            .published()
            .iter()
            .rev()
            .find(|event| event.name == "ekr.kernel.TransactionProposed")
            .map(occurrence)
            .unwrap();
        let bytes = stage_raw.get(&proposal.record_hash).unwrap();
        drop(stage_raw);
        let stream = revision_stream(&store);
        let raw = fixture.raw(&fixture.tenant);
        let mut stored = Some(bytes);
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected {
                if let Some(bytes) = stored.take() {
                    raw.put(StorageClass::Incubating, &bytes);
                }
            }
            Ok(())
        });
        let error = refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        named(&error, "stage-object-moved");
        assert_eq!(
            error,
            StoreError::StageObjectMoved {
                stage_id: begun.stage_id,
                content_hash: proposal.record_hash,
            }
        );
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
        assert_eq!(revision_stream(&store), stream);
        let published = store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        assert_eq!(published.state, StageState::Published);
        assert_eq!(revision_stream(&store).len(), stream.len() + 3);
        // The object the other writer stored weakly is raised to what the suffix needs.
        let held = fixture
            .raw(&fixture.tenant)
            .log()
            .held_classes(&BTreeSet::from([proposal.record_hash]))
            .unwrap();
        assert_eq!(
            held.get(&proposal.record_hash),
            Some(&StorageClass::Canonical)
        );
    });
}

#[test]
fn an_abandonment_landing_between_capture_and_append_is_answered_stage_already_abandoned() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let stream = revision_stream(&store);
        let (open, stage) = (fixture.opener(), begun.stage_id);
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected {
                open().abandon_stage(stage).map_err(store_error)?;
            }
            Ok(())
        });
        let error = refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        named(&error, "stage-already-abandoned");
        assert_eq!(
            error,
            StoreError::StageStateConflict {
                stage_id: begun.stage_id,
                state: StageState::Abandoned,
            }
        );
        assert_eq!(revision_stream(&store), stream, "nothing appended");
        assert_eq!(state(&store, begun.stage_id), StageState::Abandoned);
        assert_eq!(fixture.holds(&fixture.stage_tenant(begun.stage_id)), 0);
    });
}

#[test]
fn a_store_begins_a_stage_after_a_publication_refused_at_its_append() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let open = fixture.opener();
        let mut proposed = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected && !proposed {
                proposed = true;
                propose(&open());
            }
            Ok(())
        });
        named(
            &refusal(store.seal_and_publish_stage(begun.stage_id, begun.base)),
            "stage-stream-moved",
        );
        drop(hook);
        // The elected group expects a version the stream is past, and the stage is still Sealing:
        // the slot is resolved, and the store stages again (adversary F11).
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
        let next = store.begin_stage().unwrap();
        assert_eq!(next.state, StageState::Begun);
        store.abandon_stage(next.stage_id).unwrap();
        // An object conflict leaves the stream where the group expects it: that slot may still
        // land, so begin refuses until its stage is published or abandoned.
        let second = staged(&store);
        let stage_raw = fixture.raw(&fixture.stage_tenant(second.stage_id));
        let proposal = stage_raw
            .published()
            .iter()
            .rev()
            .find(|event| event.name == "ekr.kernel.TransactionProposed")
            .map(occurrence)
            .unwrap();
        let mut bytes = stage_raw.get(&proposal.record_hash);
        drop(stage_raw);
        let raw = fixture.raw(&fixture.tenant);
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected {
                if let Some(bytes) = bytes.take() {
                    raw.put(StorageClass::Cache, &bytes);
                }
            }
            Ok(())
        });
        named(
            &refusal(store.seal_and_publish_stage(second.stage_id, second.base)),
            "stage-object-moved",
        );
        drop(hook);
        named(&refusal(store.begin_stage()), "unresolved-preparation");
        store.abandon_stage(second.stage_id).unwrap();
        assert_eq!(store.begin_stage().unwrap().state, StageState::Begun);
    });
}

#[test]
fn a_store_begins_a_stage_after_an_elected_sealing_stage_is_abandoned() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let hook = ekr_store::on_stage_point(|point| {
            if point == StagePoint::PublishElected {
                Err(StoreError::Backend("interrupted after the election".into()))
            } else {
                Ok(())
            }
        });
        refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
        named(&refusal(store.begin_stage()), "unresolved-preparation");
        store.abandon_stage(begun.stage_id).unwrap();
        assert_eq!(store.begin_stage().unwrap().state, StageState::Begun);
    });
}

#[test]
fn a_store_with_a_published_stage_begins_another_and_migrates() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        store
            .seal_and_publish_stage(begun.stage_id, begun.base)
            .unwrap();
        // The inventory reads the `/4` decision of the published stage as resolved.
        let next = store.begin_stage().unwrap();
        assert_eq!(next.base, RevisionNumber::new(begun.base.get() + 1));
        store.abandon_stage(next.stage_id).unwrap();
        if fixture.config.is_some() {
            // PostgreSQL is no `ekr migrate` source (`migrate-source-not-supported`).
            return;
        }
        let destination = Runtime::sqlite(
            &fixture.directory.path().join("migrated.db"),
            "migrated",
            context(),
            evolving(),
        )
        .unwrap();
        store.migrate_into(&destination).unwrap();
        assert_eq!(
            roots(&destination.head().unwrap().unwrap()),
            roots(&store.head().unwrap().unwrap())
        );
    });
}

/// The PostgreSQL capture begin reads a store under has no bound below the tenant's own size and
/// holds the schema's publication lock while it reads (unit C's adversary): begin takes exactly
/// one capture of the store, and what it holds is counted here — every event of the store's
/// tenant and every object's bytes — rather than timed.
#[test]
fn a_stage_begin_reads_the_store_under_one_capture_and_counts_what_it_holds() {
    let Some(config) = postgres() else { return };
    let fixture = Fixture::postgres(config);
    let store = fixture.seeded();
    commit(&store);
    let inventory = match fixture.raw(&fixture.tenant) {
        Raw::Postgres(raw) => raw.inventory().unwrap(),
        Raw::Sqlite(_) => unreachable!(),
    };
    let bytes: usize = inventory
        .objects
        .values()
        .map(|object| object.object.bytes.len())
        .sum();
    assert_eq!(inventory.events, store.published_events().unwrap().len());
    let _ = ekr_kernel::stream_reads();
    store.begin_stage().unwrap();
    let reads = ekr_kernel::stream_reads();
    assert_eq!(
        reads.captures, 2,
        "one capture of the store's tenant, holding {} events and {bytes} object bytes, and one \
         zero-limit capture finding the stage's tenant empty",
        inventory.events
    );
    eprintln!(
        "begin's capture held {} events, {} objects, {bytes} object bytes",
        inventory.events,
        inventory.objects.len()
    );
}
