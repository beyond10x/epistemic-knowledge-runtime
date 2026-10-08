//! Real hosted PostgreSQL acceptance. Set EKR_TEST_POSTGRES_CONFIG and
//! EKR_TEST_POSTGRES_OWNER to file references; EKR_REQUIRE_POSTGRES=1 makes absence a failure.
//!
//! The stage-copy cases of design § 107.10 (unit C, `task:postgres-source-copy`) run their SQLite
//! halves without a database and their PostgreSQL halves where the PostgreSQL tests run. The
//! stage tenant's own case, which runs on every provider, is in `stage.rs`.
#[path = "support/hosted_store.rs"]
mod controlled;
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{context, seed, SEEDED_AT};
use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_graph::Root;
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, Commit, CommitCommandResult, GraphOperation,
    GraphTransaction, Runtime, StreamReads, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::NodeType;
use ekr_store::{ObjectStore, PostgresStore, RevisionLog, SqliteStore};
use std::collections::BTreeSet;
use std::path::Path;

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
        Runtime::postgres_schema(
            &PostgresConfiguration::read(std::path::Path::new(&owner)).unwrap(),
        )
        .unwrap();
    });
    Some(PostgresConfiguration::read(std::path::Path::new(&path)).unwrap())
}

fn anchor() -> AuthorityStateV1 {
    let mut anchor = current_fixture::anchor();
    anchor.validation_profile = ValidationProfileV1::schema_evolving(context().validator);
    anchor
}

fn tenant() -> String {
    format!("hosted-{}", TypeId::mint())
}

fn open(config: &PostgresConfiguration, tenant: &str, reading: bool) -> Runtime {
    Runtime::postgres(config, tenant, context(), anchor(), reading).unwrap()
}

fn extend(runtime: &Runtime, at: i64) {
    let transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Reading{at}"),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    submit(runtime, transaction, at);
}

fn submit(runtime: &Runtime, transaction: GraphTransaction, at: i64) {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .unwrap();
    runtime
        .propose(bytes.as_bytes(), context().operator, || {
            Timestamp::from_millis(at)
        })
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
}

#[test]
fn postgres_runtime_reopens_seed_and_committed_history() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let runtime = open(&config, &tenant, false);
    runtime.seed(seed(), || SEEDED_AT).unwrap();
    extend(&runtime, 20);
    let head = runtime.head().unwrap();
    // Design § 107.2, unit C: the inventory no longer refuses `postgres-inventory-requires-capture`;
    // it is read under one provider capture, never from the change feed.
    let raw = ekr_store::PostgresStore::postgres(&config, &tenant, false).unwrap();
    let _ = ekr_kernel::stream_reads();
    let inventory = raw.inventory().unwrap();
    assert_eq!(
        ekr_kernel::stream_reads(),
        StreamReads {
            captures: 1,
            ..StreamReads::default()
        },
        "the inventory is one provider capture and no stream or feed read"
    );
    assert_eq!(
        inventory.occurrences.len(),
        4,
        "the seed and one proposal, validation and commit"
    );
    let published: BTreeSet<_> = inventory
        .occurrences
        .iter()
        .map(|occurrence| occurrence.event.event_id)
        .collect();
    assert_eq!(
        inventory.prepared.iter().copied().collect::<BTreeSet<_>>(),
        published,
        "each decision's newest preparation is read from the capture and was published"
    );
    assert_eq!(inventory.events, raw.published_events().unwrap().len());
    drop(raw);
    drop(runtime);
    let reopened = open(&config, &tenant, true);
    assert_eq!(reopened.head().unwrap(), head);
    assert!(reopened.is_read_only());
    assert_eq!(
        reopened.replay(RevisionNumber::SEED).unwrap().nodes.len(),
        2
    );
    let events = reopened.published_events().unwrap();
    reopened.retain_checkpoint_at_rest();
    assert_eq!(reopened.published_events().unwrap(), events);
    let transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            "LaterReading",
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .unwrap();
    let error = reopened
        .propose(bytes.as_bytes(), context().operator, || {
            Timestamp::from_millis(30)
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("read-only"), "{error}");
    assert_eq!(open(&config, &tenant, true).head().unwrap(), head);
}

