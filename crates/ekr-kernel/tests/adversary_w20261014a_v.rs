//! Adversary cases for `story:schema-transaction-cites-evidence` (design § 109), kernel side.
//!
//! Each case drives the kernel through `Runtime` on the file and SQLite providers: the operation
//! order of a cited addition, an addition that names a held id, an addition the provenance profile
//! refuses, a data operation of another kind beside a citation, a seed whose version cites
//! evidence, the refusal message a store retained before citations existed, and a store holding
//! every citation refusal reopened from its checkpoint and from the seed.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EvidenceAddition,
    EvidenceAttachment, GraphOperation, GraphTransaction, NodeDraft, Runtime, SeedDocument,
    ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::NodeType;
use serde::Serialize;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn v2() -> AuthorityStateV1 {
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
        validation_profile: ValidationProfileV1::schema_evolving(c.validator),
    }
}

fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), v2())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), v2())
    }
    .unwrap()
}

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

fn evidence_for(text: &str, hash_of: &str) -> (EvidenceId, GraphOperation) {
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("a consumer's message".into()),
        },
        content_hash: ContentHash::of_bytes(hash_of.as_bytes()),
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

fn message(text: &str) -> (EvidenceId, GraphOperation) {
    evidence_for(text, text)
}

/// An `AddEvidence` for an id the caller chooses.
fn message_as(id: EvidenceId, text: &str) -> GraphOperation {
    let (_, operation) = message(text);
    match operation {
        GraphOperation::AddEvidence(mut addition) => {
            addition.evidence.id = id;
            GraphOperation::AddEvidence(addition)
        }
        _ => unreachable!(),
    }
}

fn define(name: &str) -> (TypeId, GraphOperation) {
    let id = TypeId::mint();
    (
        id,
        GraphOperation::DefineNodeType(Box::new(NodeType::new(id, name))),
    )
}

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

fn issues(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone(), issue.message.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

fn codes(verdict: &ValidationCommandResult) -> Vec<String> {
    issues(verdict)
        .into_iter()
        .map(|(_, code, _)| code)
        .collect()
}

fn seeded(file: bool) -> (tempfile::TempDir, Runtime, SeedDocument, TypeId) {
    let directory = tempfile::tempdir().unwrap();
    let (seed, message_type) = seed();
    let kernel = open(directory.path(), file);
    kernel
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    (directory, kernel, seed, message_type)
}

fn provider(file: bool) -> &'static str {
    if file {
        "file"
    } else {
        "sqlite"
    }
}

/// A transaction is unordered: the `CiteEvidence` written before the `AddEvidence` it cites
/// commits exactly as the order the unit's tests use does, and the version records the id.
#[test]
fn a_citation_written_before_the_addition_it_cites_commits() {
    for file in [false, true] {
        let at = provider(file);
        let (_directory, kernel, _, _) = seeded(file);
        let (added, add) = message("Topics are their own type.");
        let (topic, define_topic) = define("Topic");
        let version = SchemaVersionId::mint();
        let tx = transaction(
            vec![GraphOperation::CiteEvidence(added), define_topic, add],
            &[added],
            Some(version),
        );
        let verdict = submit(&kernel, &tx, 20);
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{at}: {verdict:?}"
        );
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.ontology.version().id, version, "{at}");
        assert_eq!(
            head.ontology.version().evidence,
            BTreeSet::from([added]),
            "{at}"
        );
        assert!(head.ontology.node_type(topic).is_some(), "{at}");
        assert!(head.evidence.contains_key(&added), "{at}");
    }
}

