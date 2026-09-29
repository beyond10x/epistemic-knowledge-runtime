//! Adversary pass 1 on `story:add-evidence-operation` (wave ingest-02, unit A).
//!
//! Each case drives the durable handlers through [`Runtime`] on both providers and holds what
//! the story's acceptance and `systems/ekr/domains/kernel.yaml`'s `EvidenceAdditionProjection`
//! say: evidence an `AddEvidence` brings commits with its payload stored once as a Provenance
//! object, a later or the same transaction may cite it, and every reopen — from a checkpoint or
//! with full replay — reaches the same root.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::BTreeSet;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn anchor_with(profile: ValidationProfileV1) -> AuthorityStateV1 {
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
        validation_profile: profile,
    }
}

fn anchor() -> AuthorityStateV1 {
    anchor_with(ValidationProfileV1::deterministic(context().validator))
}

fn open_with(path: &std::path::Path, file: bool, anchor: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), anchor)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor)
    }
    .unwrap()
}

fn open(path: &std::path::Path, file: bool) -> Runtime {
    open_with(path, file, anchor())
}

fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

fn id<T: std::str::FromStr>(n: u32) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012}").parse().unwrap()
}

struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
    seeded_evidence: EvidenceId,
    seeded_payload: Vec<u8>,
}

/// A seed with one node type, one node and one seeded evidence entry, all under fixed ids.
fn seeded() -> Seeded {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id: TypeId = id(5);
    let property: PropertyId = id(6);
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        property,
        PropertyDefinition::new(property, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    let node = Node::<Value>::new(id(7), document.graph.root.id, type_id, "subject");
    let bytes = b"seeded human statement".to_vec();
    let seeded_evidence = human_evidence(id(8), &bytes);
    document.graph.nodes.insert(node.id, node.clone());
    document
        .graph
        .evidence
        .insert(seeded_evidence.id, seeded_evidence.clone());
    document
        .evidence_payloads
        .insert(seeded_evidence.content_hash, bytes.clone().into());
    Seeded {
        document,
        node: node.id,
        property,
        seeded_evidence: seeded_evidence.id,
        seeded_payload: bytes,
    }
}

fn human_evidence(id: EvidenceId, payload: &[u8]) -> Evidence {
    Evidence {
        id,
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    }
}

fn add_evidence(evidence: Evidence, payload: &[u8]) -> GraphOperation {
    GraphOperation::AddEvidence(Box::new(EvidenceAddition {
        evidence,
        payload: payload.to_vec(),
    }))
}

fn assertion_with(seed: &Seeded, assertion: AssertionId, evidence: EvidenceId) -> Assertion<Value> {
    Assertion {
        id: assertion,
        root_id: seed.document.graph.root.id,
        subject: Subject::Node(seed.node),
        predicate: Predicate::Property(seed.property),
        object: Object::Value(Value::String(format!("claim {assertion}"))),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

fn assertion(seed: &Seeded, evidence: EvidenceId) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(assertion_with(
        seed,
        AssertionId::mint(),
        evidence,
    )))
}

fn transaction_with(id: TransactionId, operations: Vec<GraphOperation>) -> GraphTransaction {
    let evidence = operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    GraphTransaction {
        id,
        proposer: context().operator,
        operations,
        evidence,
        schema_version: None,
    }
}

fn transaction(operations: Vec<GraphOperation>) -> GraphTransaction {
    transaction_with(TransactionId::mint(), operations)
}

fn encode_as(tx: &GraphTransaction, format: &'static str) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format,
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    encode_as(tx, "ekr.transaction-document/2")
}

fn propose(runtime: &Runtime, tx: &GraphTransaction, now: i64) {
    runtime
        .propose(&encode(tx), context().operator, at(now))
        .unwrap();
}

fn validate(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> ValidationCommandResult {
    let head = runtime.head().unwrap().unwrap().revision;
    runtime.validate(tx.id, head, at(now)).unwrap()
}

fn verdict(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> ValidationCommandResult {
    propose(runtime, tx, now);
    validate(runtime, tx, now + 1)
}

fn codes(result: &ValidationCommandResult) -> Vec<String> {
    match result {
        ValidationCommandResult::Rejected(record) => {
            record.issues.iter().map(|i| i.code.clone()).collect()
        }
        ValidationCommandResult::Validated(_) => Vec::new(),
    }
}

fn committed(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> CommitReceiptV1 {
    let result = verdict(runtime, tx, now);
    assert!(
        matches!(result, ValidationCommandResult::Validated(_)),
        "not validated: {:?}",
        codes(&result)
    );
    match runtime
        .commit(tx.id, context().operator, at(now + 2))
        .unwrap()
    {
        CommitCommandResult::Committed(receipt) => *receipt,
        CommitCommandResult::Stale(stale) => panic!("stale: {stale:?}"),
    }
}

/// Reopens `directory` with full replay and with the checkpoint, and holds both to `head`, the
/// head graph `graph` and the retained bytes of every `(hash, payload)` in `payloads`.
fn reopens_to(
    directory: &std::path::Path,
    file: bool,
    anchor: &AuthorityStateV1,
    head: Root,
    graph: &CanonicalGraph,
    payloads: &[(ContentHash, Vec<u8>)],
) {
    for full in [true, false] {
        let mut reopened = open_with(directory, file, anchor.clone());
        reopened.set_full_replay(full);
        assert_eq!(
            reopened.head().unwrap().unwrap(),
            head,
            "file={file} full={full}"
        );
        assert_eq!(
            &reopened.snapshot().unwrap(),
            graph,
            "file={file} full={full}"
        );
        let read = reopened.read(None).unwrap();
        for (hash, payload) in payloads {
            assert_eq!(
                read.content(hash),
                Some(payload.as_slice()),
                "file={file} full={full}"
            );
        }
    }
}

/// The `ekr.store.ObjectStored` events the provider published for `hash`.
fn stored(runtime: &Runtime, hash: ContentHash) -> Vec<serde_json::Value> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.ObjectStored")
        .filter(|event| event.data["content_hash"] == hash.to_string())
        .map(|event| event.data)
        .collect()
}

#[test]
fn an_assertion_written_before_the_evidence_it_cites_commits_and_replays_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"cited before it is added".to_vec();
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let claim = assertion(&seed, evidence.id);
        let tx = transaction(vec![claim, add_evidence(evidence.clone(), &payload)]);
        committed(&runtime, &tx, 20);
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        assert_eq!(graph.evidence.get(&evidence.id), Some(&evidence));
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[(evidence.content_hash, payload)],
        );
    }
}

