//! Adversary pass, unit H (issue #96, story:view-historical-changes-and-cancelled-reads):
//! `IndexCache::changes_as` answers a historical `at` from the head's index. The unit's own test
//! covers fixtures without a merge; a merge after `at` rewrites a subject in place (design
//! § 111.2), and a commit may land between two requests of one cache. Store setup copied from
//! `adversary_w20261020b_merge_changes.rs` (revision 1 adds an assertion about the duplicate,
//! revision 2 retracts it, revision 3 merges the duplicate into the node kept).

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EvidenceId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, Runtime, SeedDocument,
    ValidationCommandResult, ValidationProfileV1,
};
use ekr_views::{ChangesFormat, ChangesRequest, Index, IndexCache, SinceKind};

/// The kernel's merge seed: WorkItems `group/app!18` (kept) and `group/app!18 (imported)` (the
/// duplicate), among others.
const SEED: &str = include_str!("../../ekr-kernel/tests/fixtures/merge-seed.yaml");
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const TITLE: &str = "00000000-0000-4000-8000-000000000211";
const KEPT: &str = "00000000-0000-4000-8000-000000000301";
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const STATEMENT_TEXT: &str = "group/app!18 was imported twice; import/18 is group/app!18.";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const VALIDATOR: &str = "00000000-0000-4000-8000-000000000102";
/// The assertion about the duplicate that revision 1 adds and revision 2 retracts.
const CLAIM: &str = "00000000-0000-4000-8000-00000000ad01";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: OPERATOR.parse().unwrap(),
        validator: VALIDATOR.parse().unwrap(),
    }
}

fn authority() -> AuthorityStateV1 {
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

fn seed() -> SeedDocument {
    let mut document = SeedDocument::from_yaml(SEED).unwrap();
    let hash = ContentHash::of_bytes(STATEMENT_TEXT.as_bytes());
    let id: EvidenceId = STATEMENT.parse().unwrap();
    document.graph.evidence.insert(
        id,
        Evidence {
            id,
            source: EvidenceSource::HumanStatement {
                identity: Some("Runtime operator".into()),
            },
            content_hash: hash,
            extracted_by: context().operator,
            observed_at: Timestamp::from_millis(1_773_273_600_000),
            confidence: Confidence::CERTAIN,
        },
    );
    document.evidence_payloads.insert(
        hash,
        std::sync::Arc::new(STATEMENT_TEXT.as_bytes().to_vec()),
    );
    document
}

/// Proposes, validates and commits the `ekr.transaction-document/2` whose transaction is `id`,
/// holding `operations` and resting on `evidence`, at `at`, `at + 1` and `at + 2`.
fn commit(kernel: &Runtime, id: &str, operations: &str, evidence: &str, at: i64) {
    let document = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: {OPERATOR}\n  \
         operations:\n{operations}  evidence:{evidence}\n"
    );
    let operator = context().operator;
    kernel
        .propose(document.as_bytes(), operator, || Timestamp::from_millis(at))
        .unwrap();
    let transaction: TransactionId = id.parse().unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(transaction, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{id}: {verdict:?}"
    );
    let committed = kernel
        .commit(transaction, operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{id}: {committed:?}"
    );
}

/// Revisions 1 and 2: an assertion about the duplicate, added and then retracted.
fn add_and_retract(kernel: &Runtime) {
    commit(
        kernel,
        "00000000-0000-4000-8000-00000000ad11",
        &format!(
            "  - !AddAssertion\n    id: {CLAIM}\n    root_id: {ROOT}\n    subject: !Node \
             {DUPLICATE}\n    predicate: !Property {TITLE}\n    object: !Value {{value_kind: \
             String, value: Fix the parser}}\n    evidence:\n    - {STATEMENT}\n    \
             proposed_by: {OPERATOR}\n    assessment: Proposed\n    lifecycle: Active\n    \
             valid_time:\n      from: 1577836800000\n      to: null\n    transaction_time:\n      \
             recorded_from: 0\n      recorded_to: null\n"
        ),
        &format!("\n  - {STATEMENT}"),
        20,
    );
    commit(
        kernel,
        "00000000-0000-4000-8000-00000000ad12",
        &format!(
            "  - !RetractAssertion\n    assertion: {CLAIM}\n    reason: the title was a guess\n"
        ),
        " []",
        30,
    );
}

