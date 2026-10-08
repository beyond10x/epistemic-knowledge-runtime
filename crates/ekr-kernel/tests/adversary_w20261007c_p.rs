//! Adversary cases for unit P of wave 2026-10-07c (`task:stage-suffix-publication`, design § 107).
//!
//! Each case runs on SQLite, and on PostgreSQL where the PostgreSQL tests run (set
//! EKR_TEST_POSTGRES_CONFIG and EKR_TEST_POSTGRES_OWNER; EKR_REQUIRE_POSTGRES=1 makes their
//! absence a failure; run with `--test-threads=1`). Interleavings are made with
//! `ekr_store::on_stage_point`, and across threads with channels: no case reads a clock.
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::{ContentHash, RevisionNumber, StageId, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, CommitCommandResult, CommitError,
    GraphOperation, GraphTransaction, Runtime, TransactionState, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::NodeType;
use ekr_store::{
    PostgresStore, PublishedEvent, SqliteStore, StageLog, StagePoint, StageResult, StageState,
    StorageClass, StoreError,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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

fn evolving() -> AuthorityStateV1 {
    let mut anchor = anchor();
    anchor.validation_profile = ValidationProfileV1::schema_evolving(context().validator);
    anchor
}

struct Fixture {
    directory: tempfile::TempDir,
    config: Option<PostgresConfiguration>,
    tenant: String,
}

impl Fixture {
    fn sqlite() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            config: None,
            tenant: "adversary-store".into(),
        }
    }
    fn postgres(config: PostgresConfiguration) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            config: Some(config),
            tenant: format!("adversary-{}", TypeId::mint()),
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("store.db")
    }
    fn open(&self) -> Runtime {
        open_at(&self.path(), self.config.as_ref(), &self.tenant)
    }
    /// What the store's location and tenant are, for a writer on another thread or in a hook.
    fn location(&self) -> (PathBuf, Option<PostgresConfiguration>, String) {
        (self.path(), self.config.clone(), self.tenant.clone())
    }
    /// How many events `tenant` of this store holds.
    fn holds(&self, tenant: &str) -> usize {
        match &self.config {
            None => SqliteStore::sqlite(&self.path(), tenant, None)
                .unwrap()
                .published_events()
                .unwrap()
                .len(),
            Some(config) => {
                PostgresStore::postgres(config, tenant, false)
                    .unwrap()
                    .inventory()
                    .unwrap()
                    .events
            }
        }
    }
    /// The class the store's own tenant holds `hash` at.
    fn held_class(&self, hash: ContentHash) -> Option<StorageClass> {
        let hashes = BTreeSet::from([hash]);
        match &self.config {
            None => SqliteStore::sqlite(&self.path(), &self.tenant, None)
                .unwrap()
                .held_classes(&hashes)
                .unwrap()
                .get(&hash)
                .copied(),
            Some(config) => PostgresStore::postgres(config, &self.tenant, false)
                .unwrap()
                .held_classes(&hashes)
                .unwrap()
                .get(&hash)
                .copied(),
        }
    }
    fn stage_tenant(&self, stage: StageId) -> String {
        ekr_store::stage_tenant(&self.tenant, stage).unwrap()
    }
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

fn reopen(location: &(PathBuf, Option<PostgresConfiguration>, String)) -> Runtime {
    open_at(&location.0, location.1.as_ref(), &location.2)
}

/// Runs `case` on SQLite, and on PostgreSQL where the PostgreSQL tests run. With
/// `EKR_ADVERSARY_POSTGRES_FIRST=1` PostgreSQL runs first, so a case red on SQLite also shows
/// what PostgreSQL answers.
fn each(case: impl Fn(&Fixture)) {
    if std::env::var("EKR_ADVERSARY_POSTGRES_FIRST").as_deref() == Ok("1") {
        if let Some(config) = postgres() {
            case(&Fixture::postgres(config));
        }
        case(&Fixture::sqlite());
        return;
    }
    case(&Fixture::sqlite());
    if let Some(config) = postgres() {
        case(&Fixture::postgres(config));
    }
}

/// A counter, not a clock: later than every instant before it.
fn at() -> i64 {
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_000);
    NEXT.fetch_add(10, std::sync::atomic::Ordering::Relaxed)
}

fn define() -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Adversary{}", TypeId::mint()),
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