#[test]
fn one_payload_under_two_evidence_ids_in_one_transaction_commits_and_replays() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"said once, entered twice".to_vec();
        let first = human_evidence(EvidenceId::mint(), &payload);
        let second = human_evidence(EvidenceId::mint(), &payload);
        let tx = transaction(vec![
            add_evidence(first.clone(), &payload),
            add_evidence(second.clone(), &payload),
            assertion(&seed, first.id),
            assertion(&seed, second.id),
        ]);
        committed(&runtime, &tx, 20);
        assert_eq!(stored(&runtime, first.content_hash).len(), 1, "file={file}");
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        assert!(graph.evidence.contains_key(&first.id) && graph.evidence.contains_key(&second.id));
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[(first.content_hash, payload)],
        );
    }
}

#[test]
fn a_payload_already_retained_added_again_under_new_ids_commits_and_replays() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        // The seed's own payload, under a new evidence id.
        let again = human_evidence(EvidenceId::mint(), &seed.seeded_payload);
        committed(
            &runtime,
            &transaction(vec![
                add_evidence(again.clone(), &seed.seeded_payload),
                assertion(&seed, again.id),
            ]),
            20,
        );
        // A payload one commit added, added again by a later one.
        let payload = b"entered by two commits".to_vec();
        let first = human_evidence(EvidenceId::mint(), &payload);
        committed(
            &runtime,
            &transaction(vec![add_evidence(first.clone(), &payload)]),
            30,
        );
        let second = human_evidence(EvidenceId::mint(), &payload);
        committed(
            &runtime,
            &transaction(vec![
                add_evidence(second.clone(), &payload),
                assertion(&seed, second.id),
            ]),
            40,
        );
        assert_eq!(stored(&runtime, first.content_hash).len(), 1, "file={file}");
        assert_eq!(stored(&runtime, again.content_hash).len(), 1, "file={file}");
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[
                (first.content_hash, payload),
                (again.content_hash, seed.seeded_payload.clone()),
            ],
        );
    }
}

