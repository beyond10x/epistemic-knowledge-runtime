//! Adversary pass 1, wave 2026-10-20b, `story:merge-entity-applies`: what `ChangesSince` lists
//! for a revision before a merge, read after it.
//!
//! `crates/ekr-views/src/changes.rs` derives each revision's change list once and holds it, keyed
//! by the revision's identity, for every later read (`ekr-mcp-fixes-20261015 #5`). Its module doc
//! rests that on one fact: a retracted or superseded assertion's subject, read from the read
//! revision's graph, is one "which no later revision changes, so a list derived at one revision is
//! the list derived at any later one". A merge rewrites an assertion's subject in place
//! (design § 111.2, step 5), so a later revision does change it. `views.yaml`
//! (`ekr.graph-changes/1`) reads the changes of one revision "off the transaction it committed".
//!
//! The store: revision 1 adds an assertion about the duplicate, revision 2 retracts it, revision 3
//! merges the duplicate into the node kept. `ekr` holds an [`IndexCache`] per session
//! (`crates/ekr/src/cli/session.rs`), so a session that read the changes before the merge and
//! reads them again after it answers from the list it held.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EvidenceId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, Runtime, SeedDocument,
    ValidationCommandResult, ValidationProfileV1,
};
use ekr_views::{ChangesRequest, Index, IndexCache, SinceKind};
use serde_json::Value;

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

fn since_seed() -> ChangesRequest {
    ChangesRequest::new(SinceKind::Revision, 0, None, None).expect("bounds")
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

/// The `subject` of revision 2's `AssertionRetracted` change in a changes page.
fn retracted_subject(bytes: &[u8]) -> Value {
    let document: Value = serde_json::from_slice(bytes).unwrap();
    document["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["change"] == "AssertionRetracted")
        .map(|change| change["subject"].clone())
        .unwrap_or_else(|| panic!("revision 2 lists its retraction: {document}"))
}

/// A session that read the changes before the merge, and reads them at the head after it, answers
/// with the bytes a fresh read of the same head answers with: the change list it held for
/// revision 2 is the list revision 3's index derives.
#[test]
fn adv_a_change_list_held_from_before_a_merge_answers_as_one_derived_after_it() {
    each(|provider, kernel| {
        add_and_retract(kernel);
        let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
        let before = cache.index(kernel, None).expect("revision 2 indexes");
        before.changes(kernel, &since_seed()).expect("answers");

        merge(kernel);
        let held = cache.index(kernel, None).expect("revision 3 indexes");
        assert_eq!(held.revision(), RevisionNumber::new(3), "{provider}");
        let cached = held.changes(kernel, &since_seed()).expect("answers");
        let fresh = Index::load(kernel, None)
            .expect("revision 3 indexes")
            .changes(kernel, &since_seed())
            .expect("answers");
        assert_eq!(
            String::from_utf8_lossy(&cached.bytes),
            String::from_utf8_lossy(&fresh.bytes),
            "{provider}: the same ChangesSince at revision 3 answers differently from a session \
             that held revision 2's list (retraction about {}) and from a fresh read (about {})",
            retracted_subject(&cached.bytes),
            retracted_subject(&fresh.bytes)
        );
    });
}

/// Without any cache: revision 2's retraction is listed about the subject the assertion had when
/// revision 2 committed it, whichever later revision reads it — the subject its own
/// `AssertionAdded` at revision 1 names, and the subject revision 2's index lists.
#[test]
fn adv_a_retraction_before_a_merge_is_listed_about_the_subject_it_retracted() {
    each(|provider, kernel| {
        add_and_retract(kernel);
        merge(kernel);
        let at_two = Index::load(kernel, Some(RevisionNumber::new(2)))
            .expect("revision 2 indexes")
            .changes(kernel, &since_seed())
            .expect("answers");
        let at_three = Index::load(kernel, None)
            .expect("revision 3 indexes")
            .changes(kernel, &since_seed())
            .expect("answers");
        assert_eq!(
            retracted_subject(&at_two.bytes),
            Value::String(DUPLICATE.to_owned()),
            "{provider}"
        );
        assert_eq!(
            retracted_subject(&at_three.bytes),
            retracted_subject(&at_two.bytes),
            "{provider}: revision 2's change, read at revision 3, names the node the merge \
             rewrote the subject to"
        );
    });
}