fn try_propose(runtime: &Runtime, transaction: &GraphTransaction) -> Result<(), CommitError> {
    let at = at();
    runtime
        .propose(&document(transaction), context().operator, || {
            Timestamp::from_millis(at)
        })
        .map(|_| ())
}

/// A transaction adding evidence whose payload a commit publishes as a Provenance object, and an
/// assertion on the seed's first node citing it.
fn evidence(payload: &[u8]) -> GraphTransaction {
    let id = ekr_core::EvidenceId::mint();
    let assertion = ekr_graph::Assertion {
        id: ekr_core::AssertionId::mint(),
        root_id: current_fixture::id(0x02),
        subject: ekr_graph::Subject::Node(current_fixture::id(0x10)),
        predicate: ekr_graph::Predicate::Property(current_fixture::id(0x06)),
        object: ekr_graph::Object::Value(ekr_ontology::Value::String("adversary".into())),
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

/// Validates `id` against `runtime`'s head and commits it.
fn validate_and_commit(runtime: &Runtime, id: TransactionId) {
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
    let committed = runtime
        .commit(id, context().operator, || Timestamp::from_millis(at + 1))
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{committed:?}"
    );
}

/// Validates `id` against `runtime`'s head, and nothing else.
fn validate(runtime: &Runtime, id: TransactionId) {
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
}

fn commit(runtime: &Runtime) -> TransactionId {
    let transaction = define();
    try_propose(runtime, &transaction).unwrap();
    validate_and_commit(runtime, transaction.id);
    transaction.id
}

fn refusal<T: std::fmt::Debug>(result: Result<T, CommitError>) -> StoreError {
    match result {
        Ok(admitted) => panic!("admitted: {admitted:?}"),
        Err(CommitError::Store(error)) => error,
        Err(other) => panic!("not a store refusal: {other}"),
    }
}

fn named(error: &StoreError, name: &str) {
    assert!(
        error.to_string().starts_with(&format!("{name}:")),
        "expected {name}: {error}"
    );
}

fn store_error(error: CommitError) -> StoreError {
    match error {
        CommitError::Store(error) => error,
        other => StoreError::Backend(other.to_string()),
    }
}

fn revision_stream(runtime: &Runtime) -> Vec<PublishedEvent> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.stream_type == "ekr.revision")
        .collect()
}

fn roots(root: &ekr_graph::Root) -> [String; 5] {
    [
        root.revision.to_string(),
        root.ontology_root.to_string(),
        root.knowledge_root.to_string(),
        root.evidence_root.to_string(),
        root.agent_root.to_string(),
    ]
}

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

// ---- 1. A seal and an abandonment of one stage, both from Begun ------------------------------
//
// Design § 107.6: "A publication and an abandonment of one stage both condition on the record's
// version, so at most one of them succeeds"; § 107.5: "An abandonment of a Sealing stage is always
// admitted"; § 107.4: seal refuses `StageStateConflict` for a stage that is Abandoned. Each command
// reads the record and appends at the version it read, and `abandon_stage` reads again on a
// conflict. These cases put the other command's append between the read and the append.

/// Abandon reads Begun; a seal lands; abandon's conditional append is refused, abandon reads the
/// record again (Sealing) and abandons it, and the stage's tenant holds nothing.
#[test]
fn an_abandonment_that_read_begun_abandons_a_stage_sealed_meanwhile() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let (location, stage, base) = (fixture.location(), begun.stage_id, begun.base);
        let mut sealed = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::AbandonRead && !sealed {
                sealed = true;
                reopen(&location)
                    .seal_stage(stage, base)
                    .map_err(store_error)?;
            }
            Ok(())
        });
        let abandoned = store.abandon_stage(stage);
        drop(hook);
        let abandoned = abandoned.unwrap_or_else(|error| {
            panic!(
                "an abandonment of a stage sealed meanwhile is admitted (design § 107.5), and \
                 the stage is now {:?}: {error}",
                state(&store, stage)
            )
        });
        assert_eq!(abandoned.state, StageState::Abandoned);
        assert_eq!(fixture.holds(&fixture.stage_tenant(stage)), 0);
    });
}