#[test]
fn sqlite_to_postgres_preserves_every_revision_schema_and_evidence_byte() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    let mut evidence = seed().graph.evidence.into_values().next().unwrap();
    let payload = b"synthetic retained evidence after the seed".to_vec();
    evidence.id = ekr_core::EvidenceId::mint();
    evidence.content_hash = ContentHash::of_bytes(&payload);
    submit(
        &source,
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![
                GraphOperation::AddEvidence(Box::new(ekr_kernel::EvidenceAddition {
                    evidence,
                    payload,
                })),
                GraphOperation::RetractAssertion(ekr_kernel::Retraction {
                    assertion: *seed().graph.assertions.keys().next().unwrap(),
                    reason: ekr_graph::RetractionReason::new("synthetic withdrawal"),
                }),
            ],
            evidence: BTreeSet::new(),
            schema_version: None,
        },
        24,
    );
    let captured = Runtime::sqlite_snapshot(&path, &tenant, context(), anchor()).unwrap();
    let captured_head = captured.head().unwrap();
    extend(&source, 30);
    assert_ne!(
        source.head().unwrap(),
        captured_head,
        "the source advances after capture"
    );
    let source_events = source.published_events().unwrap();
    let source_bytes = std::fs::read(&path).unwrap();
    let destination = open(&config, &tenant, false);
    let report = captured.migrate_into(&destination).unwrap();
    assert_ne!(report.source_seed_hash, report.destination_seed_hash);
    let migrated_head = destination.head().unwrap();
    let before = captured_head.unwrap();
    let after = migrated_head.unwrap();
    assert_eq!(before.revision, after.revision);
    assert_eq!(before.ontology_root, after.ontology_root);
    assert_eq!(before.knowledge_root, after.knowledge_root);
    assert_eq!(before.evidence_root, after.evidence_root);
    assert_eq!(before.agent_root, after.agent_root);
    for n in 0..=captured_head.unwrap().revision.get() {
        let revision = RevisionNumber::new(n);
        let before = captured.read(Some(revision)).unwrap();
        let after = destination.read(Some(revision)).unwrap();
        assert_eq!(before.graph, after.graph);
        let source_schema = captured.schema_history(revision).unwrap();
        let copied_schema = destination.schema_history(revision).unwrap();
        assert_eq!(source_schema.schemas, copied_schema.schemas);
        assert_eq!(
            source_schema.retained_evidence,
            copied_schema.retained_evidence
        );
        for (number, original) in &source_schema.revisions {
            let copied = &copied_schema.revisions[number];
            assert_eq!(original.event_id, copied.event_id);
            assert_eq!(original.revision_id, copied.revision_id);
            assert_eq!(original.committed_at, copied.committed_at);
            assert_eq!(original.root.ontology_root, copied.root.ontology_root);
            assert_eq!(original.root.knowledge_root, copied.root.knowledge_root);
            assert_eq!(original.root.evidence_root, copied.root.evidence_root);
            assert_eq!(original.root.agent_root, copied.root.agent_root);
        }
    }
    for evidence in captured.snapshot().unwrap().evidence.values() {
        let hash = &evidence.content_hash;
        assert_eq!(
            captured.content(hash).unwrap(),
            destination.content(hash).unwrap()
        );
        assert_eq!(
            ContentHash::of_bytes(&destination.content(hash).unwrap().unwrap()),
            *hash
        );
    }
    assert_eq!(source.published_events().unwrap(), source_events);
    assert_eq!(std::fs::read(path).unwrap(), source_bytes);
    drop(destination);
    assert_eq!(open(&config, &tenant, true).head().unwrap(), migrated_head);
}

