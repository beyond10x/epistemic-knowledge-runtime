//! Supplied observations are durable without a canonical revision, on both providers.
use ekr_core::contract_data::{EkrKernelAttentionKind, EkrObserveObservationImport};
use ekr_core::{bytes, AgentId, ContentHash, ObservationId, Timestamp};
use ekr_kernel::{Agent, AuthorityStateV1, BootstrapContext, Runtime, ValidationProfileV1};
use std::path::Path;

fn host() -> (BootstrapContext, AuthorityStateV1) {
    let context = BootstrapContext {
        operator: AgentId::mint(),
        validator: AgentId::mint(),
    };
    let anchor = AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [context.operator, context.validator]
            .into_iter()
            .map(|id| {
                (
                    id,
                    Agent {
                        id,
                        name: id.to_string(),
                        capabilities: Default::default(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(context.validator),
    };
    (context, anchor)
}

fn open(
    path: &Path,
    sqlite: bool,
    context: BootstrapContext,
    anchor: &AuthorityStateV1,
) -> Runtime {
    if sqlite {
        Runtime::sqlite(path, "retention", context, anchor.clone()).unwrap()
    } else {
        Runtime::file(path, "retention", context, anchor.clone()).unwrap()
    }
}

fn observation() -> EkrObserveObservationImport {
    let payload = b"An unmapped project health observation.\n";
    let hash = ContentHash::of_bytes(payload);
    let id = ekr_core::ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: Some("health-1".into()),
        content_hash: hash,
    }
    .observation_id();
    serde_json::from_value(serde_json::json!({
        "observation": {"observation_id": id, "source":"manual", "source_native_id":"health-1",
            "content_hash":hash.to_hex(), "captured_at":"1970-01-01T00:00:00.017Z", "kind":"FeedItem"},
        "key":{"source":"manual", "source_native_id":"health-1", "content_hash":hash.to_hex()},
        "payload":bytes::encode(payload)
    }))
    .unwrap()
}

#[test]
fn generated_observation_commands_preserve_provider_faults() {
    use ekr_kernel::Commit;
    use ekr_store::{
        FileStore, ObjectStore, ObservationRetention, RevisionLog, SqliteStore, StoreError,
    };

    fn check<S: RevisionLog + ObjectStore + ObservationRetention>(
        commit: Commit<S>,
        original: &EkrObserveObservationImport,
    ) {
        assert_eq!(
            commit.observations().unwrap(),
            [*original.observation.clone()]
        );
        let mut input = original.clone();
        let payload = b"another observation";
        let hash = ContentHash::of_bytes(payload);
        let id = ekr_core::ObservationIdempotencyKey {
            source: "manual".into(),
            source_native_id: Some("health-1".into()),
            content_hash: hash,
        }
        .observation_id();
        input.payload = bytes::encode(payload);
        input.key.content_hash.0 = hash.to_hex();
        input.observation.content_hash.0 = hash.to_hex();
        input.observation.observation_id.0 = id.to_string();
        let refused = commit
            .import_observation(&input, Timestamp::EPOCH)
            .unwrap_err();
        assert!(
            matches!(refused, StoreError::ReadOnly(_)),
            "provider fault was reclassified: {refused:?}"
        );
        assert_eq!(
            commit.observations().unwrap(),
            [*original.observation.clone()]
        );
    }

    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (context, anchor) = host();
        let original = observation();
        let runtime = open(&path, sqlite, context, &anchor);
        runtime
            .import_observation(&original, Timestamp::EPOCH)
            .unwrap();
        drop(runtime);
        if sqlite {
            check(
                Commit::over_with_authority(context, anchor, |authority| {
                    SqliteStore::sqlite_read_only(&path, "retention", None)
                        .map(|store| store.under(authority))
                })
                .unwrap(),
                &original,
            );
        } else {
            check(
                Commit::over_with_authority(context, anchor, |authority| {
                    FileStore::file_read_only(&path, "retention", None)
                        .map(|store| store.under(authority))
                })
                .unwrap(),
                &original,
            );
        }
    }
}

#[test]
fn observation_retry_is_idempotent() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, anchor) = host();
        let input = observation();
        let id: ObservationId = input.observation.observation_id.0.parse().unwrap();
        let runtime = open(&path, sqlite, context, &anchor);
        assert!(runtime.head().unwrap().is_none());
        let first = runtime
            .import_observation(&input, Timestamp::from_millis(21))
            .unwrap();
        assert!(!first.already_retained);
        let events = runtime.published_events().unwrap().len();
        let retry = runtime
            .import_observation(&input, Timestamp::from_millis(88))
            .unwrap();
        assert!(retry.already_retained);
        assert_eq!(first.observation_id, retry.observation_id);
        assert_eq!(events, runtime.published_events().unwrap().len());
        assert!(runtime.head().unwrap().is_none());
        drop(runtime);
        let reopened = open(&path, sqlite, context, &anchor);
        assert_eq!(reopened.observations().unwrap().len(), 1);
        let retained = reopened.observation(id).unwrap();
        assert_eq!(retained.observation, input.observation);
        assert_eq!(retained.payload, input.payload);
        assert!(
            reopened
                .import_observation(&input, Timestamp::from_millis(99))
                .unwrap()
                .already_retained
        );
        assert_eq!(events, reopened.published_events().unwrap().len());
        let destination = open(&dir.path().join("destination"), sqlite, context, &anchor);
        let refused = reopened.migrate_into(&destination).unwrap_err().to_string();
        assert!(refused.contains("migrate-uncarried-streams"), "{refused}");
        assert!(destination.published_events().unwrap().is_empty());
        assert_eq!(reopened.observations().unwrap().len(), 1);
    }
}