/// Seal reads Begun and checks the stage; an abandonment lands; seal's conditional append is
/// refused and seal answers `stage-already-abandoned`.
#[test]
fn a_seal_that_read_begun_is_refused_stage_already_abandoned_when_an_abandonment_lands() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let (location, stage) = (fixture.location(), begun.stage_id);
        let mut abandoned = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::SealChecked && !abandoned {
                abandoned = true;
                reopen(&location)
                    .abandon_stage(stage)
                    .map_err(store_error)?;
            }
            Ok(())
        });
        let error = refusal(store.seal_stage(stage, begun.base));
        drop(hook);
        assert_eq!(state(&store, stage), StageState::Abandoned);
        assert_eq!(
            error,
            StoreError::StageStateConflict {
                stage_id: stage,
                state: StageState::Abandoned,
            },
            "seal refuses an Abandoned stage by name (design § 107.4)"
        );
    });
}

/// Abandon reads Begun; `ekr stage publish` seals and publishes the stage; abandon's conditional
/// append is refused, and abandon answers `stage-already-published`.
#[test]
fn an_abandonment_that_read_begun_is_refused_stage_already_published_when_a_publication_lands() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let (location, stage, base) = (fixture.location(), begun.stage_id, begun.base);
        let mut published = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::AbandonRead && !published {
                published = true;
                reopen(&location)
                    .seal_and_publish_stage(stage, base)
                    .map_err(store_error)?;
            }
            Ok(())
        });
        let error = refusal(store.abandon_stage(stage));
        drop(hook);
        assert_eq!(state(&store, stage), StageState::Published);
        assert_eq!(
            error,
            StoreError::StageStateConflict {
                stage_id: stage,
                state: StageState::Published,
            },
            "at most one succeeds, and the other is refused by name (design § 107.6)"
        );
    });
}

// ---- 2. The refusal of an unresolved `/4` slot names its stage -------------------------------
//
// Design § 107.5: "Otherwise the slot belongs to a Sealing stage whose attempt may still land.
// The refusal names that stage, and publishing or abandoning it resolves the slot."

#[test]
fn a_begin_refused_for_an_unresolved_stage_publication_names_the_stage() {
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
        let error = refusal(store.begin_stage());
        named(&error, "unresolved-preparation");
        assert!(
            error.to_string().contains(&begun.stage_id.to_string()),
            "the refusal names the Sealing stage whose slot is unresolved, {}, so the caller \
             knows which stage to publish or abandon (design § 107.5): {error}",
            begun.stage_id
        );
    });
}

// ---- 3. A publish interrupted before its forgetting, then abandoned ---------------------------
//
// The story's acceptance: "After abandon or publish, the stage's tenant holds nothing."

#[test]
fn a_stage_published_before_its_forgetting_then_abandoned_leaves_its_tenant_empty() {
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
        // The run's caller saw its publish fail and abandons the run.
        named(
            &refusal(store.abandon_stage(begun.stage_id)),
            "stage-already-published",
        );
        assert_eq!(
            fixture.holds(&tenant),
            0,
            "after abandon or publish, the stage's tenant holds nothing (story acceptance)"
        );
    });
}

// ---- 4. What lands after a seal ---------------------------------------------------------------
//
// Design § 107.3: "the seal refuses a stage holding an elected decision never published. So a
// seal never falls between those two appends: what lands after a seal is an election, or a
// checkpoint pointer, and never an occurrence." The seal's check is its capture; a joined propose
// elects after that capture and before the seal's record, and publishes after the record.

// Its case was removed: § 107.3 now says an occurrence may land after the seal, its writer is
// refused `stage-write-landed` with the occurrence id, and the publication carries it; `stage.rs`
// holds the corrected behaviour.

// ---- 5. Another writer's validation in the store during the run -------------------------------
//
// Design § 107.4: "Occurrences another writer appended to the store's revision stream after the
// base that commit nothing (a proposal, a validation, a rejection, a stale record) do not refuse
// the publication." And: "A suffix the kernel refuses (`ekr.store.StageSuffixRefused`) ... leaves
// the store unchanged and the stage Sealing"; after `stream-moved` the successor is checked again
// "against the store as it now reads".

// Its case was removed: another writer's validation of a transaction the run also decides does
// refuse the publication, and § 107.4 now says so;
// `a_validation_another_writer_made_of_a_transaction_the_run_decides_refuses_the_publication` in
// `stage.rs` holds it. The case below keeps the retry after `stage-stream-moved`.