#[test]
fn copy_refuses_nonempty_destination_without_mutation() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let source = Runtime::sqlite(
        &directory.path().join("source.db"),
        &tenant,
        context(),
        anchor(),
    )
    .unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    let destination = open(&config, &tenant, false);
    destination.seed(seed(), || SEEDED_AT).unwrap();
    let before = destination.head().unwrap();
    let error = source.migrate_into(&destination).unwrap_err().to_string();
    assert!(error.contains("migrate-destination-not-empty"), "{error}");
    assert_eq!(destination.head().unwrap(), before);
}

#[test]
fn postgres_hosted_open_refuses_schema_authority_and_unbounded_budget() {
    let Some(mut config) = config() else { return };
    let owner = std::env::var_os("EKR_TEST_POSTGRES_OWNER").unwrap();
    let owner = PostgresConfiguration::read(std::path::Path::new(&owner)).unwrap();
    let result = Runtime::postgres(&owner, &tenant(), context(), anchor(), false);
    assert!(
        result.is_err(),
        "schema owner must not be an application role"
    );
    config.database_connections = 1;
    let result = Runtime::postgres(&config, &tenant(), context(), anchor(), false);
    assert!(
        result.is_err(),
        "deployment connections exceed admitted budget"
    );
}

#[test]
fn postgres_pool_overrides_cannot_remove_connection_or_timeout_bounds() {
    let Some(mut config) = config() else { return };
    let default_pool = ekr_store::postgres::PostgresPool::default();
    config.pool = default_pool.clone();
    let runtime = Runtime::postgres(&config, &tenant(), context(), anchor(), true)
        .expect("default pool admits the provisioned fixture");
    drop(runtime);
    for pool in [
        ekr_store::postgres::PostgresPool {
            max_connections: 0,
            ..default_pool.clone()
        },
        ekr_store::postgres::PostgresPool {
            connect_timeout_ms: 0,
            ..default_pool.clone()
        },
        ekr_store::postgres::PostgresPool {
            shutdown_timeout_ms: 0,
            ..default_pool
        },
    ] {
        config.pool = pool;
        assert!(
            Runtime::postgres(&config, &tenant(), context(), anchor(), true).is_err(),
            "an explicit pool override must not remove a finite positive bound"
        );
    }
}

#[test]
fn interrupted_postgres_copy_is_unreadable_after_reopen() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    drop(source);
    let from = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    let into = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            before_initialize: Box::new(|| {}),
            interrupt_after_seed: true,
            interrupt_completion: false,
        })
    })
    .unwrap();
    assert!(from.migrate_into(&into).is_err());
    drop(into);
    let error = open(&config, &tenant, true).head().unwrap_err().to_string();
    assert!(error.contains("migrate-incomplete"), "{error}");
    let competitor = open(&config, &tenant, false);
    let events = competitor.published_events().unwrap();
    assert!(competitor.seed(seed(), || SEEDED_AT).is_err());
    assert_eq!(competitor.published_events().unwrap(), events);
}

#[test]
fn competing_ordinary_seed_is_not_poisoned_by_losing_copy() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    let from = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    let winner = open(&config, &tenant, false);
    let into = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            before_initialize: Box::new(move || {
                winner.seed(seed(), || SEEDED_AT).unwrap();
            }),
            interrupt_after_seed: false,
            interrupt_completion: false,
        })
    })
    .unwrap();
    assert!(from.migrate_into(&into).is_err());
    drop(into);
    assert_eq!(
        open(&config, &tenant, true)
            .head()
            .unwrap()
            .unwrap()
            .revision,
        RevisionNumber::SEED
    );
}

