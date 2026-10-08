//! `story:evidence-attaches-to-a-held-assertion`: evidence attaches to an assertion the store
//! already holds, on both providers, under every validation profile.
//!
//! A consumer's store holds assertions that cite a whole source file, because they came from a run
//! before per-message evidence existed; it can now name each assertion's exact message. Before
//! `!AttachEvidence {assertion, evidence}` the only route was a supersession of a fact that had not
//! changed. The attachment is its own record — which assertion, which evidence, which revision —
//! and leaves the assertion exactly as it was; `ekr explain` lists it from the revision that made
//! it on. The refusals are held, message for message, by `tests/validation.rs`.

use std::collections::BTreeSet;
use std::time::Instant;

use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId,
};
use ekr_graph::{
    AttachedEvidence, CanonicalRef, CanonicalValue, Confidence, Evidence, EvidenceSource, Root,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EvidenceAddition,
    EvidenceAttachment, ExplanationLink, GraphOperation, GraphTransaction, Retraction, Runtime,
    SeedDocument, Supersession, ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

/// The seed: Alice and Bob (Person), Acme (Organization), `CEO_OF`, and two evidence entries.
const SEED: &str = include_str!("fixtures/attachment-base-seed.yaml");
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const BOB: &str = "00000000-0000-4000-8000-000000000302";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
const CEO_OF: &str = "00000000-0000-4000-8000-000000000203";
/// The seed's evidence: the whole source a first run cited.
const SOURCE: &str = "00000000-0000-4000-8000-000000000401";
/// The seed's second evidence entry.
const OTHER: &str = "00000000-0000-4000-8000-000000000402";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
    }
}

fn anchor(profile: ValidationProfileV1) -> AuthorityStateV1 {
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

/// The three validation profiles: an attachment is data, not a schema change, so each applies it.
fn profiles() -> [(&'static str, AuthorityStateV1); 3] {
    let validator = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(validator))),
        (
            "v2",
            anchor(ValidationProfileV1::schema_evolving(validator)),
        ),
        (
            "v3",
            anchor(ValidationProfileV1::identity_keeping(validator)),
        ),
    ]
}

fn open(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
    .unwrap()
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

/// A transaction of `operations` declaring `evidence`.
fn transaction(operations: Vec<GraphOperation>, evidence: &[EvidenceId]) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: evidence.iter().copied().collect(),
        schema_version: None,
    }
}

/// An assertion that `subject` is CEO of Acme from `from`, citing `evidence`.
fn claim(subject: &str, from: i64, evidence: EvidenceId) -> (AssertionId, GraphOperation) {
    let assertion = AssertionId::mint();
    let yaml = format!(
        "id: {assertion}
root_id: {ROOT}
subject: !Node {subject}
predicate: !Relation {CEO_OF}
object: !Node {ACME}
evidence:
- {evidence}
proposed_by: {}
assessment: Proposed
lifecycle: Active
valid_time:
  from: {from}
  to: null
transaction_time:
  recorded_from: 0
  recorded_to: null
",
        context().operator
    );
    (
        assertion,
        GraphOperation::AddAssertion(Box::new(serde_yaml_ng::from_str(&yaml).unwrap())),
    )
}

/// One evidence entry for `text`, under a fresh id, attributed to the operator.
fn message(text: &str) -> (EvidenceId, GraphOperation) {
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("a consumer's message".into()),
        },
        content_hash: ContentHash::of_bytes(text.as_bytes()),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_773_273_600_000),
        confidence: Confidence::CERTAIN,
    };
    let id = evidence.id;
    (
        id,
        GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence,
            payload: text.as_bytes().to_vec(),
        })),
    )
}

fn attach(assertion: AssertionId, evidence: EvidenceId) -> GraphOperation {
    GraphOperation::AttachEvidence(EvidenceAttachment {
        assertion,
        evidence,
    })
}

/// Propose, validate against the head, and — if validated — commit, at `at`, `at + 1`, `at + 2`.
fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    if matches!(verdict, ValidationCommandResult::Validated(_)) {
        let committed = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
    }
    verdict
}

fn committed(kernel: &Runtime, tx: &GraphTransaction, at: i64) {
    let verdict = submit(kernel, tx, at);
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
}