#[test]
fn evidence_a_rejected_transaction_carried_is_neither_stored_nor_citable() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"brought by a rejected transaction".to_vec();
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let rejected = transaction(vec![
            add_evidence(evidence.clone(), &payload),
            assertion(&seed, EvidenceId::mint()),
        ]);
        assert_eq!(
            codes(&verdict(&runtime, &rejected, 20)),
            vec!["unresolved-evidence".to_owned()]
        );
        assert_eq!(runtime.content(&evidence.content_hash).unwrap(), None);
        assert!(stored(&runtime, evidence.content_hash).is_empty());
        let citing = transaction(vec![assertion(&seed, evidence.id)]);
        assert_eq!(
            codes(&verdict(&runtime, &citing, 30)),
            vec!["unresolved-evidence".to_owned()],
            "file={file}"
        );
        // The same entry under the same id is still free: nothing retained it.
        committed(
            &runtime,
            &transaction(vec![
                add_evidence(evidence.clone(), &payload),
                assertion(&seed, evidence.id),
            ]),
            40,
        );
        assert_eq!(stored(&runtime, evidence.content_hash).len(), 1);
    }
}

#[test]
fn a_stale_add_evidence_stores_nothing_and_its_retry_stores_the_payload_once() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"validated, then the head moved".to_vec();
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let operations = vec![
            add_evidence(evidence.clone(), &payload),
            assertion(&seed, evidence.id),
        ];
        let stale = transaction(operations.clone());
        assert!(matches!(
            verdict(&runtime, &stale, 20),
            ValidationCommandResult::Validated(_)
        ));
        // Another transaction moves the head past the basis `stale` was validated at.
        committed(
            &runtime,
            &transaction(vec![assertion(&seed, seed.seeded_evidence)]),
            30,
        );
        let result = runtime
            .commit(stale.id, context().operator, at(40))
            .unwrap();
        assert!(
            matches!(result, CommitCommandResult::Stale(_)),
            "{result:?}"
        );
        assert_eq!(runtime.content(&evidence.content_hash).unwrap(), None);
        assert!(stored(&runtime, evidence.content_hash).is_empty());
        assert!(!runtime
            .snapshot()
            .unwrap()
            .evidence
            .contains_key(&evidence.id));
        assert_eq!(
            codes(&verdict(
                &runtime,
                &transaction(vec![assertion(&seed, evidence.id)]),
                50
            )),
            vec!["unresolved-evidence".to_owned()]
        );
        // A retry of the same operations under a new transaction id commits once.
        let retry = transaction(operations);
        committed(&runtime, &retry, 60);
        assert_eq!(stored(&runtime, evidence.content_hash).len(), 1);
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[(evidence.content_hash, payload)],
        );
    }
}

#[test]
fn two_transactions_adding_one_evidence_id_at_one_basis_commit_once() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let evidence_id = EvidenceId::mint();
        let a = transaction(vec![add_evidence(
            human_evidence(evidence_id, b"version a"),
            b"version a",
        )]);
        let b = transaction(vec![add_evidence(
            human_evidence(evidence_id, b"version b"),
            b"version b",
        )]);
        assert!(matches!(
            verdict(&runtime, &a, 20),
            ValidationCommandResult::Validated(_)
        ));
        assert!(matches!(
            verdict(&runtime, &b, 30),
            ValidationCommandResult::Validated(_)
        ));
        assert!(matches!(
            runtime.commit(a.id, context().operator, at(40)).unwrap(),
            CommitCommandResult::Committed(_)
        ));
        assert!(matches!(
            runtime.commit(b.id, context().operator, at(50)).unwrap(),
            CommitCommandResult::Stale(_)
        ));
        let graph = runtime.snapshot().unwrap();
        assert_eq!(
            graph.evidence[&evidence_id].content_hash,
            ContentHash::of_bytes(b"version a")
        );
        assert_eq!(
            runtime
                .content(&ContentHash::of_bytes(b"version b"))
                .unwrap(),
            None
        );
    }
}