#[test]
fn identical_seed_copies_cannot_finish_another_interrupted_history() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    let first = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    extend(&source, 30);
    let second = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    let winner = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            before_initialize: Box::new(|| {}),
            interrupt_after_seed: true,
            interrupt_completion: false,
        })
    })
    .unwrap();
    let into = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            before_initialize: Box::new(move || {
                let error = first.migrate_into(&winner).unwrap_err().to_string();
                assert!(
                    error.contains("synthetic publication interruption"),
                    "{error}"
                );
            }),
            interrupt_after_seed: false,
            interrupt_completion: false,
        })
    })
    .unwrap();
    let error = second.migrate_into(&into).unwrap_err().to_string();
    assert!(error.contains("migrate-destination-not-empty"), "{error}");
    drop(into);
    assert!(open(&config, &tenant, true)
        .head()
        .unwrap_err()
        .to_string()
        .contains("migrate-incomplete"));
}

const OLD_START: &[u8] = br#"{"format":"ekr.migration-started/1"}"#;
const OLD_FINISH: &[u8] = br#"{"format":"ekr.migration-finished/1"}"#;

fn interrupted_copy(path: &std::path::Path, config: &PostgresConfiguration, tenant: &str) {
    let from = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(path, tenant, None)?.under(authority))
    })
    .unwrap();
    let into = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(config, tenant, false)?.under(authority),
            before_initialize: Box::new(|| {}),
            interrupt_after_seed: true,
            interrupt_completion: false,
        })
    })
    .unwrap();
    let error = from.migrate_into(&into).unwrap_err().to_string();
    assert!(
        error.contains("synthetic publication interruption"),
        "{error}"
    );
    drop(into);
    let error = open(config, tenant, true).head().unwrap_err().to_string();
    assert!(error.contains("migrate-incomplete"), "{error}");
}

#[test]
fn legacy_marker_evidence_cannot_complete_an_interrupted_copy() {
    let Some(config) = config() else { return };
    for bytes in [OLD_START, OLD_FINISH] {
        let tenant = tenant();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("source.db");
        let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
        let mut document = seed();
        let hash = ContentHash::of_bytes(bytes);
        document
            .graph
            .evidence
            .values_mut()
            .next()
            .unwrap()
            .content_hash = hash;
        document.evidence_payloads = [(hash, bytes.to_vec().into())].into();
        source.seed(document, || SEEDED_AT).unwrap();
        extend(&source, 20);
        drop(source);
        // Reopen to ensure marker-shaped evidence is never mistaken for legacy control.
        assert!(
            Runtime::sqlite_snapshot(&path, &tenant, context(), anchor())
                .unwrap()
                .head()
                .is_ok()
        );
        interrupted_copy(&path, &config, &tenant);
    }
}

#[test]
fn canonical_legacy_markers_are_carried_as_content_under_a_fresh_claim() {
    use ekr_store::ObjectStore;
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    drop(source);
    let raw = ekr_store::SqliteStore::sqlite(&path, &tenant, None).unwrap();
    for bytes in [OLD_START, OLD_FINISH] {
        raw.put(ekr_store::StorageClass::Canonical, bytes, SEEDED_AT)
            .unwrap();
    }
    drop(raw);
    interrupted_copy(&path, &config, &tenant);
    // A complete second copy into another empty tenant carries both exact marker objects.
    let other = self::tenant();
    let source = Runtime::sqlite_snapshot(&path, &tenant, context(), anchor()).unwrap();
    let destination = open(&config, &other, false);
    source.migrate_into(&destination).unwrap();
    for bytes in [OLD_START, OLD_FINISH] {
        assert_eq!(
            destination
                .content(&ContentHash::of_bytes(bytes))
                .unwrap()
                .as_deref(),
            Some(bytes)
        );
    }
    drop(destination);
    assert!(open(&config, &other, true).head().unwrap().is_some());
}

