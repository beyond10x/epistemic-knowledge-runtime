//! `story:evidence-attaches-to-a-held-assertion`, coordinator decision 2: a graph with no
//! attachments encodes to exactly the bytes it did before attachments existed, so every existing
//! graph, checkpoint and store keeps its roots.
//!
//! `fixtures/base-30703729-v3-no-attachments.sqlite` is a SQLite-provider store written by the
//! kernel at `30703729` (the wave extract-07 base, before `AttachEvidence` existed), under
//! validation profile v3, by [`write_base_store`] below: the seed of
//! `fixtures/attachment-base-seed.yaml` and six commits — an assertion, an evidence entry with an
//! assertion citing it, an alias, a retraction, another assertion and a supersession. The sixth
//! commit is past `REPLAY_CHECKPOINT_COMMITS`, so the store retains a replay checkpoint, whose
//! graph document is read back by the kernel of this tree.
//!
//! [`BASE_ROOTS`] are the roots that kernel recorded for each revision, and the hash of the
//! `ekr.graph-document/2` bytes it wrote for each revision's graph, transcribed from the writer's
//! output at `30703729`. This tree's kernel opens the store, reaches the same head, and for every
//! revision replays a graph whose knowledge and evidence roots are the recorded ones and whose
//! graph document is byte for byte the one the base wrote.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, Runtime, SeedDocument,
    ValidationCommandResult, ValidationProfileV1,
};
use ekr_store::{evidence_root, knowledge_root, GraphDocument};

const BASE_SQLITE_STORE: &[u8] =
    include_bytes!("fixtures/base-30703729-v3-no-attachments.sqlite");

/// For each revision of the base store, in order: its knowledge root, its evidence root and the
/// content hash of its graph's `ekr.graph-document/2` bytes, as the base kernel wrote them.
const BASE_ROOTS: [(&str, &str, &str); 7] = include!("fixtures/base-30703729-v3-no-attachments.roots");

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
    }
}

fn v3() -> AuthorityStateV1 {
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
        validation_profile: ValidationProfileV1::identity_keeping(c.validator),
    }
}