/// A checkpoint is written at the seed and again four revisions past the retained one
/// (`REPLAY_CHECKPOINT_COMMITS`). Revision 4 adds evidence, so the checkpoint written there covers
/// it; revisions 5 and 6 add evidence after it. A reopen continues from that checkpoint — no seed
/// replay — and reaches the root full replay reaches.
#[test]
fn a_checkpoint_covering_added_evidence_is_restored_and_agrees_with_full_replay() {
    let validator = context().validator;
    for (anchor, file) in [
        ValidationProfileV1::deterministic(validator),
        ValidationProfileV1::identity_keeping(validator),
    ]
    .into_iter()
    .flat_map(|profile| {
        [
            (anchor_with(profile.clone()), false),
            (anchor_with(profile), true),
        ]
    }) {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open_with(directory.path(), file, anchor.clone());
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let mut payloads = Vec::new();
        for n in 1..=6_i64 {
            let payload = format!("statement {n}").into_bytes();
            let evidence = human_evidence(EvidenceId::mint(), &payload);
            let operations = if n <= 3 {
                vec![assertion(&seed, seed.seeded_evidence)]
            } else {
                payloads.push((evidence.content_hash, payload.clone()));
                vec![
                    add_evidence(evidence.clone(), &payload),
                    assertion(&seed, evidence.id),
                ]
            };
            committed(&runtime, &transaction(operations), 10 * (n + 1));
        }
        let head = runtime.head().unwrap().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(6));
        let graph = runtime.snapshot().unwrap();
        drop(runtime);

        let reopened = open_with(directory.path(), file, anchor.clone());
        assert_eq!(reopened.head().unwrap().unwrap(), head);
        assert_eq!(reopened.snapshot().unwrap(), graph);
        let read = reopened.read(None).unwrap();
        for (hash, payload) in &payloads {
            assert_eq!(read.content(hash), Some(payload.as_slice()));
        }
        drop(read);
        assert_eq!(
            reopened.seed_replays(),
            0,
            "file={file}: the checkpoint covering the AddEvidence commit was not restored"
        );
        drop(reopened);
        reopens_to(directory.path(), file, &anchor, head, &graph, &payloads);
    }
}

#[test]
fn a_zero_byte_payload_commits_and_replays_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let evidence = human_evidence(EvidenceId::mint(), b"");
        committed(
            &runtime,
            &transaction(vec![
                add_evidence(evidence.clone(), b""),
                assertion(&seed, evidence.id),
            ]),
            20,
        );
        assert_eq!(
            runtime.content(&evidence.content_hash).unwrap(),
            Some(Vec::new()),
            "file={file}"
        );
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[(evidence.content_hash, Vec::new())],
        );
    }
}

/// A payload is a YAML sequence of byte values, so the largest one a document carries is its
/// version's `sequence_elements` bound, not its byte cap: 16,384 bytes in `/2` and 4,096 in
/// `/1`. The largest commits and replays; one byte more is refused at propose by that name.
#[test]
fn the_largest_payload_each_document_version_carries_commits_and_one_byte_more_is_refused() {
    for (format, largest) in [
        ("ekr.transaction-document/2", 16_384_usize),
        ("ekr.transaction-document/1", 4_096),
    ] {
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = seeded();
            let runtime = open(directory.path(), file);
            runtime.seed(seed.document.clone(), at(10)).unwrap();
            let payload: Vec<u8> = (0..largest).map(|i| (i % 256) as u8).collect();
            let evidence = human_evidence(EvidenceId::mint(), &payload);
            let tx = transaction(vec![
                add_evidence(evidence.clone(), &payload),
                assertion(&seed, evidence.id),
            ]);
            runtime
                .propose(&encode_as(&tx, format), context().operator, at(20))
                .unwrap();
            assert!(matches!(
                validate(&runtime, &tx, 21),
                ValidationCommandResult::Validated(_)
            ));
            assert!(matches!(
                runtime.commit(tx.id, context().operator, at(22)).unwrap(),
                CommitCommandResult::Committed(_)
            ));

            let over: Vec<u8> = (0..=largest).map(|i| (i % 256) as u8).collect();
            let evidence_over = human_evidence(EvidenceId::mint(), &over);
            let too_large = transaction(vec![add_evidence(evidence_over, &over)]);
            let refused = runtime
                .propose(&encode_as(&too_large, format), context().operator, at(30))
                .unwrap_err()
                .to_string();
            assert!(
                refused.contains("sequence_elements"),
                "{format} file={file}: {refused}"
            );

            let head = runtime.head().unwrap().unwrap();
            let graph = runtime.snapshot().unwrap();
            drop(runtime);
            reopens_to(
                directory.path(),
                file,
                &anchor(),
                head,
                &graph,
                &[(evidence.content_hash, payload)],
            );
        }
    }
}