/// The same validation by another writer, landing between the publication's capture and its
/// append: the first publish is refused `stage-stream-moved`, and the retry's refusal is the one
/// a capture of the same store gives, `stage-suffix-refused`, or the retry publishes.
#[test]
fn a_validation_landing_between_capture_and_append_is_answered_as_a_capture_answers_it() {
    each(|fixture| {
        let store = fixture.seeded();
        let pending = define();
        try_propose(&store, &pending).unwrap();
        let begun = store.begin_stage().unwrap();
        validate_and_commit(&store.join_stage(begun.stage_id).unwrap(), pending.id);
        let (location, id) = (fixture.location(), pending.id);
        let mut validated = false;
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected && !validated {
                validated = true;
                validate(&reopen(&location), id);
            }
            Ok(())
        });
        let first = refusal(store.seal_and_publish_stage(begun.stage_id, begun.base));
        drop(hook);
        named(&first, "stage-stream-moved");
        match store.seal_and_publish_stage(begun.stage_id, begun.base) {
            Ok(published) => assert_eq!(published.state, StageState::Published),
            Err(error) => {
                let error = store_error(error);
                assert_eq!(state(&store, begun.stage_id), StageState::Sealing);
                named(&error, "stage-suffix-refused");
            }
        }
    });
}

// ---- 6. Probes: sequential publications, an empty run, a raise of a copied object --------------

#[test]
fn publications_in_sequence_from_a_seed_replay_and_migrate() {
    each(|fixture| {
        let store = fixture.open();
        store.seed(seed(), || SEEDED_AT).unwrap();
        // An evidence proposal pending in the store before the first base: its payload is the
        // store's at the class a proposal stores it with.
        let payload = format!("a payload proposed before the base {}", TypeId::mint()).into_bytes();
        let hash = ContentHash::of_bytes(&payload);
        let pending = evidence(&payload);
        try_propose(&store, &pending).unwrap();
        let class_before = fixture.held_class(hash);
        // The first run validates and commits that proposal, and commits one of its own.
        let first = store.begin_stage().unwrap();
        assert_eq!(first.base, RevisionNumber::SEED);
        let joined = store.join_stage(first.stage_id).unwrap();
        validate_and_commit(&joined, pending.id);
        let own = commit(&joined);
        let staged_head = joined.head().unwrap().unwrap();
        drop(joined);
        let published = store
            .seal_and_publish_stage(first.stage_id, first.base)
            .unwrap();
        assert_eq!(published.occurrences, Some(5));
        assert_eq!(
            roots(&store.head().unwrap().unwrap()),
            roots(&staged_head),
            "the runtime that published reads the new head"
        );
        let states = store.transaction_states([pending.id, own]).unwrap();
        assert_eq!(states.get(&pending.id), Some(&TransactionState::Committed));
        assert_eq!(states.get(&own), Some(&TransactionState::Committed));
        assert_eq!(
            fixture.held_class(hash),
            Some(StorageClass::Provenance),
            "the payload, {class_before:?} in the store before, is raised with the commit"
        );
        // An empty run.
        let second = store.begin_stage().unwrap();
        assert_eq!(second.base, staged_head.revision);
        let stream = revision_stream(&store);
        let empty = store
            .seal_and_publish_stage(second.stage_id, second.base)
            .unwrap();
        assert_eq!(
            (
                empty.occurrences,
                empty.published_first,
                empty.published_last
            ),
            (Some(0), None, None)
        );
        assert_eq!(revision_stream(&store), stream);
        assert_eq!(fixture.holds(&fixture.stage_tenant(second.stage_id)), 0);
        // A third run after them.
        let third = staged(&store);
        assert_eq!(third.base, staged_head.revision);
        store
            .seal_and_publish_stage(third.stage_id, third.base)
            .unwrap();
        let head = store.head().unwrap().unwrap();
        assert_eq!(head.revision.get(), staged_head.revision.get() + 1);
        let mut replayed = fixture.open();
        replayed.set_full_replay(true);
        assert_eq!(roots(&replayed.head().unwrap().unwrap()), roots(&head));
        replayed.snapshot().unwrap();
        assert_eq!(
            replayed.content(&hash).unwrap(),
            Some(payload.clone()),
            "the evidence payload is the store's"
        );
        let listed: Vec<(StageId, StageState, usize)> = store
            .stages()
            .unwrap()
            .into_iter()
            .map(|listed| {
                (
                    listed.stage.stage_id,
                    listed.state,
                    listed.stage.published_revisions.len(),
                )
            })
            .collect();
        for (stage, revisions) in [
            (first.stage_id, 2),
            (second.stage_id, 0),
            (third.stage_id, 1),
        ] {
            assert!(
                listed.contains(&(stage, StageState::Published, revisions)),
                "{stage}: {listed:?}"
            );
        }
        if fixture.config.is_none() {
            let destination = Runtime::sqlite(
                &fixture.directory.path().join("migrated.db"),
                "migrated",
                context(),
                evolving(),
            )
            .unwrap();
            store.migrate_into(&destination).unwrap();
            assert_eq!(roots(&destination.head().unwrap().unwrap()), roots(&head));
        }
    });
}