/// A cited `AddEvidence` travels with the schema change, so before § 109 it was refused as mixed
/// whatever it named. One that re-adds an id canonical state already retains must still be
/// refused, and must not overwrite the retained entry.
#[test]
fn a_cited_addition_of_an_id_already_retained_is_refused_and_overwrites_nothing() {
    for file in [false, true] {
        let at = provider(file);
        let (_directory, kernel, _, _) = seeded(file);
        let (held, add_held) = message("Reviews are recorded from now on.");
        let verdict = submit(&kernel, &transaction(vec![add_held], &[], None), 20);
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{at}"
        );
        let before = kernel.snapshot().unwrap().evidence[&held].clone();

        let tx = transaction(
            vec![
                define("Review").1,
                GraphOperation::CiteEvidence(held),
                message_as(held, "A different payload under the held id."),
            ],
            &[held],
            Some(SchemaVersionId::mint()),
        );
        let verdict = submit(&kernel, &tx, 30);
        assert!(
            matches!(verdict, ValidationCommandResult::Rejected(_)),
            "{at}: a re-added held id committed: {verdict:?}"
        );
        assert_eq!(
            kernel.head().unwrap().unwrap().revision,
            RevisionNumber::new(1),
            "{at}"
        );
        assert_eq!(kernel.snapshot().unwrap().evidence[&held], before, "{at}");
    }
}

/// The added entry is held to the provenance profile exactly as any `AddEvidence` is (§ 109.2):
/// a cited addition whose payload is not the bytes its content hash names is
/// `evidence-payload-mismatch`, and nothing commits.
#[test]
fn a_cited_addition_whose_payload_does_not_match_its_hash_is_refused() {
    for file in [false, true] {
        let at = provider(file);
        let (_directory, kernel, _, _) = seeded(file);
        let (added, add) = evidence_for("the bytes sent", "other bytes entirely");
        let tx = transaction(
            vec![add, define("Topic").1, GraphOperation::CiteEvidence(added)],
            &[added],
            Some(SchemaVersionId::mint()),
        );
        let verdict = submit(&kernel, &tx, 20);
        let codes = codes(&verdict);
        assert!(
            codes.iter().any(|code| code == "evidence-payload-mismatch"),
            "{at}: {codes:?}"
        );
        assert_eq!(
            kernel.head().unwrap().unwrap().revision,
            RevisionNumber::SEED,
            "{at}"
        );
    }
}

/// Any operation beside the schema changes other than a citation and the addition it cites is
/// still mixed: an `AttachEvidence` of the cited entry to a held assertion is not exempt because
/// it names the same evidence.
#[test]
fn an_attachment_of_the_cited_evidence_beside_a_schema_change_is_still_mixed() {
    for file in [false, true] {
        let at = provider(file);
        let (_directory, kernel, _, _) = seeded(file);
        let (held, add_held) = message("Reviews are recorded from now on.");
        submit(&kernel, &transaction(vec![add_held], &[], None), 20);
        let tx = transaction(
            vec![
                define("Review").1,
                GraphOperation::CiteEvidence(held),
                GraphOperation::AttachEvidence(EvidenceAttachment {
                    assertion: AssertionId::mint(),
                    evidence: held,
                }),
            ],
            &[held],
            Some(SchemaVersionId::mint()),
        );
        let verdict = submit(&kernel, &tx, 30);
        let codes = codes(&verdict);
        assert!(
            codes.iter().any(|code| code == "mixed-schema-transaction"),
            "{at}: {codes:?}"
        );
        assert_eq!(
            kernel.head().unwrap().unwrap().revision,
            RevisionNumber::new(1),
            "{at}"
        );
    }
}

/// § 109.3: a seed whose `ontology.version.evidence` is not empty is `seed-ontology-lineage`.
#[test]
fn a_seed_whose_version_cites_evidence_is_refused_as_seed_ontology_lineage() {
    for file in [false, true] {
        let at = provider(file);
        let directory = tempfile::tempdir().unwrap();
        let (mut seed, _) = seed();
        seed.ontology.version.evidence.insert(EvidenceId::mint());
        let kernel = open(directory.path(), file);
        let refusal = kernel
            .seed(seed, || Timestamp::from_millis(10))
            .expect_err("a seed version citing evidence was admitted");
        assert!(
            refusal.to_string().contains("seed-ontology-lineage"),
            "{at}: {refusal}"
        );
        assert!(kernel.head().unwrap().is_none(), "{at}");
    }
}