/// Each issue of a rejection as `(validator, code)`.
fn refused(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

/// The evidence ids of an explanation's Evidence links, and its Attachment links as
/// `(assertion, evidence, revision)`, each in chain order.
fn explained(
    kernel: &Runtime,
    at: Option<RevisionNumber>,
    assertion: AssertionId,
) -> (
    Vec<EvidenceId>,
    Vec<(AssertionId, EvidenceId, RevisionNumber)>,
) {
    let explanation = kernel.read(at).unwrap().explain(assertion).unwrap();
    let mut evidence = Vec::new();
    let mut attachments = Vec::new();
    for link in explanation.links {
        match link {
            ExplanationLink::Evidence(entry) => evidence.push(entry.id),
            ExplanationLink::Attachment(attached) => {
                assert_eq!(attached.commit.result.revision, attached.revision);
                attachments.push((
                    attached.assertion_id,
                    attached.evidence_id,
                    attached.revision,
                ));
            }
            _ => {}
        }
    }
    (evidence, attachments)
}

/// The canonical bytes of an assertion: its claim fields, lifecycle, evidence and times.
fn bytes(assertion: &ekr_graph::Assertion<CanonicalValue>) -> ContentHash {
    ContentHash::of(assertion)
}

/// Every revision's knowledge root, as the replay of each revision computes it.
fn roots(kernel: &Runtime) -> Vec<Root> {
    let head = kernel.head().unwrap().unwrap();
    (0..=head.revision.get())
        .map(|number| kernel.read(Some(RevisionNumber::new(number))).unwrap().root)
        .collect()
}

/// Acceptance 1: a transaction of `AddEvidence` plus `AttachEvidence` commits on both providers
/// and under every profile; the assertion is byte-identical before and after; `explain` at the new
/// revision lists both evidence entries and the attachment, and at the revision before lists only
/// the original; the knowledge root moves; and a reopened store replays every root.
#[test]
fn evidence_added_and_attached_in_one_transaction_commits_and_explain_lists_it_from_then_on() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
                    Timestamp::from_millis(10)
                })
                .unwrap();
            let (held, add) = claim(ALICE, 1_577_836_800_000, id(SOURCE));
            committed(&kernel, &transaction(vec![add], &[id(SOURCE)]), 20);
            let before = kernel.snapshot().unwrap();
            assert_eq!(before.revision, RevisionNumber::new(1), "{at}");

            let (exact, entry) = message("Alice is CEO of Acme.");
            committed(
                &kernel,
                &transaction(vec![entry, attach(held, exact)], &[exact]),
                30,
            );
            let after = kernel.snapshot().unwrap();
            assert_eq!(after.revision, RevisionNumber::new(2), "{at}");
            assert_eq!(
                bytes(&after.assertions[&held]),
                bytes(&before.assertions[&held]),
                "{at}: the assertion is not edited"
            );
            assert_eq!(after.assertions[&held], before.assertions[&held], "{at}");
            assert_eq!(
                after.attachments[&held],
                BTreeSet::from([AttachedEvidence {
                    evidence: CanonicalRef::new(exact),
                    revision: RevisionNumber::new(2),
                }]),
                "{at}"
            );
            assert!(after.evidence.contains_key(&exact), "{at}");

            let (evidence, attachments) = explained(&kernel, None, held);
            assert_eq!(
                evidence.iter().copied().collect::<BTreeSet<_>>(),
                BTreeSet::from([id(SOURCE), exact]),
                "{at}: the new revision explains both entries"
            );
            assert_eq!(attachments, [(held, exact, RevisionNumber::new(2))], "{at}");
            let (evidence, attachments) = explained(&kernel, Some(RevisionNumber::new(1)), held);
            assert_eq!(
                evidence,
                [id(SOURCE)],
                "{at}: the revision before lists the original"
            );
            assert!(attachments.is_empty(), "{at}");

            let recorded = roots(&kernel);
            assert_ne!(
                recorded[1].knowledge_root, recorded[2].knowledge_root,
                "{at}: the attachment moves the knowledge root"
            );
            assert_ne!(
                recorded[1].evidence_root, recorded[2].evidence_root,
                "{at}: the added entry moves the evidence root"
            );
            let records = kernel.transactions().unwrap();
            drop(kernel);

            let reopened = open(directory.path(), file, authority.clone());
            assert_eq!(roots(&reopened), recorded, "{at}");
            assert_eq!(reopened.transactions().unwrap(), records, "{at}");
            assert_eq!(reopened.snapshot().unwrap(), after, "{at}");
            assert_eq!(
                explained(&reopened, None, held).1,
                [(held, exact, RevisionNumber::new(2))],
                "{at}"
            );

            let migrated_directory = tempfile::tempdir().unwrap();
            let migrated = open(migrated_directory.path(), file, authority.clone());
            reopened.migrate_into(&migrated).unwrap();
            assert_eq!(migrated.snapshot().unwrap(), after, "{at}: migrated graph");
            for revision in [RevisionNumber::new(1), RevisionNumber::new(2)] {
                assert_eq!(
                    explained(&migrated, Some(revision), held),
                    explained(&reopened, Some(revision), held),
                    "{at}: migrated historical explanation {revision}"
                );
            }
            for (before, after) in recorded.iter().zip(roots(&migrated)) {
                assert_eq!(before.knowledge_root, after.knowledge_root, "{at}");
                assert_eq!(before.evidence_root, after.evidence_root, "{at}");
            }
            drop(migrated);
            let mut migrated = open(migrated_directory.path(), file, authority.clone());
            migrated.set_full_replay(true);
            assert_eq!(migrated.snapshot().unwrap(), after, "{at}: migrated replay");
        }
    }
}

