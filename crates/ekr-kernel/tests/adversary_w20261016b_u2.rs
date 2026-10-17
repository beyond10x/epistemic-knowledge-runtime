//! Adversary cases for unit U2 of wave 2026-10-16b (`story:stage-list-reads-committed`, #94).
//!
//! `ekr stage list` reads the store's stage index (`ekr.store.StageIndexed`) unioned with the
//! published feed's stage streams, and refuses an index that names a stage twice, names a stage
//! with no record, or holds an entry of another kind. The SQLite cases forge provider state
//! through `eventlog-sqlite` directly: a stage recorded before the index existed, and an index
//! that disagrees with the records. The probe of concurrent begins and every state runs on SQLite
//! and, where the PostgreSQL tests run, on PostgreSQL (set EKR_TEST_POSTGRES_CONFIG and
//! EKR_TEST_POSTGRES_OWNER; EKR_REQUIRE_POSTGRES=1 makes their absence a failure; run with
//! `--test-threads=1`). No case reads a clock.
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::{ContentHash, StageId, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, CommitCommandResult, CommitError,
    GraphOperation, GraphTransaction, Runtime, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::NodeType;
use ekr_store::{StageState, StoreError};
use std::collections::{BTreeMap, BTreeSet};
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
        Runtime::postgres_schema(&PostgresConfiguration::read(Path::new(&owner)).unwrap()).unwrap();
    });
    Some(PostgresConfiguration::read(Path::new(&path)).unwrap())
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
            tenant: "adversary-u2-store".into(),
        }
    }
    fn postgres(config: PostgresConfiguration) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            config: Some(config),
            tenant: format!("adversary-u2-{}", TypeId::mint()),
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("store.db")
    }
    fn open_tenant(&self, tenant: &str) -> Runtime {
        match &self.config {
            None => Runtime::sqlite(&self.path(), tenant, context(), evolving()),
            Some(config) => Runtime::postgres(config, tenant, context(), evolving(), false),
        }
        .unwrap()
    }
    fn open(&self) -> Runtime {
        self.open_tenant(&self.tenant)
    }
    fn seeded(&self) -> Runtime {
        let store = self.open();
        store.seed(seed(), || SEEDED_AT).unwrap();
        store
    }
}

fn each(case: impl Fn(&Fixture)) {
    case(&Fixture::sqlite());
    if let Some(config) = postgres() {
        case(&Fixture::postgres(config));
    }
}

fn at() -> i64 {
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1_000);
    NEXT.fetch_add(10, std::sync::atomic::Ordering::Relaxed)
}

/// One transaction defining a new node type, proposed, validated and committed.
fn commit(runtime: &Runtime) -> TransactionId {
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
            format!("Listed{}", TypeId::mint()),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    let document = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .unwrap()
    .into_bytes();
    let at = at();
    runtime
        .propose(&document, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = runtime
        .validate(
            transaction.id,
            runtime.head().unwrap().unwrap().revision,
            || Timestamp::from_millis(at + 1),
        )
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let committed = runtime
        .commit(transaction.id, context().operator, || {
            Timestamp::from_millis(at + 2)
        })
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{committed:?}"
    );
    transaction.id
}

fn listed(store: &Runtime) -> BTreeMap<StageId, StageState> {
    store
        .stages()
        .unwrap()
        .into_iter()
        .map(|listing| (listing.stage.stage_id, listing.state))
        .collect()
}

fn refusal<T: std::fmt::Debug>(result: Result<T, CommitError>) -> String {
    match result {
        Ok(admitted) => panic!("admitted: {admitted:?}"),
        Err(CommitError::Store(error @ StoreError::Document(_))) => error.to_string(),
        Err(other) => panic!("refused, but not as a document that disagrees: {other}"),
    }
}

/// Appends one event named `name` with `data` to the stream `stream_type`/`stream_id` of tenant
/// `store`, through the SQLite provider and past every check of the runtime: a forged or
/// legacy state of the provider log, as a store written by an older binary or damaged holds it.
fn forge(
    path: &Path,
    store: &str,
    stream_type: &str,
    stream_id: &str,
    name: &str,
    data: serde_json::Value,
) {
    use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider = eventlog_sqlite::SqliteEventStore::open(&path.to_string_lossy(), "ekr")
            .await
            .unwrap();
        let stream = StreamId::new(TenantId::new(store).unwrap(), stream_type, stream_id).unwrap();
        let event = NewEvent::new(name, 1, data).unwrap();
        let key = format!("forged.{}", TypeId::mint());
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
            .append(&stream, Expected::Any, &[event], &meta)
            .await
            .unwrap();
    });
}

fn forge_index_entry(fixture: &Fixture, stage: StageId) {
    forge(
        &fixture.path(),
        &fixture.tenant,
        "ekr.stage-index",
        "stages",
        "ekr.store.StageIndexed",
        serde_json::json!({ "stage_id": stage }),
    );
}