#[test]
fn a_claimed_copy_can_be_captured_and_copied_again() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let source = Runtime::sqlite(
        &directory.path().join("source.db"),
        &tenant,
        context(),
        anchor(),
    )
    .unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    let copy_path = directory.path().join("first-copy.db");
    let first = Runtime::sqlite(&copy_path, &tenant, context(), anchor()).unwrap();
    let first_map = source.migrate_into(&first).unwrap();
    let original: serde_json::Value = serde_json::from_slice(
        &source
            .content(&first_map.source_seed_hash)
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(original["format"], "ekr-seed-envelope/3");
    assert!(original.get("migration").is_none());
    drop(first);
    let captured = Runtime::sqlite_snapshot(&copy_path, &tenant, context(), anchor()).unwrap();
    let second = open(&config, &tenant, false);
    let second_map = captured.migrate_into(&second).unwrap();
    let first_seed: serde_json::Value = serde_json::from_slice(
        &captured
            .content(&first_map.destination_seed_hash)
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let second_seed: serde_json::Value = serde_json::from_slice(
        &second
            .content(&second_map.destination_seed_hash)
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(first_seed["format"], "ekr-seed-envelope/4");
    assert_eq!(second_seed["format"], "ekr-seed-envelope/4");
    assert_ne!(first_seed["migration"], second_seed["migration"]);
    assert_ne!(
        first_map.destination_seed_hash,
        second_map.destination_seed_hash
    );
    assert_eq!(captured.snapshot().unwrap(), second.snapshot().unwrap());
    assert_eq!(
        captured
            .schema_history(RevisionNumber::new(1))
            .unwrap()
            .schemas,
        second
            .schema_history(RevisionNumber::new(1))
            .unwrap()
            .schemas
    );
    drop(second);
    assert!(open(&config, &tenant, true).head().unwrap().is_some());
}

#[test]
fn copy_refuses_an_object_only_postgres_destination() {
    use ekr_store::ObjectStore;
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let source = Runtime::sqlite(
        &directory.path().join("source.db"),
        &tenant,
        context(),
        anchor(),
    )
    .unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    let raw = ekr_store::PostgresStore::postgres(&config, &tenant, false).unwrap();
    let bytes = b"synthetic unrelated retained object";
    raw.put(ekr_store::StorageClass::Canonical, bytes, SEEDED_AT)
        .unwrap();
    drop(raw);
    let destination = open(&config, &tenant, false);
    let error = source.migrate_into(&destination).unwrap_err().to_string();
    assert!(error.contains("migrate-destination-not-empty"), "{error}");
    assert_eq!(
        destination
            .content(&ContentHash::of_bytes(bytes))
            .unwrap()
            .as_deref(),
        Some(bytes.as_slice())
    );
}

#[test]
fn a_checkpoint_cannot_admit_a_copy_without_its_completion_receipt() {
    use ekr_store::RevisionLog;
    let Some(config) = config() else { return };
    let tenant = tenant();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.db");
    let source = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    source.seed(seed(), || SEEDED_AT).unwrap();
    extend(&source, 20);
    drop(source);
    let from = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(ekr_store::SqliteStore::sqlite_read_only(&path, &tenant, None)?.under(authority))
    })
    .unwrap();
    let into = ekr_kernel::Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: ekr_store::PostgresStore::postgres(&config, &tenant, false)?.under(authority),
            before_initialize: Box::new(|| {}),
            interrupt_after_seed: false,
            interrupt_completion: true,
        })
    })
    .unwrap();
    let error = from.migrate_into(&into).unwrap_err().to_string();
    assert!(
        error.contains("synthetic completion interruption"),
        "{error}"
    );
    drop(into);
    let raw = ekr_store::PostgresStore::postgres(&config, &tenant, true).unwrap();
    assert!(
        raw.checkpoint_covered().unwrap().is_some(),
        "the interruption must follow checkpoint publication"
    );
    drop(raw);
    let reopened = open(&config, &tenant, true);
    let error = reopened.head().unwrap_err().to_string();
    assert!(error.contains("migrate-incomplete"), "{error}");
    assert!(reopened
        .snapshot()
        .unwrap_err()
        .to_string()
        .contains("migrate-incomplete"));
}

