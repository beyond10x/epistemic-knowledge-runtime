//! Adversary pass 1 on `story:seed-envelope-decoded-once` (wave ingest-01, unit E).
//!
//! 1. The seed publication's admission replay must replay the *staged bytes*, not the envelope
//!    the seeding handle holds in memory (AGENTS.md invariant 1: "the store publishes one only
//!    after replaying the staged candidate through the injected kernel authority"). A seed whose
//!    envelope bytes do not decode must not be published: once it is, no later process can open
//!    the store.
//! 2. A checkpointed open must answer as a full replay does. A seed envelope whose payload values
//!    are not bytes is refused by a full replay with `seed-decode`; a checkpoint over it must not
//!    make a fresh open answer instead.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{RecordedOccurrence, RetainedHistory, RetainedObject, RevisionLog, StoreError};
use std::collections::BTreeSet;
use std::path::Path;

const TENANT: &str = "adversary-envelope";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
fn open_in_full(path: &Path, file: bool) -> Runtime {
    let mut runtime = open(path, file);
    runtime.set_full_replay(true);
    runtime
}
/// A seed with one node, one evidenced assertion about it, and that evidence's payload.
fn fixture() -> (SeedDocument, ContentHash, AssertionId) {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let many = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    let mut definition = PropertyDefinition::new(many, "labels", ValueType::String);
    definition.cardinality = Cardinality::Many;
    declared.properties.insert(many, definition);
    seed.ontology.node_types.push(declared);
    let node = Node::<Value>::new(NodeId::mint(), seed.graph.root.id, type_id, "seed");
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("seed".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let id = assertion.id;
    seed.graph.nodes.insert(node.id, node);
    seed.graph.assertions.insert(assertion.id, assertion);
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes.into());
    (seed, hash, id)
}

/// A seed document a Rust caller of the public API can build: `TemporalRange`'s fields are
/// public, so an inverted valid time is one struct literal away. Its JSON is refused by
/// `TemporalRange`'s own deserializer (`InvertedRange`), so the envelope bytes the seed writes do
/// not decode. Before this change the admission replay decoded exactly those bytes and refused
/// the publication; now it takes the envelope the seeding handle holds and publishes.
#[test]
fn a_seed_whose_envelope_bytes_do_not_decode_is_never_published() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let (mut seed, _, assertion) = fixture();
        seed.graph
            .assertions
            .get_mut(&assertion)
            .unwrap()
            .valid_time = TemporalRange {
            from: Some(Timestamp::from_millis(10)),
            to: Some(Timestamp::from_millis(5)),
        };
        // The bytes such a document serializes to are not a seed document.
        let yaml = serde_yaml_ng::to_string(&seed).unwrap();
        assert!(
            SeedDocument::from_yaml(&yaml).is_err(),
            "file={file}: an inverted range does not decode"
        );
        let seeding = open(path, file);
        let seeded = seeding.seed(seed, || Timestamp::from_millis(10));
        let fresh = open(path, file).snapshot();
        let full = open_in_full(path, file).snapshot();
        assert!(
            seeded.is_err() || (fresh.is_ok() && full.is_ok()),
            "file={file}: seed() published an envelope no later open admits: \
             seed={seeded:?} fresh-open={fresh:?} full-replay={full:?} \
             seeding-handle={:?}",
            seeding.snapshot().map(|graph| graph.revision)
        );
    }
}

// ---- forged checkpoint over an envelope whose payload values are not bytes ----

fn native(
    path: &Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = executor.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (executor, provider)
}
fn captured_history(path: &Path) -> RetainedHistory {
    let mut history = None;
    let _kernel = Commit::over_with_authority(context(), anchor(), |authority| {
        let store =
            ekr_store::SqliteStore::sqlite(&path.join("state.db"), TENANT, None)?.under(authority);
        history = Some(store.history()?);
        Ok(store)
    })
    .unwrap();
    history.unwrap()
}
/// Writes objects and revision occurrences straight through the native provider.
fn install_history(path: &Path, file: bool, history: &RetainedHistory) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId, TenantId};
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        let tenant = TenantId::new(TENANT).unwrap();
        let meta = |key: String| CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        for (hash, object) in &history.objects {
            provider
                .put_blob(&tenant, &hash.to_hex(), &object.bytes)
                .await
                .unwrap();
            let stream = StreamId::new(tenant.clone(), "ekr.store.object", hash.to_hex()).unwrap();
            let event = NewEvent::new(
                "ekr.store.ObjectStored",
                2,
                serde_json::to_value(&object.metadata).unwrap(),
            )
            .unwrap();
            provider
                .append(
                    &stream,
                    Expected::NoStream,
                    &[event],
                    &meta(format!("object-{hash}")),
                )
                .await
                .unwrap();
        }
        let stream = StreamId::new(tenant, "ekr.revision", "canonical").unwrap();
        let events = history
            .occurrences
            .iter()
            .map(|o| {
                NewEvent::new(o.event.name(), 2, serde_json::to_value(&o.event).unwrap()).unwrap()
            })
            .collect::<Vec<_>>();
        provider
            .append(
                &stream,
                Expected::NoStream,
                &events,
                &meta("history".into()),
            )
            .await
            .unwrap();
    });
}
/// The bytes of the checkpoint the newest pointer names, read from the provider itself.
fn newest_checkpoint(path: &Path, file: bool) -> serde_json::Value {
    let pointer = open(path, file)
        .published_events()
        .unwrap()
        .into_iter()
        .rfind(|event| event.name == "ekr.store.CheckpointWritten")
        .expect("the seed leaves a checkpoint")
        .data;
    let key = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let (executor, provider) = native(path, file);
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    let bytes = executor
        .block_on(provider.get_blob(&tenant, &key))
        .unwrap()
        .expect("the newest checkpoint is retained");
    serde_json::from_slice(&bytes).unwrap()
}
fn install_checkpoint(path: &Path, file: bool, covered: u64, bytes: &[u8]) {
    // The binding only feeds the head shortcut; the checkpoint itself is offered regardless.
    let binding = ContentHash::of_bytes(b"not this host's binding");
    if file {
        ekr_store::FileStore::file_existing(path, TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    } else {
        ekr_store::SqliteStore::sqlite_existing(&path.join("state.db"), TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    }
}
/// The replay prefix digest over `occurrences`, as `ekr.replay-prefix/1` defines it.
fn prefix(occurrences: &[RecordedOccurrence]) -> ContentHash {
    struct Step<'a>(ContentHash, &'a RecordedOccurrence);
    impl Canonical for Step<'_> {
        fn encode(&self, out: &mut Encoder) {
            "ekr.replay-prefix/1".encode(out);
            self.0.encode(out);
            self.1.version.encode(out);
            self.1.event.encode(out);
        }
    }
    let mut digest = ContentHash::of("ekr.replay-prefix/1");
    for occurrence in occurrences {
        digest = ContentHash::of(&Step(digest, occurrence));
    }
    digest
}
fn retained(bytes: Vec<u8>, like: &RetainedObject) -> (ContentHash, RetainedObject) {
    let hash = ContentHash::of_bytes(&bytes);
    let mut metadata = like.metadata.clone();
    metadata.content_hash = hash;
    metadata.byte_len = bytes.len() as u64;
    (
        hash,
        RetainedObject {
            metadata,
            bytes: bytes.into(),
        },
    )
}