#[test]
fn inconsistent_observation_is_refused_without_retaining_anything() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (context, anchor) = host();
        let runtime = open(&dir.path().join("store"), sqlite, context, &anchor);
        let mut input = observation();
        input.payload = bytes::encode(b"different bytes");
        assert!(runtime
            .import_observation(&input, Timestamp::EPOCH)
            .is_err());
        assert!(runtime.observations().unwrap().is_empty());
        assert!(runtime.published_events().unwrap().is_empty());
        let mut invalid_time = serde_json::to_value(observation()).unwrap();
        invalid_time["observation"]["captured_at"] = "17".into();
        assert!(serde_json::from_value::<EkrObserveObservationImport>(invalid_time).is_err());
        // Direct typed construction can still carry a time outside RFC 3339's year range.
        // The runtime must refuse that value before any provider mutation.
        let mut invalid_time = observation();
        invalid_time.observation.captured_at =
            time::Date::from_calendar_date(-1, time::Month::January, 1)
                .unwrap()
                .midnight()
                .assume_utc()
                .into();
        assert!(runtime
            .import_observation(&invalid_time, Timestamp::EPOCH)
            .is_err());
        assert!(runtime.published_events().unwrap().is_empty());
    }
}

#[test]
fn retained_sources_pin_exact_bytes_on_both_providers() {
    use ekr_store::{Inventory, ObjectStore, ObservationRetention, StorageClass};
    fn check<S: ObservationRetention + ObjectStore + Inventory>(store: S) {
        let input = observation();
        assert!(store.retain_observation(&input, Timestamp::EPOCH).unwrap());
        let hash = input.key.content_hash.0.parse().unwrap();
        assert_eq!(
            store.inventory().unwrap().objects[&hash]
                .object
                .metadata
                .storage_class,
            StorageClass::Provenance
        );
        assert_eq!(
            store.get(&hash).unwrap().unwrap(),
            bytes::decode(&input.payload).unwrap()
        );
        assert_eq!(store.retained_observations().unwrap(), [input]);
    }
    let dir = tempfile::tempdir().unwrap();
    check(ekr_store::FileStore::file(&dir.path().join("file"), "retention", None).unwrap());
    check(ekr_store::SqliteStore::sqlite(&dir.path().join("sqlite"), "retention", None).unwrap());
}

fn interpretation(
    source: &EkrObserveObservationImport,
    operator: AgentId,
) -> ekr_core::contract_data::EkrIntegrateInterpretationImport {
    use ekr_core::canonical::Canonical;
    use ekr_core::generated_identity::{Identity, InterpretationId};
    let evidence = ekr_core::EvidenceId::mint();
    let reference = serde_json::json!({"node_type":"Project", "aliases":["Maple"]});
    let document = serde_json::json!({
        "version":{"interpretation_id":InterpretationId::mint(), "version":1},
        "root_id":ekr_core::GraphRootId::mint(), "observations":[source.observation.observation_id],
        "local_schema":{"node_types":[{"name":"Project", "parents":[], "abstract_type":false,
            "properties":[{"name":"health", "value":{"value_kind":"String"}, "required":false, "cardinality":"One"}]}], "edge_types":[]},
        "entities":[reference.clone()],
        "facts":[{"kind":"Property", "value":{"subject":reference, "property":"health",
            "value":{"kind":"String", "canonical_bytes":bytes::encode(&ekr_graph::CanonicalValue::String("amber".into()).canonical_bytes())}, "evidence":[evidence]}}],
        "evidence":[{"evidence":{"id":evidence, "source":{"kind":"Observation", "observation":source.observation.observation_id},
            "content_hash":source.observation.content_hash, "observed_at":"1970-01-01T00:00:00.017Z", "extracted_by":operator, "confidence_bp":7500}, "payload":source.payload}]
    });
    let payload = bytes::encode(&serde_json::to_vec_pretty(&document).unwrap());
    serde_json::from_value(serde_json::json!({"document":document, "payload":payload})).unwrap()
}