// Design § 107.10, unit C (`task:postgres-source-copy`): a stage is begun by a preserving copy of
// the store at its head into the stage's own tenant of the same store. The source is read once —
// one SQLite image, one PostgreSQL capture (`Commit::capture`) — and the capture is then copied
// (`CapturedStore::copy_into`). A stage id is minted as begin mints it (`ekr.store.StageId`).

fn stage_id() -> ekr_core::StageId {
    ekr_core::StageId::mint()
}

/// A seeded store with a schema change, retained evidence added after the seed and a retraction.
fn populate(runtime: &Runtime) {
    runtime.seed(seed(), || SEEDED_AT).unwrap();
    extend(runtime, 20);
    let mut evidence = seed().graph.evidence.into_values().next().unwrap();
    let payload = b"synthetic retained evidence a stage carries".to_vec();
    evidence.id = ekr_core::EvidenceId::mint();
    evidence.content_hash = ContentHash::of_bytes(&payload);
    submit(
        runtime,
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![
                GraphOperation::AddEvidence(Box::new(ekr_kernel::EvidenceAddition {
                    evidence,
                    payload,
                })),
                GraphOperation::RetractAssertion(ekr_kernel::Retraction {
                    assertion: *seed().graph.assertions.keys().next().unwrap(),
                    reason: ekr_graph::RetractionReason::new("synthetic withdrawal"),
                }),
            ],
            evidence: BTreeSet::new(),
            schema_version: None,
        },
        24,
    );
}

/// A kernel over a PostgreSQL tenant; `full` replays every read from the seed.
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

/// A kernel over a SQLite tenant of the database at `path`; `full` replays every read from the
/// seed.
fn sqlite_kernel(path: &Path, tenant: &str, full: bool) -> Commit<SqliteStore> {
    Commit::over_with_authority(context(), anchor(), |authority| {
        let mut store = SqliteStore::sqlite(path, tenant, None)?;
        store.set_full_replay(full);
        Ok(store.under(authority))
    })
    .unwrap()
}

/// A kernel over one SQLite image of `tenant` in the database at `path`, as
/// `Runtime::sqlite_snapshot` takes it.
fn sqlite_image(path: &Path, tenant: &str) -> Commit<SqliteStore> {
    Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(SqliteStore::sqlite_read_only(path, tenant, None)?.under(authority))
    })
    .unwrap()
}

/// The stage, replayed in full from its seed, holds the source's lineage through `at` and nothing
/// after it: every revision's identities, time, graph and knowledge, evidence, ontology and
/// authority roots, every schema and every retained evidence byte. Its head is `at`.
fn assert_stage_holds<S, D>(source: &Commit<S>, at: Root, stage: &Commit<D>)
where
    S: RevisionLog + ObjectStore,
    D: RevisionLog + ObjectStore,
{
    let head = stage.head().unwrap().expect("the stage is seeded");
    assert_eq!(
        (
            head.revision,
            head.ontology_root,
            head.knowledge_root,
            head.evidence_root,
            head.agent_root
        ),
        (
            at.revision,
            at.ontology_root,
            at.knowledge_root,
            at.evidence_root,
            at.agent_root
        ),
        "the stage's head is the captured head"
    );
    let after = RevisionNumber::new(at.revision.get() + 1);
    assert!(
        stage.read(Some(after)).is_err(),
        "the stage holds no revision after the captured head"
    );
    for n in 0..=at.revision.get() {
        let revision = RevisionNumber::new(n);
        assert_eq!(
            source.read(Some(revision)).unwrap().graph,
            stage.read(Some(revision)).unwrap().graph,
            "revision {n}"
        );
        let original = source.schema_history(revision).unwrap();
        let copied = stage.schema_history(revision).unwrap();
        assert_eq!(original.schemas, copied.schemas, "revision {n}");
        assert_eq!(
            original.retained_evidence, copied.retained_evidence,
            "revision {n}"
        );
        for (number, held) in &original.revisions {
            let staged = &copied.revisions[number];
            assert_eq!(held.event_id, staged.event_id);
            assert_eq!(held.revision_id, staged.revision_id);
            assert_eq!(held.committed_at, staged.committed_at);
            assert_eq!(held.root.ontology_root, staged.root.ontology_root);
            assert_eq!(held.root.knowledge_root, staged.root.knowledge_root);
            assert_eq!(held.root.evidence_root, staged.root.evidence_root);
            assert_eq!(held.root.agent_root, staged.root.agent_root);
        }
    }
    let evidence = source
        .read(Some(at.revision))
        .unwrap()
        .graph
        .evidence
        .clone();
    assert!(evidence.len() >= 2, "the seed's evidence and the added one");
    for held in evidence.values() {
        let hash = &held.content_hash;
        let bytes = stage.content(hash).unwrap().expect("the stage retains it");
        assert_eq!(source.content(hash).unwrap().as_deref(), Some(&bytes[..]));
        assert_eq!(ContentHash::of_bytes(&bytes), *hash);
    }
}

