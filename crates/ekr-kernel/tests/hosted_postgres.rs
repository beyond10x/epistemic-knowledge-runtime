//! Real hosted PostgreSQL acceptance. Set EKR_TEST_POSTGRES_CONFIG and
//! EKR_TEST_POSTGRES_OWNER to file references; EKR_REQUIRE_POSTGRES=1 makes absence a failure.
#[path = "support/hosted_store.rs"]
mod controlled;
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{context, seed, SEEDED_AT};
use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    runtime::PostgresConfiguration, AuthorityStateV1, CommitCommandResult, GraphOperation,
    GraphTransaction, Runtime, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::NodeType;
use std::collections::BTreeSet;

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
    let raw = ekr_store::PostgresStore::postgres(&config, &tenant, false).unwrap();
    assert!(raw
        .inventory()
        .unwrap_err()
        .to_string()
        .contains("postgres-inventory-requires-capture"));
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