/// A seed-only lineage whose envelope names its one evidence payload by the right address but
/// holds a string where the payload's bytes belong, with every record re-addressed to match, and
/// a checkpoint over it built exactly as the kernel builds one. A full replay refuses it with
/// `seed-decode`; before this change the checkpoint restore decoded the same envelope and refused
/// too, so every open answered `seed-decode`. The light view skips payload values, the restore
/// is admitted, and a fresh open answers the graph.
#[test]
fn a_checkpoint_over_an_envelope_with_non_byte_payloads_answers_as_a_full_replay() {
    let source = tempfile::tempdir().unwrap();
    let (seed, hash, _) = fixture();
    open(source.path(), false)
        .seed(seed, || Timestamp::from_millis(10))
        .unwrap();
    let genuine_history = captured_history(source.path());
    let genuine_checkpoint = newest_checkpoint(source.path(), false);
    assert_eq!(genuine_history.occurrences.len(), 1);
    assert_eq!(
        serde_json::to_value(prefix(&genuine_history.occurrences)).unwrap(),
        genuine_checkpoint["prefix"],
        "this file computes the replay prefix digest as the kernel does"
    );

    let first = &genuine_history.occurrences[0];
    let RevisionPayload::Seeded {
        revision_id,
        seed_hash,
    } = first.event.payload
    else {
        panic!("the first occurrence is the seed");
    };
    let old_envelope = &genuine_history.objects[&seed_hash];
    let old_record = &genuine_history.objects[&first.event.record_hash];

    // A new seed retains `ekr-seed-envelope/3`, which names its payloads and carries no value for
    // one; the forgery is the `/2` layout of the same envelope, which carries each payload's value.
    let mut envelope: serde_json::Value = serde_json::from_slice(&old_envelope.bytes).unwrap();
    let key = serde_json::to_value(hash).unwrap();
    assert_eq!(
        envelope["input"]["evidence_payloads"],
        serde_json::json!([key])
    );
    envelope["format"] = "ekr-seed-envelope/2".into();
    envelope["input"]["evidence_payloads"] = serde_json::json!({});
    envelope["input"]["evidence_payloads"][key.as_str().unwrap()] =
        serde_json::json!("these are not the payload's bytes");
    let (forged_hash, forged_envelope) =
        retained(serde_json::to_vec(&envelope).unwrap(), old_envelope);

    let mut record = SeedResultV1::from_bytes(&old_record.bytes).unwrap();
    record.seed_hash = forged_hash;
    record.result.transaction = forged_hash;
    record.result_hash = ContentHash::of(&record.result);
    let (record_hash, forged_record) = retained(record.to_bytes().unwrap(), old_record);

    let mut forged = genuine_history.clone();
    forged.objects.remove(&seed_hash);
    forged.objects.remove(&first.event.record_hash);
    forged.objects.insert(forged_hash, forged_envelope);
    forged.objects.insert(record_hash, forged_record);
    forged.occurrences[0].event.record_hash = record_hash;
    forged.occurrences[0].event.payload = RevisionPayload::Seeded {
        revision_id,
        seed_hash: forged_hash,
    };
    let mut checkpoint = genuine_checkpoint.clone();
    checkpoint["prefix"] = serde_json::to_value(prefix(&forged.occurrences)).unwrap();
    let checkpoint = serde_json::to_vec(&checkpoint).unwrap();

    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        install_history(path, file, &forged);
        install_checkpoint(path, file, 1, &checkpoint);
        let full = open_in_full(path, file).snapshot().map(|graph| graph.root);
        assert!(
            matches!(&full, Err(StoreError::InvalidSeed(reason)) if reason.starts_with("seed-decode")),
            "file={file}: a full replay refuses the envelope: {full:?}"
        );
        let checkpointed = open(path, file);
        let answered = checkpointed.snapshot().map(|graph| graph.root);
        assert_eq!(
            answered,
            full,
            "file={file}: a checkpointed open answers as a full replay does; the verified read \
             of the same runtime says {:?}",
            checkpointed.read(None).map(|read| read.root)
        );
    }
}