/// Acceptance 2, through the store: each refusal is a named code, none moves the head, and an
/// evidence entry the seed retains attaches without an `AddEvidence`.
#[test]
fn each_attachment_refusal_is_named_and_moves_nothing() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
                    Timestamp::from_millis(10)
                })
                .unwrap();
            let (active, add_active) = claim(ALICE, 1_577_836_800_000, id(SOURCE));
            let (retracted, add_retracted) = claim(BOB, 1_577_836_800_000, id(SOURCE));
            let (superseded, add_superseded) = claim(BOB, 1_600_000_000_000, id(OTHER));
            let (replacement, add_replacement) = claim(BOB, 1_700_000_000_000, id(OTHER));
            committed(
                &kernel,
                &transaction(
                    vec![add_active, add_retracted, add_superseded],
                    &[id(SOURCE), id(OTHER)],
                ),
                20,
            );
            committed(
                &kernel,
                &transaction(
                    vec![
                        GraphOperation::RetractAssertion(Retraction {
                            assertion: retracted,
                            reason: ekr_graph::RetractionReason::new("withdrawn"),
                        }),
                        add_replacement,
                        GraphOperation::SupersedeAssertion(Supersession {
                            assertion: superseded,
                            by: replacement,
                            effective_from: Timestamp::from_millis(1_700_000_000_000),
                        }),
                    ],
                    &[id(OTHER)],
                ),
                30,
            );
            // An entry the seed retains attaches with no AddEvidence.
            committed(
                &kernel,
                &transaction(vec![attach(active, id(OTHER))], &[id(OTHER)]),
                40,
            );
            let head = kernel.head().unwrap().unwrap();
            assert_eq!(head.revision, RevisionNumber::new(3), "{at}");

            let (exact, entry) = message("Bob was CEO of Acme.");
            let (fresh, add_fresh) = claim(ALICE, 1_577_836_800_000, id(SOURCE));
            let (later, add_later) = claim(ALICE, 1_800_000_000_000, id(SOURCE));
            let structural = |code: &str| vec![(ValidatorName::Structural, code.to_owned())];
            let reference = |code: &str| vec![(ValidatorName::Reference, code.to_owned())];
            type RefusalCase = (&'static str, GraphTransaction, Vec<(ValidatorName, String)>);
            let cases: Vec<RefusalCase> = vec![
                (
                    "an assertion no revision holds",
                    transaction(vec![attach(AssertionId::mint(), id(OTHER))], &[id(OTHER)]),
                    reference("unresolved-assertion"),
                ),
                (
                    "an assertion the same transaction adds",
                    transaction(
                        vec![add_fresh, attach(fresh, id(OTHER))],
                        &[id(SOURCE), id(OTHER)],
                    ),
                    reference("unresolved-assertion"),
                ),
                (
                    "evidence no revision retains and no AddEvidence adds",
                    transaction(vec![attach(active, exact)], &[exact]),
                    reference("unresolved-evidence"),
                ),
                (
                    "a retracted assertion",
                    transaction(vec![entry.clone(), attach(retracted, exact)], &[exact]),
                    structural("assertion-not-active"),
                ),
                (
                    "a superseded assertion",
                    transaction(vec![entry.clone(), attach(superseded, exact)], &[exact]),
                    structural("assertion-not-active"),
                ),
                (
                    "an assertion the same transaction retracts",
                    transaction(
                        vec![
                            entry.clone(),
                            attach(replacement, exact),
                            GraphOperation::RetractAssertion(Retraction {
                                assertion: replacement,
                                reason: ekr_graph::RetractionReason::new("withdrawn"),
                            }),
                        ],
                        &[exact],
                    ),
                    structural("assertion-not-active"),
                ),
                (
                    "an assertion the same transaction supersedes",
                    transaction(
                        vec![
                            entry.clone(),
                            add_later,
                            attach(active, exact),
                            GraphOperation::SupersedeAssertion(Supersession {
                                assertion: active,
                                by: later,
                                effective_from: Timestamp::from_millis(1_800_000_000_000),
                            }),
                        ],
                        &[id(SOURCE), exact],
                    ),
                    structural("assertion-not-active"),
                ),
                (
                    "evidence the assertion already cites",
                    transaction(vec![attach(active, id(SOURCE))], &[id(SOURCE)]),
                    structural("evidence-already-attached"),
                ),
                (
                    "evidence already attached to the assertion",
                    transaction(vec![attach(active, id(OTHER))], &[id(OTHER)]),
                    structural("evidence-already-attached"),
                ),
                (
                    "one attachment twice in a transaction",
                    transaction(
                        vec![entry.clone(), attach(active, exact), attach(active, exact)],
                        &[exact],
                    ),
                    structural("evidence-already-attached"),
                ),
                (
                    "an attachment the evidence set leaves out",
                    transaction(vec![entry.clone(), attach(active, exact)], &[]),
                    structural("evidence-set-mismatch"),
                ),
            ];
            for (offset, (what, tx, expected)) in cases.into_iter().enumerate() {
                let verdict = submit(&kernel, &tx, 100 + 10 * i64::try_from(offset).unwrap());
                assert_eq!(refused(&verdict), expected, "{at}: {what}");
            }
            assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
        }
    }
}