/// Revision 3: the duplicate merged into the node kept.
fn merge(kernel: &Runtime) {
    commit(
        kernel,
        "00000000-0000-4000-8000-00000000ad13",
        &format!(
            "  - !MergeEntity\n    absorbed: {DUPLICATE}\n    into: {KEPT}\n    reason: the import \
             wrote group/app!18 twice\n    evidence:\n    - {STATEMENT}\n"
        ),
        &format!("\n  - {STATEMENT}"),
        40,
    );
}

/// Each provider, freshly seeded.
fn each(mut run: impl FnMut(&str, &Runtime)) {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = if file {
            Runtime::file(directory.path(), "test", context(), authority())
        } else {
            Runtime::sqlite(
                &directory.path().join("state.db"),
                "test",
                context(),
                authority(),
            )
        }
        .unwrap();
        kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
        run(if file { "file" } else { "sqlite" }, &kernel);
    }
}

fn requests() -> Vec<ChangesRequest> {
    vec![
        ChangesRequest::new(SinceKind::Revision, 0, None, None).expect("bounds"),
        ChangesRequest::new(SinceKind::Revision, 1, Some(1), Some(1)).expect("bounds"),
        ChangesRequest::new(SinceKind::Revision, 3, None, None).expect("bounds"),
        ChangesRequest::new(SinceKind::TransactionTime, 25, None, None).expect("bounds"),
        ChangesRequest::new(SinceKind::ValidTime, 0, None, None).expect("bounds"),
    ]
}

const FORMATS: [ChangesFormat; 2] = [ChangesFormat::V1, ChangesFormat::V2];

/// Every revision before and at a merge, every since kind (a since beyond `at` included) and both
/// formats, read from the head's index after the merge, answers the bytes a freshly loaded index
/// of that revision answers.
#[test]
fn adv_h_a_historical_at_before_a_merge_answers_a_loaded_indexs_bytes() {
    each(|provider, kernel| {
        add_and_retract(kernel);
        merge(kernel);
        let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
        for at in 0..=3 {
            for request in requests() {
                for format in FORMATS {
                    let served = cache
                        .changes_as(kernel, Some(RevisionNumber::new(at)), &request, format)
                        .expect("answers");
                    let fresh = Index::load(kernel, Some(RevisionNumber::new(at)))
                        .expect("indexes")
                        .changes_as(kernel, &request, format)
                        .expect("answers");
                    assert_eq!(
                        String::from_utf8_lossy(&served.bytes),
                        String::from_utf8_lossy(&fresh.bytes),
                        "{provider} at {at} {request:?} {format:?}"
                    );
                    assert_eq!(served.summary, fresh.summary, "{provider} at {at}");
                }
            }
        }
        assert_eq!(cache.len(), 1, "{provider}: only the head's index is held");
    });
}

/// One cache, a commit between two requests: a historical `at` read after the merge answers what
/// a freshly loaded index of that revision answers, and an absent `at` names the new head.
#[test]
fn adv_h_a_commit_between_requests_moves_the_head_and_keeps_historical_bytes() {
    each(|provider, kernel| {
        add_and_retract(kernel);
        let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
        let mut before = Vec::new();
        for at in 0..=2 {
            for format in FORMATS {
                before.push(
                    cache
                        .changes_as(
                            kernel,
                            Some(RevisionNumber::new(at)),
                            &requests()[0],
                            format,
                        )
                        .expect("answers")
                        .bytes,
                );
            }
        }
        let head_before = cache
            .changes_as(kernel, None, &requests()[0], ChangesFormat::V1)
            .expect("answers");
        assert_eq!(head_before.summary.revision, 2, "{provider}");

        merge(kernel);
        let head_after = cache
            .changes_as(kernel, None, &requests()[0], ChangesFormat::V1)
            .expect("answers");
        assert_eq!(head_after.summary.revision, 3, "{provider}: the head moved");
        let mut after = Vec::new();
        for at in 0..=2 {
            for format in FORMATS {
                let served = cache
                    .changes_as(
                        kernel,
                        Some(RevisionNumber::new(at)),
                        &requests()[0],
                        format,
                    )
                    .expect("answers")
                    .bytes;
                let fresh = Index::load(kernel, Some(RevisionNumber::new(at)))
                    .expect("indexes")
                    .changes_as(kernel, &requests()[0], format)
                    .expect("answers")
                    .bytes;
                assert_eq!(
                    String::from_utf8_lossy(&served),
                    String::from_utf8_lossy(&fresh),
                    "{provider} at {at} {format:?} after the merge"
                );
                after.push(served);
            }
        }
        assert_eq!(
            before, after,
            "{provider}: a historical answer moved with the head"
        );
    });
}