/// A store written before § 109 retains a refusal of a schema transaction carrying an
/// `AddEvidence`. Replay compares that rejection's message with what the ruleset says now, so a
/// transaction without a citation must read the base's words exactly (the literal below is the
/// format string at the wave's base, `structural.rs`, `schema_shape`).
#[test]
fn a_mixed_refusal_without_a_citation_reads_the_words_the_base_wrote() {
    for file in [false, true] {
        let at = provider(file);
        let (_directory, kernel, _, _) = seeded(file);
        let (_, add) = message("Reviews are recorded from now on.");
        let tx = transaction(
            vec![add, define("Review").1],
            &[],
            Some(SchemaVersionId::mint()),
        );
        let verdict = submit(&kernel, &tx, 20);
        assert_eq!(
            issues(&verdict),
            vec![(
                ValidatorName::Structural,
                "mixed-schema-transaction".to_owned(),
                "1 of the transaction's 2 operations change the schema; a schema change is \
                 proposed in a transaction of its own"
                    .to_owned()
            )],
            "{at}"
        );
    }
}

/// Every citation refusal is retained, and replay revalidates every retained rejection against
/// today's ruleset: reopening from the checkpoint and with a full replay from the seed answers the
/// same head, the same retained records and the same version, after a citing commit too.
#[test]
fn a_store_holding_every_citation_refusal_reopens_and_replays_from_the_seed() {
    for file in [false, true] {
        let at = provider(file);
        let (directory, kernel, seed, message_type) = seeded(file);
        let (held, add_held) = message("Reviews are recorded from now on.");
        submit(&kernel, &transaction(vec![add_held], &[], None), 20);
        let nowhere = EvidenceId::mint();
        let (other, add_other) = message("Something else entirely.");
        let version = || Some(SchemaVersionId::mint());
        let refusals = [
            transaction(
                vec![define("A").1, GraphOperation::CiteEvidence(nowhere)],
                &[nowhere],
                version(),
            ),
            transaction(vec![GraphOperation::CiteEvidence(held)], &[held], None),
            transaction(
                vec![
                    define("B").1,
                    GraphOperation::CiteEvidence(held),
                    GraphOperation::CiteEvidence(held),
                ],
                &[held],
                version(),
            ),
            transaction(
                vec![define("C").1, GraphOperation::CiteEvidence(held)],
                &[],
                version(),
            ),
            transaction(
                vec![
                    define("D").1,
                    GraphOperation::CiteEvidence(held),
                    add_other,
                    GraphOperation::CreateNode(NodeDraft {
                        id: NodeId::mint(),
                        root_id: seed.graph.root.id,
                        type_id: message_type,
                        canonical_name: "a message".into(),
                        aliases: Vec::new(),
                        properties: BTreeMap::new(),
                    }),
                ],
                &[held],
                version(),
            ),
        ];
        let _ = other;
        for (step, tx) in refusals.iter().enumerate() {
            let at_ms = 30 + 10 * i64::try_from(step).unwrap();
            let verdict = submit(&kernel, tx, at_ms);
            assert!(
                matches!(verdict, ValidationCommandResult::Rejected(_)),
                "{at}: refusal {step} validated"
            );
        }
        let (_, define_e) = define("E");
        let citing = transaction(
            vec![define_e, GraphOperation::CiteEvidence(held)],
            &[held],
            version(),
        );
        let verdict = submit(&kernel, &citing, 100);
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{at}: {verdict:?}"
        );

        let head = kernel.head().unwrap();
        let records = kernel.transactions().unwrap();
        let ontology = kernel.snapshot().unwrap().ontology;
        drop(kernel);
        for full in [false, true] {
            let mut reopened = open(directory.path(), file);
            reopened.set_full_replay(full);
            assert_eq!(reopened.head().unwrap(), head, "{at} full={full}");
            assert_eq!(
                reopened.transactions().unwrap(),
                records,
                "{at} full={full}"
            );
            assert_eq!(
                reopened.snapshot().unwrap().ontology,
                ontology,
                "{at} full={full}"
            );
            assert_eq!(
                reopened.snapshot().unwrap().ontology.version().evidence,
                BTreeSet::from([held]),
                "{at} full={full}"
            );
        }
    }
}