/// Acceptance 3: a supersession does not carry attachments to the replacement. After it, the
/// replacement's explanation lists none of the original's attachments, the original's at the
/// revision before the supersession lists them, and the original keeps them at the head.
#[test]
fn a_supersession_leaves_attachments_with_the_superseded_assertion() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (_, authority) = profiles()[2].clone();
        let kernel = open(directory.path(), file, authority);
        kernel
            .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
                Timestamp::from_millis(10)
            })
            .unwrap();
        let (original, add) = claim(ALICE, 1_577_836_800_000, id(SOURCE));
        committed(&kernel, &transaction(vec![add], &[id(SOURCE)]), 20);
        let (exact, entry) = message("Alice is CEO of Acme, says the minutes.");
        committed(
            &kernel,
            &transaction(vec![entry, attach(original, exact)], &[exact]),
            30,
        );
        let (replacement, add_replacement) = claim(ALICE, 1_800_000_000_000, id(OTHER));
        committed(
            &kernel,
            &transaction(
                vec![
                    add_replacement,
                    GraphOperation::SupersedeAssertion(Supersession {
                        assertion: original,
                        by: replacement,
                        effective_from: Timestamp::from_millis(1_800_000_000_000),
                    }),
                ],
                &[id(OTHER)],
            ),
            40,
        );
        let head = kernel.snapshot().unwrap();
        assert!(!head.attachments.contains_key(&replacement));
        assert_eq!(head.attachments[&original].len(), 1);

        let (evidence, attachments) = explained(&kernel, None, replacement);
        assert!(attachments.is_empty(), "{attachments:?}");
        assert!(!evidence.contains(&exact), "{evidence:?}");

        let (evidence, attachments) = explained(&kernel, Some(RevisionNumber::new(2)), original);
        assert_eq!(attachments, [(original, exact, RevisionNumber::new(2))]);
        assert!(evidence.contains(&exact));

        let (_, attachments) = explained(&kernel, None, original);
        assert_eq!(attachments, [(original, exact, RevisionNumber::new(2))]);
    }
}