#[test]
fn a_postgres_store_is_copied_into_a_stage_under_one_capture() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let store = open(&config, &tenant, false);
    populate(&store);
    let events = store.published_events().unwrap();
    let source = postgres_kernel(&config, &tenant, false, false);
    let _ = ekr_kernel::stream_reads();
    let captured = source.capture().unwrap();
    assert_eq!(
        ekr_kernel::stream_reads(),
        StreamReads {
            captures: 1,
            ..StreamReads::default()
        },
        "the source is read under one provider capture and nothing else"
    );
    let at = captured.head();
    assert_eq!(store.head().unwrap(), Some(at));
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    let stage = postgres_kernel(&config, &staged, false, false);
    let report = captured.copy_into(&stage).unwrap();
    assert_ne!(report.source_seed_hash, report.destination_seed_hash);
    drop(stage);
    assert_eq!(
        store.published_events().unwrap(),
        events,
        "the store is only read"
    );
    assert_eq!(store.head().unwrap(), Some(at));
    assert_stage_holds(&source, at, &postgres_kernel(&config, &staged, true, true));
}

#[test]
fn a_sqlite_store_is_copied_into_a_stage_under_one_image() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.db");
    let tenant = tenant();
    let store = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    populate(&store);
    let events = store.published_events().unwrap();
    let source = sqlite_image(&path, &tenant);
    let captured = source.capture().unwrap();
    let at = captured.head();
    assert_eq!(store.head().unwrap(), Some(at));
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    let stage = sqlite_kernel(&path, &staged, false);
    captured.copy_into(&stage).unwrap();
    drop(stage);
    assert_eq!(
        store.published_events().unwrap(),
        events,
        "the store is only read"
    );
    assert_eq!(store.head().unwrap(), Some(at));
    assert_stage_holds(&source, at, &sqlite_kernel(&path, &staged, true));
}

#[test]
fn a_commit_racing_the_capture_is_not_in_the_stage() {
    // SQLite: the image is the capture. A commit after the image is taken is not in it, even
    // though the copy reads the image afterwards.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.db");
    let tenant = tenant();
    let store = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    populate(&store);
    let source = sqlite_image(&path, &tenant);
    extend(&store, 30);
    let captured = source.capture().unwrap();
    let at = captured.head();
    let moved = store.head().unwrap().unwrap();
    assert!(
        moved.revision > at.revision,
        "the store moved past the image"
    );
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    captured
        .copy_into(&sqlite_kernel(&path, &staged, false))
        .unwrap();
    assert_eq!(store.head().unwrap(), Some(moved));
    assert_stage_holds(&source, at, &sqlite_kernel(&path, &staged, true));

    // PostgreSQL: a commit lands after the capture, while the copy is writing the stage.
    let Some(config) = config() else { return };
    let tenant = self::tenant();
    let store = open(&config, &tenant, false);
    populate(&store);
    let source = postgres_kernel(&config, &tenant, false, false);
    let captured = source.capture().unwrap();
    let at = captured.head();
    let staged = ekr_store::stage_tenant(&tenant, stage_id()).unwrap();
    let racer = open(&config, &tenant, false);
    let stage = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(controlled::Controlled {
            inner: PostgresStore::postgres(&config, &staged, false)?.under(authority),
            before_initialize: Box::new(move || extend(&racer, 30)),
            interrupt_after_seed: false,
            interrupt_completion: false,
        })
    })
    .unwrap();
    captured.copy_into(&stage).unwrap();
    drop(stage);
    let moved = store.head().unwrap().unwrap();
    assert!(
        moved.revision > at.revision,
        "the racing commit landed in the store"
    );
    assert_stage_holds(&source, at, &postgres_kernel(&config, &staged, true, true));
}