// ---- 7. Probes: another store's stage id, and a stage of a stage -----------------------------

#[test]
fn another_store_cannot_reach_a_stage_and_a_stage_has_no_stage() {
    each(|fixture| {
        let store = fixture.seeded();
        let other = format!("neighbour-{}", TypeId::mint());
        let neighbour = open_at(&fixture.path(), fixture.config.as_ref(), &other);
        neighbour.seed(seed(), || SEEDED_AT).unwrap();
        let begun = staged(&store);
        let (stage, base) = (begun.stage_id, begun.base);
        let tenant = fixture.stage_tenant(stage);
        let held = fixture.holds(&tenant);
        let neighbour_stream = revision_stream(&neighbour);
        for refused in [
            neighbour.seal_stage(stage, base),
            neighbour.publish_stage(stage, base),
            neighbour.seal_and_publish_stage(stage, base),
            neighbour.abandon_stage(stage),
        ] {
            named(&refusal(refused), "stage-not-found");
        }
        named(
            &refusal(neighbour.join_stage(stage).map(|_| ())),
            "stage-not-found",
        );
        assert_eq!(fixture.holds(&tenant), held);
        assert_eq!(state(&store, stage), StageState::Begun);
        let joined = store.join_stage(stage).unwrap();
        let on_stage = |error: StoreError| {
            assert!(
                error.to_string().contains("stage-command-on-stage"),
                "{error}"
            );
        };
        on_stage(refusal(joined.begin_stage()));
        on_stage(refusal(joined.join_stage(stage).map(|_| ())));
        on_stage(refusal(joined.seal_stage(stage, base)));
        on_stage(refusal(joined.publish_stage(stage, base)));
        on_stage(refusal(joined.abandon_stage(stage)));
        on_stage(refusal(joined.stages()));
        drop(joined);
        store.seal_and_publish_stage(stage, base).unwrap();
        assert_eq!(revision_stream(&neighbour), neighbour_stream);
        assert_eq!(fixture.holds(&tenant), 0);
    });
}

// ---- 8. Probe: two publishes of one stage ------------------------------------------------------

#[test]
fn two_publishes_of_one_stage_append_it_once_and_answer_alike() {
    each(|fixture| {
        let store = fixture.seeded();
        let begun = staged(&store);
        let stream = revision_stream(&store).len();
        let other = std::rc::Rc::new(std::cell::RefCell::new(None));
        let (location, stage, base, outcome) = (
            fixture.location(),
            begun.stage_id,
            begun.base,
            std::rc::Rc::clone(&other),
        );
        let hook = ekr_store::on_stage_point(move |point| {
            if point == StagePoint::PublishElected && outcome.borrow().is_none() {
                *outcome.borrow_mut() = Some(
                    reopen(&location)
                        .seal_and_publish_stage(stage, base)
                        .map_err(store_error),
                );
            }
            Ok(())
        });
        let first = store.seal_and_publish_stage(stage, base);
        drop(hook);
        let second = other.borrow_mut().take().expect("the other publish ran");
        let (first, second) = (first.unwrap(), second.unwrap());
        assert_eq!(first, second);
        assert_eq!(first.state, StageState::Published);
        let fed = revision_stream(&store).len();
        let held = match &fixture.config {
            None => SqliteStore::sqlite(&fixture.path(), &fixture.tenant, None)
                .unwrap()
                .inventory()
                .unwrap()
                .occurrences
                .len(),
            Some(config) => PostgresStore::postgres(config, &fixture.tenant, false)
                .unwrap()
                .inventory()
                .unwrap()
                .occurrences
                .len(),
        };
        let head = fixture.open().head().unwrap().unwrap().revision;
        eprintln!(
            "two publishes on {:?}: feed {fed}, capture {held}",
            store.provider()
        );
        assert_eq!(
            (held, head),
            (stream + 3, RevisionNumber::new(base.get() + 1)),
            "once, on {:?}: the provider's feed showed {fed} occurrences, a capture {held}",
            store.provider()
        );
        assert_eq!(fixture.holds(&fixture.stage_tenant(stage)), 0);
    });
}