#[test]
fn schema_gap_discovery_is_stable_after_replay_and_does_not_mutate_canonical_state() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, anchor) = host();
        let runtime = open(&path, sqlite, context, &anchor);
        assert!(runtime
            .discover_schema_gaps()
            .unwrap_err()
            .to_string()
            .contains("not-seeded"));
        runtime
            .seed(
                ekr_kernel::SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml"))
                    .unwrap(),
                || Timestamp::EPOCH,
            )
            .unwrap();
        let canonical = runtime.read(None).unwrap().root;
        let source = observation();
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .unwrap();
        let first = interpretation(&source, context.operator);
        let second = interpretation(&source, context.operator);
        runtime
            .import_interpretation(&first, Timestamp::EPOCH)
            .unwrap();
        runtime
            .import_interpretation(&second, Timestamp::EPOCH)
            .unwrap();
        let before = runtime.published_events().unwrap();
        let request = runtime.discover_schema_gaps().unwrap();
        assert_eq!(
            request.base_schema.0,
            "00000000-0000-4000-8000-000000000001"
        );
        assert_eq!(request.groups.len(), 1);
        let group = &request.groups[0];
        assert_eq!(group.declaration, "Project");
        assert_eq!(serde_json::to_value(&group.kind).unwrap(), "UnknownType");
        assert_eq!(group.blockers.len(), 4);
        assert_eq!(group.sources.len(), 2);
        assert_eq!(group.observations.len(), 1);
        assert_eq!(request.evidence.len(), 2);
        assert_eq!(runtime.discover_schema_gaps().unwrap(), request);
        assert_eq!(runtime.published_events().unwrap(), before);
        assert_eq!(runtime.read(None).unwrap().root, canonical);
        drop(runtime);
        let mut runtime = open(&path, sqlite, context, &anchor);
        runtime.set_full_replay(true);
        assert_eq!(runtime.discover_schema_gaps().unwrap(), request);
        runtime
            .import_interpretation(&first, Timestamp::EPOCH)
            .unwrap();
        assert_eq!(runtime.discover_schema_gaps().unwrap(), request);
        assert_eq!(runtime.read(None).unwrap().root, canonical);
    }
}

#[test]
fn schema_gap_discovery_rechecks_the_current_schema_and_preserves_import_history() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, mut anchor) = host();
        anchor.validation_profile = ValidationProfileV1::schema_evolving(context.validator);
        let runtime = open(&path, sqlite, context, &anchor);
        runtime
            .seed(
                ekr_kernel::SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml"))
                    .unwrap(),
                || Timestamp::EPOCH,
            )
            .unwrap();
        let source = observation();
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .unwrap();
        let input = interpretation(&source, context.operator);
        let receipt = runtime
            .import_interpretation(&input, Timestamp::EPOCH)
            .unwrap();
        let historical = runtime.interpretation(&receipt.version).unwrap();
        let original = runtime.discover_schema_gaps().unwrap();
        assert_eq!(
            serde_json::to_value(&original.groups[0].kind).unwrap(),
            "UnknownType"
        );
        let tx = ekr_kernel::GraphTransaction {
            id: ekr_core::TransactionId::mint(),
            proposer: context.operator,
            schema_version: Some(ekr_core::SchemaVersionId::mint()),
            evidence: Default::default(),
            operations: vec![ekr_kernel::GraphOperation::DefineNodeType(Box::new(
                ekr_ontology::NodeType::new(ekr_core::TypeId::mint(), "Project"),
            ))],
        };
        #[derive(serde::Serialize)]
        struct Envelope<'a> {
            format: &'static str,
            transaction: &'a ekr_kernel::GraphTransaction,
        }
        let bytes = serde_yaml_ng::to_string(&Envelope {
            format: "ekr.transaction-document/1",
            transaction: &tx,
        })
        .unwrap();
        runtime
            .propose(bytes.as_bytes(), context.operator, || {
                Timestamp::from_millis(1)
            })
            .unwrap();
        let validated = runtime
            .validate(tx.id, ekr_core::RevisionNumber::SEED, || {
                Timestamp::from_millis(2)
            })
            .unwrap();
        assert!(
            matches!(validated, ekr_kernel::ValidationCommandResult::Validated(_)),
            "{validated:?}"
        );
        let committed = runtime
            .commit(tx.id, context.operator, || Timestamp::from_millis(3))
            .unwrap();
        assert!(
            matches!(committed, ekr_kernel::CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
        let before = runtime.published_events().unwrap();
        let root = runtime.read(None).unwrap().root;
        let current = runtime.discover_schema_gaps().unwrap();
        assert_eq!(
            current.base_schema.0,
            tx.schema_version.unwrap().to_string()
        );
        assert_eq!(current.groups.len(), 1);
        assert_eq!(current.groups[0].declaration, "Project.health");
        assert_eq!(
            serde_json::to_value(&current.groups[0].kind).unwrap(),
            "UnknownProperty"
        );
        assert_eq!(current.groups[0].blockers.len(), 1);
        assert_eq!(
            runtime.interpretation(&receipt.version).unwrap(),
            historical
        );
        assert_eq!(runtime.published_events().unwrap(), before);
        assert_eq!(runtime.read(None).unwrap().root, root);
        drop(runtime);
        let mut runtime = open(&path, sqlite, context, &anchor);
        runtime.set_full_replay(true);
        assert_eq!(runtime.discover_schema_gaps().unwrap(), current);
        assert_eq!(
            runtime.interpretation(&receipt.version).unwrap(),
            historical
        );
    }
}