/// (c) A store whose stage was begun before this release holds its `StageBegun` and no index
/// entry. The listing still names it, in its record's state, beside a stage begun now. This is
/// the one case in the suite that reaches the feed half of the union: with the feed scan removed
/// from `StageLog::stages`, every other listing case stays green.
#[test]
fn a_stage_recorded_before_the_index_existed_is_listed() {
    let fixture = Fixture::sqlite();
    let store = fixture.seeded();
    commit(&store);
    let base = store.head().unwrap().unwrap().revision;
    let begun = store.begin_stage().unwrap();
    let legacy = StageId::mint();
    forge(
        &fixture.path(),
        &fixture.tenant,
        "ekr.stage",
        &legacy.to_string(),
        "ekr.store.StageBegun",
        serde_json::json!({
            "stage_id": legacy,
            "tenant": ekr_store::stage_tenant(&fixture.tenant, legacy).unwrap(),
            "base": base,
            "base_revision": ekr_core::RevisionId::mint(),
        }),
    );
    let states = listed(&store);
    assert_eq!(
        states,
        BTreeMap::from([
            (begun.stage_id, StageState::Begun),
            (legacy, StageState::Begun),
        ]),
        "a stage recorded before the stage index existed is not listed"
    );
}

/// The index names a stage twice: the listing refuses by name, and does not fold the duplicate
/// away. With the duplicate check removed (a `BTreeSet` absorbs it), the listing answers one
/// stage and this case fails.
#[test]
fn an_index_naming_a_stage_twice_is_refused_by_name() {
    let fixture = Fixture::sqlite();
    let store = fixture.seeded();
    let begun = store.begin_stage().unwrap();
    forge_index_entry(&fixture, begun.stage_id);
    let refused = refusal(store.stages());
    assert!(
        refused.contains(&format!(
            "stage-index-disagrees: stage {} is indexed twice",
            begun.stage_id
        )),
        "{refused}"
    );
}

/// The index names a stage that has no record: the listing refuses by name rather than leaving
/// the stage out. With `None if indexed.contains(..)` removed, the listing answers the one real
/// stage and this case fails.
#[test]
fn an_index_entry_with_no_record_is_refused_by_name() {
    let fixture = Fixture::sqlite();
    let store = fixture.seeded();
    store.begin_stage().unwrap();
    let missing = StageId::mint();
    forge_index_entry(&fixture, missing);
    let refused = refusal(store.stages());
    assert!(
        refused.contains(&format!(
            "stage-record-disagrees: {missing}: indexed, with no record"
        )),
        "{refused}"
    );
}

/// The index holds an entry of another kind: the listing refuses it by name.
#[test]
fn an_index_entry_of_another_kind_is_refused_by_name() {
    let fixture = Fixture::sqlite();
    let store = fixture.seeded();
    let begun = store.begin_stage().unwrap();
    forge(
        &fixture.path(),
        &fixture.tenant,
        "ekr.stage-index",
        "stages",
        "ekr.store.StageBegun",
        serde_json::json!({ "stage_id": begun.stage_id }),
    );
    let refused = refusal(store.stages());
    assert!(
        refused.contains("stage-index-disagrees: version 2 is not a StageIndexed entry"),
        "{refused}"
    );
}

/// (a), (b), (e) Probe: four begins at once, each through its own handle; a begin whose copy
/// fails (`stage-copy-failed`, its stage abandoned) is a legitimate outcome of contention and is
/// listed in its record's state. Then three further stages are begun, one sealed, one published
/// and one abandoned. The store lists every stage once, in the state its own record holds, and
/// lists again after every move; another store of the same provider lists none of them.
#[test]
fn concurrent_begins_and_every_state_list_once_each_and_only_in_their_store() {
    each(|fixture| {
        let store = fixture.seeded();
        commit(&store);
        let head = store.head().unwrap().unwrap().revision;
        let concurrent: Vec<(StageId, StageState)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| match fixture.open().begin_stage() {
                        Ok(result) => (result.stage_id, StageState::Begun),
                        Err(CommitError::Store(StoreError::StageCopyFailed {
                            stage_id,
                            state,
                            ..
                        })) => (stage_id, state),
                        Err(other) => panic!("a concurrent begin: {other}"),
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert_eq!(
            concurrent
                .iter()
                .map(|(id, _)| id)
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        let mut expected: BTreeMap<_, _> = concurrent.into_iter().collect();
        assert_eq!(listed(&store), expected, "after four concurrent begins");
        let begun: Vec<StageId> = (0..3)
            .map(|_| store.begin_stage().unwrap().stage_id)
            .collect();
        for id in &begun {
            expected.insert(*id, StageState::Begun);
        }
        assert_eq!(listed(&store), expected);

        commit(&store.join_stage(begun[1]).unwrap());
        store
            .seal_stage(begun[0], head)
            .expect("the first stage seals");
        store
            .seal_and_publish_stage(begun[1], head)
            .expect("the second stage publishes");
        store.abandon_stage(begun[2]).unwrap();
        expected.insert(begun[0], StageState::Sealing);
        expected.insert(begun[1], StageState::Published);
        expected.insert(begun[2], StageState::Abandoned);
        assert_eq!(listed(&store), expected);
        assert_eq!(
            listed(&fixture.open()),
            expected,
            "a fresh handle reads the same"
        );

        let neighbour = match &fixture.config {
            None => "adversary-u2-neighbour".to_owned(),
            Some(_) => format!("adversary-u2-neighbour-{}", TypeId::mint()),
        };
        let other = fixture.open_tenant(&neighbour);
        other.seed(seed(), || SEEDED_AT).unwrap();
        assert_eq!(listed(&other), BTreeMap::new(), "another store lists none");
        let theirs = other.begin_stage().unwrap().stage_id;
        assert_eq!(
            listed(&other),
            BTreeMap::from([(theirs, StageState::Begun)])
        );
        assert_eq!(listed(&store), expected, "nor does this store list theirs");
    });
}