/// Acceptance 4, measured and not bounded: 1,000 attachments in one transaction against a store
/// of 70,000 assertions commit, and the time each step took is printed. No bound is asserted
/// before `story:commit-cost-flat-with-store-size` lands. Run by hand:
/// `cargo test -p ekr-kernel --test attach_evidence -- --ignored --nocapture`.
#[test]
#[ignore = "builds a 70,000-assertion store; run by hand to record the time"]
fn a_thousand_attachments_against_seventy_thousand_assertions_commit() {
    const ASSERTIONS: usize = 70_000;
    const ATTACHMENTS: usize = 1_000;
    let mut seed = SeedDocument::from_yaml(SEED).unwrap();
    let root = seed.graph.root.id;
    let subject = |n: usize| -> NodeId {
        if n.is_multiple_of(2) {
            id(ALICE)
        } else {
            id(BOB)
        }
    };
    let mut held = Vec::with_capacity(ASSERTIONS);
    for n in 0..ASSERTIONS {
        let (assertion, operation) = claim(
            &subject(n).to_string(),
            1_577_836_800_000 + i64::try_from(n).unwrap(),
            id(SOURCE),
        );
        let GraphOperation::AddAssertion(mut proposed) = operation else {
            unreachable!()
        };
        proposed.root_id = root;
        proposed.proposed_by = context().operator;
        let proposed: ekr_graph::Assertion<Value> = *proposed;
        seed.graph.assertions.insert(assertion, proposed);
        held.push(assertion);
    }
    for file in [false, true] {
        let provider = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let (_, authority) = profiles()[2].clone();
        let kernel = open(directory.path(), file, authority);
        let started = Instant::now();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        println!(
            "{provider}: seeded {ASSERTIONS} assertions in {:?}",
            started.elapsed()
        );

        let mut operations = Vec::with_capacity(2 * ATTACHMENTS);
        let mut evidence = Vec::with_capacity(ATTACHMENTS);
        for (n, assertion) in held.iter().take(ATTACHMENTS).enumerate() {
            let (exact, entry) = message(&format!("message {n}"));
            operations.push(entry);
            operations.push(attach(*assertion, exact));
            evidence.push(exact);
        }
        let tx = transaction(operations, &evidence);
        let bytes = encode(&tx);
        let started = Instant::now();
        kernel
            .propose(&bytes, context().operator, || Timestamp::from_millis(20))
            .unwrap();
        let proposed = started.elapsed();
        let started = Instant::now();
        let verdict = kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(21))
            .unwrap();
        let validated = started.elapsed();
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{verdict:?}"
        );
        let started = Instant::now();
        let outcome = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(22))
            .unwrap();
        let commit = started.elapsed();
        assert!(matches!(outcome, CommitCommandResult::Committed(_)));
        let graph = kernel.snapshot().unwrap();
        assert_eq!(graph.attachments.len(), ATTACHMENTS);
        println!(
            "{provider}: {ATTACHMENTS} AddEvidence + {ATTACHMENTS} AttachEvidence ({} bytes) \
             against {ASSERTIONS} assertions: propose {proposed:?}, validate {validated:?}, \
             commit {commit:?}, total {:?}",
            bytes.len(),
            proposed + validated + commit
        );
    }
}

/// A public graph document must not silently collapse repeated attachment records.
#[test]
fn adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder() {
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false, profiles()[2].1.clone());
    kernel
        .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
            Timestamp::from_millis(10)
        })
        .unwrap();
    let (held, add) = claim(ALICE, 100, id(SOURCE));
    committed(&kernel, &transaction(vec![add], &[id(SOURCE)]), 20);
    committed(
        &kernel,
        &transaction(vec![attach(held, id(OTHER))], &[id(OTHER)]),
        30,
    );
    let document = ekr_store::GraphDocument::of(&kernel.snapshot().unwrap());
    let bytes = document.to_bytes().unwrap();
    assert_eq!(
        ekr_store::GraphDocument::from_bytes(&bytes).unwrap(),
        document
    );
    let mut wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let entries = wire["graph"]["attachments"][held.to_string()]
        .as_array_mut()
        .unwrap();
    assert_eq!(entries.len(), 1);
    entries.push(entries[0].clone());
    let refusal = ekr_store::GraphDocument::from_bytes(&serde_json::to_vec(&wire).unwrap())
        .expect_err("duplicate attachment must not silently become one record");
    assert!(refusal.to_string().contains("duplicate"), "{refusal}");
}