#[test]
fn unmapped_knowledge_survives_reopen() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, anchor) = host();
        let source = observation();
        let input = interpretation(&source, context.operator);
        let runtime = open(&path, sqlite, context, &anchor);
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .unwrap();
        let first = runtime
            .import_interpretation(&input, Timestamp::EPOCH)
            .unwrap();
        assert!(!first.already_retained);
        assert_eq!(
            serde_json::to_value(&first.outcome).unwrap(),
            "RetainedWithBlockers"
        );
        let events = runtime.published_events().unwrap().len();
        let before = runtime.interpretation(&first.version).unwrap();
        assert_eq!(before.document, input.document);
        assert_eq!(before.blockers.len(), 2);
        let questions = runtime.attention().unwrap();
        assert_eq!(questions.len(), before.blockers.len());
        for question in &questions {
            assert_eq!(*question.subject.kind, EkrKernelAttentionKind::V0);
            assert_eq!(question.observations, input.document.observations);
            assert!(!question.evidence.is_empty());
            assert!(question.claims.is_empty());
            assert_eq!(
                runtime.attention_item(&question.subject).unwrap(),
                *question
            );
        }
        assert_eq!(before.receipts.len(), 1);
        assert_eq!(
            serde_json::to_value(&before.receipts[0].disposition).unwrap(),
            "Parked"
        );
        let graph = runtime.incubation_graph(&first.version).unwrap();
        assert_eq!(graph.root.id.to_string(), input.document.root_id.0);
        assert_eq!(graph.root.space, ekr_graph::Space::Transient);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.assertions.len(), 1);
        assert!(graph.assertions.values().all(|a| !a.is_current()));
        assert!(runtime.head().unwrap().is_none());
        drop(runtime);
        let reopened = open(&path, sqlite, context, &anchor);
        assert_eq!(
            reopened.interpretations().unwrap(),
            [*first.version.clone()]
        );
        assert_eq!(reopened.interpretation(&first.version).unwrap(), before);
        assert_eq!(reopened.attention().unwrap(), questions);
        let retry = reopened
            .import_interpretation(&input, Timestamp::from_millis(33))
            .unwrap();
        assert!(retry.already_retained);
        assert_eq!(retry.blockers, first.blockers);
        assert_eq!(reopened.published_events().unwrap().len(), events);
        let mut changed_bytes = input.clone();
        let mut payload = bytes::decode(&changed_bytes.payload).unwrap();
        payload.push(b'\n');
        changed_bytes.payload = bytes::encode(&payload);
        assert!(reopened
            .import_interpretation(&changed_bytes, Timestamp::EPOCH)
            .is_err());
        assert_eq!(reopened.published_events().unwrap().len(), events);
        let mut changed_version = (*first.version).clone();
        changed_version.document_digest.0 = ContentHash::of_bytes(b"different").to_hex();
        assert!(reopened.interpretation(&changed_version).is_err());
    }
}

