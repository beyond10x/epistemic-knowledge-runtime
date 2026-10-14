//! `story:schema-transaction-cites-evidence`: a schema transaction cites the evidence that
//! introduced it, on both providers, under validation profiles v2 and v3 (design § 109).
//!
//! A consumer grows its schema one type at a time, each just before the first fact that uses it,
//! and wants each version to say which message introduced it. `!CiteEvidence <evidence id>`
//! beside the schema changes names evidence the store retains, or evidence an `!AddEvidence` of
//! the same transaction adds; commit records every cited id in the version it makes
//! (`SchemaVersion::evidence`), and every later read of that version names them. A schema
//! transaction that carries any other data operation is still `mixed-schema-transaction`.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    ContentHash, EvidenceId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId,
    TypeId,
};
use ekr_graph::{CanonicalGraph, Confidence, Evidence, EvidenceSource, Root};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EvidenceAddition,
    GraphOperation, GraphTransaction, NodeDraft, Runtime, SeedDocument, ValidationCommandResult,
    ValidationProfileV1, ValidatorName,
};
use ekr_ontology::NodeType;
use serde::Serialize;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
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

/// The two profiles that admit schema changes.
fn evolving() -> [(&'static str, AuthorityStateV1); 2] {
    [
        (
            "v2",
            anchor(ValidationProfileV1::schema_evolving(context().validator)),
        ),
        (
            "v3",
            anchor(ValidationProfileV1::identity_keeping(context().validator)),
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

/// The minimal v2 seed with one node type, `Message`, so that a data operation has a type.
fn seed() -> (SeedDocument, TypeId) {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let message = TypeId::mint();
    document
        .ontology
        .node_types
        .push(NodeType::new(message, "Message"));
    (document, message)
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

fn transaction(
    operations: Vec<GraphOperation>,
    evidence: &[EvidenceId],
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: evidence.iter().copied().collect(),
        schema_version,
    }
}

/// One evidence entry for `text`, under a fresh id, attributed to the operator.
fn message(text: &str) -> (EvidenceId, ContentHash, GraphOperation) {
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
    let (id, hash) = (evidence.id, evidence.content_hash);
    (
        id,
        hash,
        GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence,
            payload: text.as_bytes().to_vec(),
        })),
    )
}

fn define(name: &str) -> (TypeId, GraphOperation) {
    let id = TypeId::mint();
    (
        id,
        GraphOperation::DefineNodeType(Box::new(NodeType::new(id, name))),
    )
}

fn create(seed: &SeedDocument, type_id: TypeId) -> GraphOperation {
    GraphOperation::CreateNode(NodeDraft {
        id: NodeId::mint(),
        root_id: seed.graph.root.id,
        type_id,
        canonical_name: "a message".into(),
        aliases: Vec::new(),
        properties: BTreeMap::new(),
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

/// Every revision's graph as this runtime replays it, with the root its commit receipt holds.
fn every_revision(kernel: &Runtime) -> Vec<(Option<Root>, CanonicalGraph)> {
    let head = kernel.head().unwrap().unwrap();
    let records = kernel.transactions().unwrap();
    (0..=head.revision.get())
        .map(|number| {
            let graph = kernel.replay(RevisionNumber::new(number)).unwrap();
            let root = records
                .values()
                .filter_map(|record| record.committed.as_ref())
                .find(|receipt| receipt.result.revision == graph.revision)
                .map(|receipt| receipt.result);
            if let Some(root) = root {
                assert_eq!(root.ontology_root, ContentHash::of(&graph.ontology));
            }
            (root, graph)
        })
        .collect()
}

/// Acceptance 1 and 2, kernel side: a schema transaction citing one retained evidence entry, and
/// one adding and citing one, each commit as a new schema version, whose record names the cited
/// evidence; the evidence an `AddEvidence` brings is retained with its payload; and a reopened
/// store, from its checkpoint and from the seed, replays every root and every version's evidence.
#[test]
fn a_schema_transaction_citing_retained_or_added_evidence_commits_and_the_version_records_it() {
    for (profile, authority) in evolving() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let (seed, _) = seed();
            let seed_version = seed.ontology.version.id;
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();

            // Revision 1 retains a message no assertion cites.
            let (retained, _, add_retained) = message("Reviews are recorded from now on.");
            let verdict = submit(&kernel, &transaction(vec![add_retained], &[], None), 20);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );

            // Revision 2: a schema change citing the retained message.
            let (review, define_review) = define("Review");
            let first = SchemaVersionId::mint();
            let citing = transaction(
                vec![define_review, GraphOperation::CiteEvidence(retained)],
                &[retained],
                Some(first),
            );
            let verdict = submit(&kernel, &citing, 30);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let head = kernel.snapshot().unwrap();
            assert_eq!(head.revision, RevisionNumber::new(2), "{at}");
            let version = head.ontology.version();
            assert_eq!(
                (version.id, version.number, version.parent),
                (first, 1, Some(seed_version)),
                "{at}"
            );
            assert_eq!(version.evidence, BTreeSet::from([retained]), "{at}");
            assert!(head.ontology.node_type(review).is_some(), "{at}");

            // Revision 3: a schema change adding the message it cites.
            let (added, added_hash, add_cited) = message("Topics are their own type.");
            let (topic, define_topic) = define("Topic");
            let second = SchemaVersionId::mint();
            let adding = transaction(
                vec![add_cited, define_topic, GraphOperation::CiteEvidence(added)],
                &[added],
                Some(second),
            );
            let verdict = submit(&kernel, &adding, 40);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let head = kernel.snapshot().unwrap();
            let version = head.ontology.version();
            assert_eq!(
                (version.id, version.number, version.parent),
                (second, 2, Some(first)),
                "{at}"
            );
            assert_eq!(version.evidence, BTreeSet::from([added]), "{at}");
            assert!(head.ontology.node_type(topic).is_some(), "{at}");
            assert!(
                head.evidence.contains_key(&added),
                "{at}: added is retained"
            );
            assert_eq!(
                kernel.content(&added_hash).unwrap().as_deref(),
                Some("Topics are their own type.".as_bytes()),
                "{at}: the payload is held at its content hash"
            );
            // The earlier version keeps its own citation.
            let at_two = kernel.replay(RevisionNumber::new(2)).unwrap();
            assert_eq!(
                at_two.ontology.version().evidence,
                BTreeSet::from([retained]),
                "{at}"
            );

            let expected = every_revision(&kernel);
            let expected_head = kernel.head().unwrap();
            drop(kernel);
            let reopened = open(directory.path(), file, authority.clone());
            assert_eq!(reopened.head().unwrap(), expected_head, "{at}");
            assert_eq!(every_revision(&reopened), expected, "{at}");
            drop(reopened);
            let mut full = open(directory.path(), file, authority.clone());
            full.set_full_replay(true);
            assert_eq!(full.head().unwrap(), expected_head, "{at}: full replay");
            assert_eq!(every_revision(&full), expected, "{at}: full replay");
        }
    }
}

/// Acceptance 3: a schema transaction that carries any other data operation — a create, or an
/// `AddEvidence` of an entry it does not cite — is refused as `mixed-schema-transaction`, by the
/// structural validator, citing or not; the head does not move.
#[test]
fn a_schema_transaction_carrying_another_data_operation_is_still_mixed() {
    for (profile, authority) in evolving() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let (seed, message_type) = seed();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();

            let (cited, _, add_cited) = message("Reviews are recorded from now on.");
            let (uncited, _, add_uncited) = message("Something else entirely.");
            let (_, define_review) = define("Review");
            let cases = [
                (
                    "a create beside a cited addition",
                    vec![
                        add_cited.clone(),
                        define_review.clone(),
                        GraphOperation::CiteEvidence(cited),
                        create(&seed, message_type),
                    ],
                    vec![cited],
                ),
                (
                    "an addition the version does not cite",
                    vec![
                        add_cited.clone(),
                        add_uncited,
                        define_review.clone(),
                        GraphOperation::CiteEvidence(cited),
                    ],
                    vec![cited],
                ),
                (
                    "an addition and no citation",
                    vec![add_cited, define_review],
                    vec![],
                ),
            ];
            for (step, (case, operations, evidence)) in cases.into_iter().enumerate() {
                let tx = transaction(operations, &evidence, Some(SchemaVersionId::mint()));
                let at_ms = 20 + 10 * i64::try_from(step).unwrap();
                let codes = refused(&submit(&kernel, &tx, at_ms));
                assert_eq!(
                    codes,
                    vec![(
                        ValidatorName::Structural,
                        "mixed-schema-transaction".to_owned()
                    )],
                    "{at}: {case}"
                );
            }
            let _ = uncited;
            assert_eq!(
                kernel.head().unwrap().unwrap().revision,
                RevisionNumber::SEED,
                "{at}"
            );
        }
    }
}

