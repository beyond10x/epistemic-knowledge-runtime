//! Adversary pass 1, wave 2026-10-21b, unit U1: profile v4 (`ekr.p4-deterministic/1`,
//! `story:supersession-verdict-fixes-replay-safe`, design § 112).
//!
//! Profile v4's messages are frozen once released (ADR 0102 decision 1), so a v4 message that
//! contradicts itself, or a v4 issue list that judges one transaction two ways, can only be
//! corrected by a profile v5 once it ships.
//!
//! * D8 follows a `by` "the same transaction supersedes" to that supersession's `by`. A
//!   supersession `X by X` of a held node is admitted by `candidate::admits` (`by == node`), so it
//!   enters the candidate's `superseding` map as `X -> X`, and every D8 message then says the
//!   transaction supersedes `X` by `X`, and an alias held by `X` "then resolves to node X" through
//!   a supersession that `Structural` refuses as `supersede-into-itself`.
//! * D8's `supersede-by-superseded-node` follows the kept node through a supersession of the same
//!   transaction without checking where it leads: `OTHER by DUPLICATE` (DUPLICATE superseded by
//!   KEPT) beside `KEPT by OTHER` advises "supersede node OTHER by node OTHER".
//! * D7 says a supersession `Reference` refuses retires nothing in its transaction. The merge
//!   check reads that for edges (`merge.rs:229`, `Candidate::retires`) and not for node values: its
//!   own `superseding` set (`merge.rs:198-200`) still skips every `SupersedeNode`'s node, so a merge
//!   beside a refused supersession is not judged as the same merge alone.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EdgeId, EvidenceId, NodeId, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, EntityMerge,
    GraphOperation, GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Runtime,
    SeedDocument, ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const REASON: &str = "the two are one";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
    }
}

/// The authority anchor of profile v4.
fn v4() -> AuthorityStateV1 {
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
        validation_profile: ValidationProfileV1::supersession_exact(c.validator),
    }
}

fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), v4())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), v4())
    }
    .unwrap()
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

/// `yaml` as a seed document, with the statement `text` retained as its evidence.
fn seed(yaml: &str, text: &str) -> SeedDocument {
    let mut document = SeedDocument::from_yaml(yaml).unwrap();
    let hash = ContentHash::of_bytes(text.as_bytes());
    let statement = Evidence {
        id: id(STATEMENT),
        source: EvidenceSource::HumanStatement {
            identity: Some("Runtime operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_773_273_600_000),
        confidence: Confidence::CERTAIN,
    };
    document.graph.evidence.insert(statement.id, statement);
    document
        .evidence_payloads
        .insert(hash, std::sync::Arc::new(text.as_bytes().to_vec()));
    document
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

fn transaction(operations: Vec<GraphOperation>, evidence: &[EvidenceId]) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: evidence.iter().copied().collect(),
        schema_version: None,
    }
}

fn supersede(node: NodeId, by: NodeId) -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node,
        by,
        reason: REASON.to_owned(),
        evidence: BTreeSet::from([id(STATEMENT)]),
    })
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

fn committed(kernel: &Runtime, tx: &GraphTransaction, at: i64) {
    let verdict = submit(kernel, tx, at);
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
}