#[test]
fn add_evidence_commits_and_replays_under_each_validation_profile() {
    let validator = context().validator;
    for profile in [
        ValidationProfileV1::deterministic(validator),
        ValidationProfileV1::schema_evolving(validator),
        ValidationProfileV1::identity_keeping(validator),
    ] {
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = seeded();
            let anchor = anchor_with(profile.clone());
            let runtime = open_with(directory.path(), file, anchor.clone());
            runtime.seed(seed.document.clone(), at(10)).unwrap();
            let payload = b"one profile".to_vec();
            let evidence = human_evidence(EvidenceId::mint(), &payload);
            committed(
                &runtime,
                &transaction(vec![
                    add_evidence(evidence.clone(), &payload),
                    assertion(&seed, evidence.id),
                ]),
                20,
            );
            // Reused under the profile: refused by name.
            assert_eq!(
                codes(&verdict(
                    &runtime,
                    &transaction(vec![add_evidence(evidence.clone(), &payload)]),
                    30
                )),
                vec!["identity-already-exists".to_owned()],
                "{profile:?}"
            );
            let head = runtime.head().unwrap().unwrap();
            let graph = runtime.snapshot().unwrap();
            drop(runtime);
            reopens_to(
                directory.path(),
                file,
                &anchor,
                head,
                &graph,
                &[(evidence.content_hash, payload)],
            );
        }
    }
}

#[test]
fn file_and_sqlite_reach_equal_roots_for_one_add_evidence_history() {
    let mut roots = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"the same on both providers".to_vec();
        let evidence = human_evidence(id(900), &payload);
        committed(
            &runtime,
            &transaction_with(
                id(901),
                vec![
                    add_evidence(evidence.clone(), &payload),
                    GraphOperation::AddAssertion(Box::new(assertion_with(
                        &seed,
                        id(902),
                        evidence.id,
                    ))),
                ],
            ),
            20,
        );
        committed(
            &runtime,
            &transaction_with(
                id(903),
                vec![GraphOperation::AddAssertion(Box::new(assertion_with(
                    &seed,
                    id(904),
                    evidence.id,
                )))],
            ),
            30,
        );
        roots.push((
            runtime.head().unwrap().unwrap(),
            runtime.snapshot().unwrap(),
        ));
    }
    assert_eq!(roots[0].0, roots[1].0);
    assert_eq!(roots[0].1, roots[1].1);
}

/// The payload's bytes equal an object the store already retains at a stronger class: the seed
/// envelope, Canonical. The evidence entry validates, so its commit publishes, and the envelope
/// stays readable at Canonical.
#[test]
fn a_payload_equal_to_a_canonical_object_the_store_retains_commits_and_replays() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let canonical = runtime
            .published_events()
            .unwrap()
            .into_iter()
            .filter(|event| event.name == "ekr.store.ObjectStored")
            .find(|event| event.data["storage_class"] == "Canonical")
            .expect("the seed publishes a Canonical object");
        let hash: ContentHash = canonical.data["content_hash"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let payload = runtime.content(&hash).unwrap().unwrap();
        // Within the `/2` sequence bound, so the document itself is admissible.
        assert!(payload.len() <= 16_384, "{}", payload.len());
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let tx = transaction(vec![
            add_evidence(evidence.clone(), &payload),
            assertion(&seed, evidence.id),
        ]);
        assert!(matches!(
            verdict(&runtime, &tx, 20),
            ValidationCommandResult::Validated(_)
        ));
        let result = runtime.commit(tx.id, context().operator, at(22));
        assert!(
            matches!(result, Ok(CommitCommandResult::Committed(_))),
            "file={file}: a validated AddEvidence did not commit: {result:?}"
        );
        let head = runtime.head().unwrap().unwrap();
        let graph = runtime.snapshot().unwrap();
        drop(runtime);
        reopens_to(
            directory.path(),
            file,
            &anchor(),
            head,
            &graph,
            &[(evidence.content_hash, payload)],
        );
    }
}