/// The documented explanation selects only this assertion's attachments, even when their
/// transaction attaches different evidence to another assertion and a checkpoint is restored.
#[test]
fn adversary_checkpoint_explanations_do_not_import_peer_attachments() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let authority = profiles()[2].1.clone();
        let kernel = open(directory.path(), file, authority.clone());
        kernel
            .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
                Timestamp::from_millis(10)
            })
            .unwrap();
        let (alice, add_alice) = claim(ALICE, 100, id(SOURCE));
        let (bob, add_bob) = claim(BOB, 100, id(SOURCE));
        committed(
            &kernel,
            &transaction(vec![add_alice, add_bob], &[id(SOURCE)]),
            20,
        );
        let (alice_evidence, add_alice_evidence) = message("Alice's exact evidence");
        let (bob_evidence, add_bob_evidence) = message("Bob's exact evidence");
        committed(
            &kernel,
            &transaction(
                vec![
                    attach(alice, alice_evidence),
                    add_bob_evidence,
                    attach(bob, bob_evidence),
                    add_alice_evidence,
                ],
                &[alice_evidence, bob_evidence],
            ),
            30,
        );
        let head = kernel.head().unwrap().unwrap();
        kernel.retain_checkpoint_at_rest();
        drop(kernel);
        for full in [false, true] {
            let mut reopened = open(directory.path(), file, authority.clone());
            reopened.set_full_replay(full);
            assert_eq!(reopened.head().unwrap().unwrap(), head);
            for (assertion, own) in [(alice, alice_evidence), (bob, bob_evidence)] {
                let (evidence, attachments) = explained(&reopened, None, assertion);
                assert_eq!(
                    evidence.into_iter().collect::<BTreeSet<_>>(),
                    BTreeSet::from([id(SOURCE), own])
                );
                assert_eq!(attachments, [(assertion, own, RevisionNumber::new(2))]);
                assert_eq!(
                    explained(&reopened, Some(RevisionNumber::new(1)), assertion),
                    (vec![id(SOURCE)], vec![])
                );
            }
        }
    }
}

/// Validation cannot attach evidence after a competing retraction commits.
#[test]
fn adversary_a_validated_attachment_loses_to_a_committed_retraction() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file, profiles()[2].1.clone());
        kernel
            .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
                Timestamp::from_millis(10)
            })
            .unwrap();
        let (held, add) = claim(ALICE, 100, id(SOURCE));
        committed(&kernel, &transaction(vec![add], &[id(SOURCE)]), 20);
        let pending = transaction(vec![attach(held, id(OTHER))], &[id(OTHER)]);
        kernel
            .propose(&encode(&pending), context().operator, || {
                Timestamp::from_millis(30)
            })
            .unwrap();
        assert!(matches!(
            kernel
                .validate(pending.id, RevisionNumber::new(1), || {
                    Timestamp::from_millis(31)
                })
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        committed(
            &kernel,
            &transaction(
                vec![GraphOperation::RetractAssertion(Retraction {
                    assertion: held,
                    reason: ekr_graph::RetractionReason::new("withdrawn before attachment"),
                })],
                &[],
            ),
            40,
        );
        let head = kernel.head().unwrap().unwrap();
        let result = kernel
            .commit(pending.id, context().operator, || {
                Timestamp::from_millis(50)
            })
            .unwrap();
        assert!(
            matches!(result, CommitCommandResult::Stale(_)),
            "{result:?}"
        );
        assert_eq!(kernel.head().unwrap().unwrap(), head);
        assert!(kernel.snapshot().unwrap().attachments.is_empty());
    }
}

#[test]
fn adversary_equivalent_attachment_spellings_are_duplicate_records() {
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false, profiles()[2].1.clone());
    kernel
        .seed(SeedDocument::from_yaml(SEED).unwrap(), || {
            Timestamp::from_millis(10)
        })
        .unwrap();
    let (held, add) = claim(ALICE, 100, id(SOURCE));
    committed(&kernel, &transaction(vec![add], &[id(SOURCE)]), 20);
    committed(
        &kernel,
        &transaction(vec![attach(held, id(OTHER))], &[id(OTHER)]),
        30,
    );
    let document = ekr_store::GraphDocument::of(&kernel.snapshot().unwrap());
    let bytes = document.to_bytes().unwrap();
    assert_eq!(
        ekr_store::GraphDocument::from_bytes(&bytes).unwrap(),
        document
    );
    let entry = serde_json::to_string(document.attachments[&held].first().unwrap()).unwrap();
    let escaped = entry.replacen('-', "\\u002d", 1);
    assert_ne!(entry, escaped);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&entry).unwrap(),
        serde_json::from_str::<serde_json::Value>(&escaped).unwrap()
    );
    let needle = format!("[{entry}]");
    let encoded = String::from_utf8(bytes).unwrap();
    assert!(encoded.contains(&needle));
    let duplicate = encoded.replacen(&needle, &format!("[{entry},{escaped}]"), 1);
    let refusal = ekr_store::GraphDocument::from_bytes(duplicate.as_bytes()).unwrap_err();
    assert!(
        refusal.to_string().contains("duplicate decoded set member"),
        "{refusal}"
    );
}