fn refused(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone(), issue.message.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

/// The messages of the issues of `verdict` with `code`.
fn messages(verdict: &ValidationCommandResult, code: &str) -> Vec<String> {
    refused(verdict)
        .into_iter()
        .filter(|(_, issued, _)| issued == code)
        .map(|(_, _, message)| message)
        .collect()
}

/// The supersession fixture of `supersede_node.rs`: KEPT (a WorkItem) holds, DUPLICATE (a
/// MergeRequest, alias `group/app!18`) is superseded by it with its TAGGED edge retired, LATER (a
/// WorkItem, alias `tracker/18`) and OTHER (a MergeRequest) are live.
mod supersession {
    use super::*;

    pub const SEED: &str = include_str!("fixtures/supersession-seed.yaml");
    const TEXT: &str = "group/app!18 is one merge request, tracked as one work item.";
    pub const PARSER: &str = "00000000-0000-4000-8000-000000000303";
    pub const WORK_ITEM: &str = "00000000-0000-4000-8000-000000000202";
    pub const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
    pub const MERGE_REQUEST_TITLE: &str = "00000000-0000-4000-8000-000000000213";
    pub const KEPT: &str = "00000000-0000-4000-8000-000000000301";
    pub const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
    pub const LATER: &str = "00000000-0000-4000-8000-000000000304";
    pub const OTHER: &str = "00000000-0000-4000-8000-000000000305";
    pub const DUPLICATE_TAGGED: &str = "00000000-0000-4000-8000-000000000501";

    /// A v4 store at revision 1: DUPLICATE superseded by KEPT, and a TAGGED edge from KEPT.
    pub fn store(path: &std::path::Path, file: bool) -> Runtime {
        let kernel = open(path, file);
        kernel
            .seed(seed(SEED, TEXT), || Timestamp::from_millis(10))
            .unwrap();
        committed(
            &kernel,
            &transaction(
                vec![
                    supersede(id(DUPLICATE), id(KEPT)),
                    GraphOperation::CreateEdge(EdgeDraft {
                        id: EdgeId::mint(),
                        root_id: id(ROOT),
                        type_id: id(TAGGED),
                        source: id(KEPT),
                        target: id(PARSER),
                        properties: std::collections::BTreeMap::new(),
                    }),
                ],
                &[id(STATEMENT)],
            ),
            30,
        );
        kernel
    }

    pub fn retitle(node: NodeId) -> GraphOperation {
        GraphOperation::UpdateProperty(PropertyMutation {
            node,
            property: id(MERGE_REQUEST_TITLE),
            values: vec![Value::String("Fix the lexer".into())],
        })
    }

    pub fn work_item(node: NodeId, alias: &str) -> GraphOperation {
        GraphOperation::CreateNode(NodeDraft {
            id: node,
            root_id: id(ROOT),
            type_id: id(WORK_ITEM),
            canonical_name: alias.to_owned(),
            properties: std::collections::BTreeMap::new(),
            aliases: vec![alias.to_owned()],
        })
    }
}

/// D8 under v4: no refusal names a supersession of a node by itself — which `Structural` refuses
/// as `supersede-into-itself` — as the node the head will hold. Each of the four messages that
/// follow a supersession of the same transaction is checked against `X by X` of a held node.
#[test]
fn v4_a_supersession_into_itself_is_not_named_as_what_the_head_will_hold() {
    use supersession::*;
    for file in [false, true] {
        let at = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let kernel = store(directory.path(), file);
        let statement: EvidenceId = id(STATEMENT);
        let created = NodeId::mint();
        let into_itself = format!("node {KEPT}, which this transaction supersedes by node {KEPT}");
        // What, the transaction, the code whose message is read, and what it must not say.
        let cases: Vec<(&str, GraphTransaction, &str, String)> = vec![
            (
                "node-superseded of a node superseded by KEPT, beside KEPT by KEPT",
                transaction(
                    vec![supersede(id(KEPT), id(KEPT)), retitle(id(DUPLICATE))],
                    &[statement],
                ),
                "node-superseded",
                into_itself.clone(),
            ),
            (
                "supersede-by-superseded-node by a node superseded by KEPT, beside KEPT by KEPT",
                transaction(
                    vec![
                        supersede(id(OTHER), id(DUPLICATE)),
                        supersede(id(KEPT), id(KEPT)),
                    ],
                    &[statement],
                ),
                "supersede-by-superseded-node",
                into_itself.clone(),
            ),
            (
                "node-superseded of an edge retired with a node superseded by KEPT, beside KEPT \
                 by KEPT",
                transaction(
                    vec![
                        supersede(id(KEPT), id(KEPT)),
                        GraphOperation::DeleteEdge(id(DUPLICATE_TAGGED)),
                    ],
                    &[statement],
                ),
                "node-superseded",
                into_itself.clone(),
            ),
            (
                "alias-already-exists for LATER's alias, beside LATER by LATER",
                transaction(
                    vec![
                        supersede(id(LATER), id(LATER)),
                        work_item(created, "tracker/18"),
                    ],
                    &[statement],
                ),
                "alias-already-exists",
                format!("this transaction supersedes node {LATER} by node {LATER}"),
            ),
        ];
        let mut wrong = Vec::new();
        for (offset, (what, tx, code, forbidden)) in cases.into_iter().enumerate() {
            let verdict = submit(&kernel, &tx, 2_000 + 10 * i64::try_from(offset).unwrap());
            let found = messages(&verdict, code);
            if found.len() != 1 {
                wrong.push(format!("{at}: {what}: expected one {code}: {verdict:?}"));
                continue;
            }
            if found[0].contains(&forbidden) {
                wrong.push(format!("{at}: {what}: {code}: {:?}", found[0]));
            }
        }
        assert!(
            wrong.is_empty(),
            "v4 names a supersession into itself as what the head will hold:\n{}",
            wrong.join("\n")
        );
    }
}

/// D8 under v4: `supersede-by-superseded-node` never advises superseding a node by itself. OTHER
/// by DUPLICATE (superseded by KEPT) is refused; KEPT by OTHER in the same transaction is admitted;
/// following KEPT to OTHER, the message advises "supersede node OTHER by node OTHER", which is
/// `supersede-into-itself`.
#[test]
fn v4_supersede_by_superseded_node_never_advises_superseding_a_node_by_itself() {
    use supersession::*;
    for file in [false, true] {
        let at = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let kernel = store(directory.path(), file);
        let tx = transaction(
            vec![
                supersede(id(OTHER), id(DUPLICATE)),
                supersede(id(KEPT), id(OTHER)),
            ],
            &[id(STATEMENT)],
        );
        let verdict = submit(&kernel, &tx, 4_000);
        let found = messages(&verdict, "supersede-by-superseded-node");
        assert_eq!(found.len(), 1, "{at}: {verdict:?}");
        assert!(
            !found[0].contains(&format!("supersede node {OTHER} by node {OTHER}")),
            "{at}: v4 advises a supersession into itself: {:?}",
            found[0]
        );
    }
}

/// The public `Pipeline::supersession_exact(..).validate` — which no other test calls — gives the
/// v4 verdict a v4 store gives: D7, the update of a node no revision holds beside its refused
/// supersession, is refused only for what it is. The mutant `Pipeline::validate` building its
/// candidate as `Supersessions::Released` computes exactly what `Pipeline::identity_keeping`
/// computes, so the second assertion shows this case tells the two apart.
#[test]
fn the_public_v4_pipeline_gives_the_v4_verdict() {
    use supersession::*;
    let directory = tempfile::tempdir().unwrap();
    let kernel = store(directory.path(), false);
    let unknown = NodeId::mint();
    let tx = transaction(
        vec![supersede(unknown, id(KEPT)), retitle(unknown)],
        &[id(STATEMENT)],
    );
    let in_store = submit(&kernel, &tx, 5_000);
    let graph = kernel.snapshot().unwrap();
    let snapshot = ekr_graph::GraphSnapshot::of(&graph);
    let validator = context().validator;
    let codes = |pipeline: ekr_kernel::Pipeline| -> Vec<(ValidatorName, String)> {
        pipeline
            .validate(&snapshot, &tx)
            .expect_err("refused")
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect()
    };
    let held = || ekr_kernel::validate::HeldIdentities::of([&graph]);
    let exact = codes(ekr_kernel::Pipeline::supersession_exact(
        validator,
        BTreeSet::new(),
        held(),
    ));
    let store_codes: Vec<(ValidatorName, String)> = refused(&in_store)
        .into_iter()
        .map(|(validator, code, _)| (validator, code))
        .collect();
    assert_eq!(exact, store_codes, "the public v4 pipeline and a v4 store");
    assert_ne!(
        codes(ekr_kernel::Pipeline::identity_keeping(
            validator,
            BTreeSet::new(),
            held(),
        )),
        exact,
        "the mutant's verdict is told apart"
    );
}

/// The merge fixture of `merge_entity.rs`: REQUEST is a MergeRequest (a child of WorkItem), KEPT
/// and LATER are WorkItems, and a WorkItem's `follows` admits a NodeRef to a MergeRequest only.
mod merging {
    use super::*;

    pub const SEED: &str = include_str!("fixtures/merge-seed.yaml");
    const TEXT: &str = "group/app!18 was imported twice; import/18 is group/app!18.";
    pub const FOLLOWS: &str = "00000000-0000-4000-8000-000000000215";
    pub const KEPT: &str = "00000000-0000-4000-8000-000000000301";
    pub const LATER: &str = "00000000-0000-4000-8000-000000000304";
    pub const REQUEST: &str = "00000000-0000-4000-8000-000000000306";

    pub fn store(path: &std::path::Path, file: bool) -> Runtime {
        let kernel = open(path, file);
        kernel
            .seed(seed(SEED, TEXT), || Timestamp::from_millis(10))
            .unwrap();
        kernel
    }

    pub fn merge(absorbed: &str, into: &str) -> GraphOperation {
        GraphOperation::MergeEntity(EntityMerge {
            absorbed: id(absorbed),
            into: id(into),
            reason: Some(REASON.to_owned()),
            evidence: Some(BTreeSet::from([id(STATEMENT)])),
        })
    }
}

/// D7 under v4, read by the merge check: a supersession `Reference` refuses retires nothing in its
/// transaction, so a merge beside it is judged as the same merge alone. LATER's `follows` names
/// REQUEST; merging REQUEST into KEPT (a WorkItem) leaves `follows` naming a node it does not
/// admit (`wrong-type`). Beside `LATER by <unknown>`, which is `supersede-by-unknown-node`, LATER
/// is not retired, and the same `wrong-type` is owed; the merge check's own set of superseded nodes
/// still skips LATER's values.
#[test]
fn v4_a_merge_beside_a_refused_supersession_is_judged_as_the_merge_alone() {
    use merging::*;
    for file in [false, true] {
        let at = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let kernel = store(directory.path(), file);
        committed(
            &kernel,
            &transaction(
                vec![GraphOperation::UpdateProperty(PropertyMutation {
                    node: id(LATER),
                    property: id(FOLLOWS),
                    values: vec![Value::NodeRef(id(REQUEST))],
                })],
                &[],
            ),
            20,
        );
        let statement: EvidenceId = id(STATEMENT);
        let unknown = NodeId::mint();
        let codes = |verdict: &ValidationCommandResult| -> Vec<(ValidatorName, String)> {
            let mut codes: Vec<_> = refused(verdict)
                .into_iter()
                .map(|(validator, code, _)| (validator, code))
                .collect();
            codes.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
            codes
        };
        let alone = submit(
            &kernel,
            &transaction(vec![merge(REQUEST, KEPT)], &[statement]),
            100,
        );
        let refused_alone = submit(
            &kernel,
            &transaction(vec![supersede(id(LATER), unknown)], &[statement]),
            200,
        );
        let both = submit(
            &kernel,
            &transaction(
                vec![supersede(id(LATER), unknown), merge(REQUEST, KEPT)],
                &[statement],
            ),
            300,
        );
        let mut expected = codes(&alone);
        expected.extend(codes(&refused_alone));
        expected.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        assert!(
            expected.contains(&(ValidatorName::Type, "wrong-type".to_owned())),
            "{at}: the merge alone is wrong-type: {alone:?}"
        );
        assert_eq!(
            codes(&both),
            expected,
            "{at}: the merge beside a refused supersession: {both:?}"
        );
    }
}