/// Opens `directory` three times each way, alternating, and answers the fastest open-and-head
/// with the checkpoint, the fastest with full replay, and the time one handle takes to read the
/// retained bytes of every evidence entry through [`Runtime::content`], as `ekr_views::load`
/// does once per entry.
fn measured(
    directory: &std::path::Path,
    file: bool,
    head: Root,
) -> (
    std::time::Duration,
    std::time::Duration,
    std::time::Duration,
) {
    let mut best = [std::time::Duration::MAX; 2];
    for _ in 0..3 {
        for (slot, full) in [false, true].into_iter().enumerate() {
            let started = std::time::Instant::now();
            let mut reopened = open(directory, file);
            reopened.set_full_replay(full);
            assert_eq!(reopened.head().unwrap().unwrap(), head);
            best[slot] = best[slot].min(started.elapsed());
        }
    }
    let runtime = open(directory, file);
    let hashes: Vec<ContentHash> = runtime
        .snapshot()
        .unwrap()
        .evidence
        .values()
        .map(|evidence| evidence.content_hash)
        .collect();
    let started = std::time::Instant::now();
    for hash in &hashes {
        assert!(runtime.content(hash).unwrap().is_some());
    }
    (best[0], best[1], started.elapsed())
}

/// Not a correctness case: the cost of opening a store, and of reading every evidence entry's
/// bytes, against the number of evidence entries — seeded, added by one commit each, or absent
/// (commits citing seeded evidence). Run with `--ignored --nocapture`; `EKR_ADV_SIZES` and
/// `EKR_ADV_PROVIDERS` (`file` or `sqlite`) choose what is built, `EKR_ADV_KINDS` which of
/// `seeded,added,cited`.
#[test]
#[ignore = "measurement: run explicitly"]
fn cold_open_time_by_number_of_add_evidence_commits() {
    let sizes: Vec<usize> = std::env::var("EKR_ADV_SIZES")
        .unwrap_or_else(|_| "1,100".into())
        .split(',')
        .map(|n| n.trim().parse().unwrap())
        .collect();
    let providers: Vec<bool> = match std::env::var("EKR_ADV_PROVIDERS").as_deref() {
        Ok("file") => vec![true],
        Ok("sqlite") => vec![false],
        _ => vec![false, true],
    };
    let kinds = std::env::var("EKR_ADV_KINDS").unwrap_or_else(|_| "seeded,added,cited".into());
    for file in providers {
        let provider = if file { "file" } else { "sqlite" };
        for &n in &sizes {
            for kind in kinds.split(',') {
                let directory = tempfile::tempdir().unwrap();
                let mut seed = seeded();
                if kind == "seeded" {
                    for i in 0..n {
                        let payload = format!("seeded statement {i}").into_bytes();
                        let evidence = human_evidence(EvidenceId::mint(), &payload);
                        seed.document
                            .graph
                            .evidence
                            .insert(evidence.id, evidence.clone());
                        seed.document
                            .evidence_payloads
                            .insert(evidence.content_hash, payload.into());
                    }
                }
                let runtime = open(directory.path(), file);
                runtime.seed(seed.document.clone(), at(10)).unwrap();
                let build = std::time::Instant::now();
                if kind != "seeded" {
                    for i in 0..n {
                        let payload = format!("statement {i}").into_bytes();
                        let evidence = human_evidence(EvidenceId::mint(), &payload);
                        let operations = if kind == "added" {
                            vec![
                                add_evidence(evidence.clone(), &payload),
                                assertion(&seed, evidence.id),
                            ]
                        } else {
                            vec![assertion(&seed, seed.seeded_evidence)]
                        };
                        committed(&runtime, &transaction(operations), 100 + 10 * i as i64);
                    }
                }
                let built = build.elapsed();
                let head = runtime.head().unwrap().unwrap();
                drop(runtime);
                let (checkpoint, full, contents) = measured(directory.path(), file, head);
                println!(
                    "provider={provider} kind={kind} n={n} build={built:.2?} \
                     open_checkpoint={checkpoint:.2?} open_full_replay={full:.2?} \
                     content_of_every_evidence={contents:.2?} load={}",
                    std::fs::read_to_string("/proc/loadavg")
                        .unwrap_or_default()
                        .split(' ')
                        .next()
                        .unwrap_or_default()
                );
            }
        }
    }
}