/// The six transactions the base store commits, in order, as `ekr.transaction-document/2`.
fn history() -> Vec<String> {
    const ROOT: &str = "00000000-0000-4000-8000-000000000002";
    const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
    const CEO_OF: &str = "00000000-0000-4000-8000-000000000203";
    let assertion = |id: &str, subject: &str, from: i64, evidence: &str| {
        format!(
            "  - !AddAssertion
    id: {id}
    root_id: {ROOT}
    subject: !Node {subject}
    predicate: !Relation {CEO_OF}
    object: !Node 00000000-0000-4000-8000-000000000303
    evidence:
    - {evidence}
    proposed_by: {OPERATOR}
    assessment: Proposed
    lifecycle: Active
    valid_time:
      from: {from}
      to: null
    transaction_time:
      recorded_from: 0
      recorded_to: null
"
        )
    };
    let document = |id: &str, operations: &str, evidence: &[&str]| {
        let evidence = if evidence.is_empty() {
            " []\n".to_owned()
        } else {
            evidence
                .iter()
                .map(|id| format!("\n  - {id}"))
                .collect::<String>()
                + "\n"
        };
        format!(
            "format: ekr.transaction-document/2
transaction:
  id: {id}
  proposer: {OPERATOR}
  operations:
{operations}  evidence:{evidence}"
        )
    };
    let alice = "00000000-0000-4000-8000-000000000301";
    let bob = "00000000-0000-4000-8000-000000000302";
    vec![
        document(
            "00000000-0000-4000-8000-000000000a01",
            &assertion(
                "00000000-0000-4000-8000-000000000b01",
                alice,
                1_577_836_800_000,
                "00000000-0000-4000-8000-000000000401",
            ),
            &["00000000-0000-4000-8000-000000000401"],
        ),
        document(
            "00000000-0000-4000-8000-000000000a02",
            &(String::from(
                "  - !AddEvidence
    evidence:
      id: 00000000-0000-4000-8000-000000000403
      source: !HumanStatement
        identity: Runtime operator
      content_hash: d665088b6d8d615784418d2e9e79245f5aad71d0565a60fd45ed4649cb8c425c
      extracted_by: 00000000-0000-4000-8000-000000000101
      observed_at: 1773273600000
      confidence: 10000
    payload: [66, 111, 98, 32, 105, 115, 32, 67, 69, 79, 32, 111, 102, 32, 65, 99, 109, 101, 46]
",
            ) + &assertion(
                "00000000-0000-4000-8000-000000000b02",
                bob,
                1_773_273_600_000,
                "00000000-0000-4000-8000-000000000403",
            )),
            &["00000000-0000-4000-8000-000000000403"],
        ),
        document(
            "00000000-0000-4000-8000-000000000a03",
            "  - !AddAlias
    node: 00000000-0000-4000-8000-000000000301
    alias: the-founder
",
            &[],
        ),
        document(
            "00000000-0000-4000-8000-000000000a04",
            "  - !RetractAssertion
    assertion: 00000000-0000-4000-8000-000000000b02
    reason: the statement was withdrawn
",
            &[],
        ),
        document(
            "00000000-0000-4000-8000-000000000a05",
            &assertion(
                "00000000-0000-4000-8000-000000000b03",
                bob,
                1_780_000_000_000,
                "00000000-0000-4000-8000-000000000402",
            ),
            &["00000000-0000-4000-8000-000000000402"],
        ),
        document(
            "00000000-0000-4000-8000-000000000a06",
            &(assertion(
                "00000000-0000-4000-8000-000000000b04",
                alice,
                1_790_000_000_000,
                "00000000-0000-4000-8000-000000000401",
            ) + "  - !SupersedeAssertion
    assertion: 00000000-0000-4000-8000-000000000b01
    by: 00000000-0000-4000-8000-000000000b04
    effective_from: 1790000000000
"),
            &["00000000-0000-4000-8000-000000000401"],
        ),
    ]
}

/// Each revision of `kernel` as `(knowledge root, evidence root, graph document hash)`, each
/// replayed graph's roots held to the root its commit or seed recorded.
fn roots(kernel: &Runtime) -> Vec<(String, String, String)> {
    let head = kernel.head().unwrap().unwrap();
    (0..=head.revision.get())
        .map(|number| {
            let read = kernel.read(Some(RevisionNumber::new(number))).unwrap();
            let graph = &read.graph;
            assert_eq!(knowledge_root(graph), read.root.knowledge_root, "{number}");
            assert_eq!(evidence_root(graph), read.root.evidence_root, "{number}");
            assert_eq!(
                knowledge_root(&kernel.replay(RevisionNumber::new(number)).unwrap()),
                read.root.knowledge_root,
                "{number}"
            );
            let bytes = GraphDocument::of(graph).to_bytes().unwrap();
            (
                read.root.knowledge_root.to_string(),
                read.root.evidence_root.to_string(),
                ContentHash::of_bytes(&bytes).to_string(),
            )
        })
        .collect()
}

/// Writes the base store into the directory `EKR_WRITE_BASE_STORE` names and prints its roots in
/// the form [`BASE_ROOTS`] is transcribed in. Run once, by hand, at the base:
/// `EKR_WRITE_BASE_STORE=<dir> cargo test -p ekr-kernel --test attachment_base_store -- --ignored
/// --nocapture write_base_store`.
#[test]
#[ignore = "writes the base fixture; run by hand at the base commit"]
fn write_base_store() {
    let directory = std::path::PathBuf::from(
        std::env::var("EKR_WRITE_BASE_STORE").expect("EKR_WRITE_BASE_STORE names a directory"),
    );
    let kernel = Runtime::sqlite(&directory.join("state.db"), "test", context(), v3()).unwrap();
    let seed = SeedDocument::from_yaml(include_str!("fixtures/attachment-base-seed.yaml")).unwrap();
    kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();
    for (offset, document) in history().iter().enumerate() {
        let at = 100 * (i64::try_from(offset).unwrap() + 1);
        let proposed = kernel
            .propose(document.as_bytes(), context().operator, || {
                Timestamp::from_millis(at)
            })
            .unwrap();
        let id: TransactionId = proposed.transaction_id;
        let head = kernel.head().unwrap().unwrap().revision;
        let verdict = kernel
            .validate(id, head, || Timestamp::from_millis(at + 1))
            .unwrap();
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{offset}: {verdict:?}"
        );
        let committed = kernel
            .commit(id, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
    }
    let written = roots(&kernel);
    drop(kernel);
    let reopened = Runtime::sqlite(&directory.join("state.db"), "test", context(), v3()).unwrap();
    assert_eq!(roots(&reopened), written);
    println!("[");
    for (knowledge, evidence, document) in written {
        println!("    (\n        \"{knowledge}\",\n        \"{evidence}\",\n        \"{document}\",\n    ),");
    }
    println!("]");
}

/// The store the base kernel wrote opens under this tree's kernel — from its replay checkpoint,
/// and again with a full replay from the seed — at the same head, and every revision's knowledge
/// root, evidence root and graph document is the one the base recorded.
#[test]
fn a_store_written_before_attachments_keeps_every_recorded_root_and_graph_document() {
    let expected: Vec<(String, String, String)> = BASE_ROOTS
        .iter()
        .map(|(k, e, d)| ((*k).to_owned(), (*e).to_owned(), (*d).to_owned()))
        .collect();
    for full in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("state.db"), BASE_SQLITE_STORE).unwrap();
        let mut kernel =
            Runtime::sqlite(&directory.path().join("state.db"), "test", context(), v3())
                .expect("the base store opens");
        kernel.set_full_replay(full);
        let head = kernel.head().unwrap().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(6), "full replay {full}");
        assert_eq!(
            head.knowledge_root.to_string(),
            expected[6].0,
            "full replay {full}"
        );
        assert_eq!(roots(&kernel), expected, "full replay {full}");
        assert!(
            kernel
                .snapshot()
                .unwrap()
                .attachments
                .is_empty(),
            "a store written before attachments holds none"
        );
    }
}