/// Interrupts a copy of `captured` into a fresh stage tenant, once just after the stage's seed and
/// once just before its completion receipt, and hands each stage tenant to `refused`.
fn interrupted_copies<S, D>(
    captured: &ekr_kernel::CapturedStore<'_, S>,
    store_tenant: &str,
    open: impl Fn(&str, ekr_kernel::KernelAuthority) -> D,
    refused: impl Fn(&str),
) where
    S: RevisionLog + ObjectStore + ekr_store::Inventory,
    D: RevisionLog + ObjectStore + ekr_store::Initialize + ekr_store::Inventory,
{
    for (after_seed, completion, says) in [
        (true, false, "synthetic publication interruption"),
        (false, true, "synthetic completion interruption"),
    ] {
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
        let error = captured.copy_into(&into).unwrap_err().to_string();
        assert!(error.contains(says), "{error}");
        drop(into);
        refused(&staged);
    }
}

#[test]
fn an_interrupted_stage_copy_is_refused_as_incomplete_on_postgres() {
    let Some(config) = config() else { return };
    let tenant = tenant();
    let store = open(&config, &tenant, false);
    populate(&store);
    let events = store.published_events().unwrap();
    let source = postgres_kernel(&config, &tenant, false, false);
    let captured = source.capture().unwrap();
    let at = captured.head();
    interrupted_copies(
        &captured,
        &tenant,
        |staged, authority| {
            PostgresStore::postgres(&config, staged, false)
                .unwrap()
                .under(authority)
        },
        |staged| {
            let stage = postgres_kernel(&config, staged, true, false);
            for error in [
                stage.head().map(|_| ()).unwrap_err().to_string(),
                stage.snapshot().map(|_| ()).unwrap_err().to_string(),
                stage.read(None).map(|_| ()).unwrap_err().to_string(),
            ] {
                assert!(error.contains("migrate-incomplete"), "{error}");
            }
        },
    );
    assert_eq!(store.published_events().unwrap(), events);
    assert_eq!(store.head().unwrap(), Some(at));
}

#[test]
fn an_interrupted_stage_copy_is_refused_as_incomplete_on_sqlite() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store.db");
    let tenant = tenant();
    let store = Runtime::sqlite(&path, &tenant, context(), anchor()).unwrap();
    populate(&store);
    let events = store.published_events().unwrap();
    let source = sqlite_image(&path, &tenant);
    let captured = source.capture().unwrap();
    let at = captured.head();
    interrupted_copies(
        &captured,
        &tenant,
        |staged, authority| {
            SqliteStore::sqlite(&path, staged, None)
                .unwrap()
                .under(authority)
        },
        |staged| {
            let stage = sqlite_kernel(&path, staged, false);
            for error in [
                stage.head().map(|_| ()).unwrap_err().to_string(),
                stage.snapshot().map(|_| ()).unwrap_err().to_string(),
                stage.read(None).map(|_| ()).unwrap_err().to_string(),
            ] {
                assert!(error.contains("migrate-incomplete"), "{error}");
            }
        },
    );
    assert_eq!(store.published_events().unwrap(), events);
    assert_eq!(store.head().unwrap(), Some(at));
}