#[test]
fn rejected_interpretation_retains_its_sources() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, anchor) = host();
        let source = observation();
        let mut input = interpretation(&source, context.operator);
        input.document.local_schema.node_types[0].properties.clear();
        input.payload = bytes::encode(&serde_json::to_vec(&input.document).unwrap());
        let runtime = open(&path, sqlite, context, &anchor);
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .unwrap();
        let events = runtime.published_events().unwrap().len();
        assert!(runtime
            .import_interpretation(&input, Timestamp::EPOCH)
            .is_err());
        assert!(runtime.interpretations().unwrap().is_empty());
        assert_eq!(runtime.published_events().unwrap().len(), events);
        drop(runtime);
        let reopened = open(&path, sqlite, context, &anchor);
        let id = source.observation.observation_id.0.parse().unwrap();
        assert_eq!(reopened.observation(id).unwrap().payload, source.payload);
        assert!(reopened.head().unwrap().is_none());
    }
}

#[test]
fn concurrent_imports_choose_one_immutable_record() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let (context, anchor) = host();
        drop(open(&path, sqlite, context, &anchor));
        let source = observation();
        let input = interpretation(&source, context.operator);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let mut children = Vec::new();
            for time in [31, 32] {
                let barrier = barrier.clone();
                let source = &source;
                let input = &input;
                let path = &path;
                let anchor = &anchor;
                children.push(scope.spawn(move || {
                    barrier.wait();
                    let runtime = open(path, sqlite, context, anchor);
                    let observation =
                        runtime.import_observation(source, Timestamp::from_millis(time));
                    assert!(
                        observation.is_ok(),
                        "concurrent observation: {observation:?}"
                    );
                    let interpretation =
                        runtime.import_interpretation(input, Timestamp::from_millis(time));
                    assert!(
                        interpretation.is_ok(),
                        "concurrent interpretation: {interpretation:?}"
                    );
                    (observation.unwrap(), interpretation.unwrap())
                }));
            }
            children
                .into_iter()
                .map(|child| child.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| !r.0.already_retained).count(), 1);
        assert_eq!(results.iter().filter(|r| !r.1.already_retained).count(), 1);
        assert_eq!(results[0].1.blockers, results[1].1.blockers);
        let reopened = open(&path, sqlite, context, &anchor);
        assert_eq!(reopened.observations().unwrap().len(), 1);
        assert_eq!(reopened.interpretations().unwrap().len(), 1);
    }
}

#[test]
fn invalid_local_shape_and_evidence_are_refused_before_any_publication() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (context, anchor) = host();
        let runtime = open(&directory.path().join("store"), sqlite, context, &anchor);
        let source = observation();
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .unwrap();
        let held = runtime.published_events().unwrap().len();
        let original = interpretation(&source, context.operator);
        let original = serde_json::to_value(original.document).unwrap();
        for (pointer, replacement) in [
            ("/version/version", serde_json::json!(0)),
            ("/version/version", serde_json::json!(1.5)),
            ("/root_id", serde_json::json!("not-an-id")),
            ("/observations", serde_json::json!([])),
            (
                "/local_schema/node_types/0/parents",
                serde_json::json!(["Project"]),
            ),
            ("/facts/0/value/evidence", serde_json::json!([])),
            ("/facts/0/value/property", serde_json::json!("undeclared")),
            ("/facts/0/value/value/kind", serde_json::json!("Integer")),
            (
                "/evidence/0/evidence/confidence_bp",
                serde_json::json!(10001),
            ),
            ("/evidence/0/evidence/observed_at", serde_json::json!("17")),
            (
                "/evidence/0/payload",
                serde_json::json!(bytes::encode(b"substitution")),
            ),
        ] {
            let mut document = original.clone();
            *document.pointer_mut(pointer).unwrap() = replacement;
            let payload = bytes::encode(&serde_json::to_vec(&document).unwrap());
            let input =
                serde_json::from_value(serde_json::json!({"document":document,"payload":payload}));
            if pointer == "/evidence/0/evidence/observed_at" {
                assert!(
                    input.is_err(),
                    "generated timestamp admitted malformed input"
                );
            } else {
                assert!(
                    runtime
                        .import_interpretation(&input.unwrap(), Timestamp::EPOCH)
                        .is_err(),
                    "accepted {pointer}"
                );
            }
            assert_eq!(
                runtime.published_events().unwrap().len(),
                held,
                "mutated after {pointer}"
            );
        }
        assert!(runtime.interpretations().unwrap().is_empty());
        assert_eq!(runtime.observations().unwrap().len(), 1);
    }
}