/// The refusals of a citation, each by its own validator and code: evidence no revision holds
/// and no `AddEvidence` adds (`unresolved-evidence`, Reference); a citation in a transaction that
/// changes no schema (`citation-without-schema-change`, Structural); one id cited twice
/// (`duplicate-citation`, Structural); a cited id the manifest leaves out
/// (`evidence-set-mismatch`, Structural); and under profile v1, `unsupported-operation`.
#[test]
fn a_citation_is_refused_by_name() {
    let (_, v2) = evolving().into_iter().next().unwrap();
    for file in [false, true] {
        let at = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let (seed, _) = seed();
        let kernel = open(directory.path(), file, v2.clone());
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (held, _, add_held) = message("Reviews are recorded from now on.");
        let verdict = submit(&kernel, &transaction(vec![add_held], &[], None), 20);
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{at}"
        );

        let nowhere = EvidenceId::mint();
        let version = || Some(SchemaVersionId::mint());
        let cases = [
            (
                transaction(
                    vec![define("Review").1, GraphOperation::CiteEvidence(nowhere)],
                    &[nowhere],
                    version(),
                ),
                vec![(ValidatorName::Reference, "unresolved-evidence")],
            ),
            (
                transaction(vec![GraphOperation::CiteEvidence(held)], &[held], None),
                vec![(ValidatorName::Structural, "citation-without-schema-change")],
            ),
            (
                transaction(
                    vec![
                        define("Review").1,
                        GraphOperation::CiteEvidence(held),
                        GraphOperation::CiteEvidence(held),
                    ],
                    &[held],
                    version(),
                ),
                vec![(ValidatorName::Structural, "duplicate-citation")],
            ),
            (
                transaction(
                    vec![define("Review").1, GraphOperation::CiteEvidence(held)],
                    &[],
                    version(),
                ),
                vec![(ValidatorName::Structural, "evidence-set-mismatch")],
            ),
        ];
        for (step, (tx, expected)) in cases.into_iter().enumerate() {
            let at_ms = 30 + 10 * i64::try_from(step).unwrap();
            let expected: Vec<(ValidatorName, String)> = expected
                .into_iter()
                .map(|(validator, code)| (validator, code.to_owned()))
                .collect();
            assert_eq!(refused(&submit(&kernel, &tx, at_ms)), expected, "{at}");
        }
        assert_eq!(
            kernel.head().unwrap().unwrap().revision,
            RevisionNumber::new(1),
            "{at}"
        );
    }

    // Profile v1 applies no schema change, and no citation either.
    let v1 = anchor(ValidationProfileV1::deterministic(context().validator));
    let directory = tempfile::tempdir().unwrap();
    let (seed, _) = seed();
    let kernel = open(directory.path(), false, v1);
    kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();
    let (held, _, add_held) = message("Reviews are recorded from now on.");
    submit(&kernel, &transaction(vec![add_held], &[], None), 20);
    let tx = transaction(
        vec![define("Review").1, GraphOperation::CiteEvidence(held)],
        &[held],
        Some(SchemaVersionId::mint()),
    );
    let codes = refused(&submit(&kernel, &tx, 30));
    assert!(
        codes.contains(&(
            ValidatorName::Structural,
            "unsupported-operation".to_owned()
        )),
        "v1: {codes:?}"
    );
}
