//! Adversary pass 2, wave 2026-10-21b, unit U1: profile v4 (`ekr.p4-deterministic/1`,
//! `story:supersession-verdict-fixes-replay-safe`, design § 112) at `d11b3acdf`.
//!
//! Design § 112.2 item 2: "A refusal names what the head will hold (D8)", following "a `by` the
//! same transaction supersedes to that supersession's `by`, as commit repoints the record". Once
//! 0.0.42 ships every v4 message is frozen (ADR 0102 decision 1). The cases below hold the v4
//! messages to that rule where the transaction supersedes the node a message names, and hold v1
//! to v3 to the 0.0.41 wording on the same inputs.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EdgeId, EvidenceId, NodeId, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AliasAddition, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft,
    GraphOperation, GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Runtime,
    SeedDocument, ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

const SEED: &str = include_str!("fixtures/supersession-seed.yaml");
const TEXT: &str = "group/app!18 is one merge request, tracked as one work item.";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const REASON: &str = "the two are one";
const WORK_ITEM: &str = "00000000-0000-4000-8000-000000000202";
const MERGE_REQUEST: &str = "00000000-0000-4000-8000-000000000203";
const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
const MERGE_REQUEST_TITLE: &str = "00000000-0000-4000-8000-000000000213";
const KEPT: &str = "00000000-0000-4000-8000-000000000301";
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
const PARSER: &str = "00000000-0000-4000-8000-000000000303";
const LATER: &str = "00000000-0000-4000-8000-000000000304";
const OTHER: &str = "00000000-0000-4000-8000-000000000305";
const DUPLICATE_TAGGED: &str = "00000000-0000-4000-8000-000000000501";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
    }
}

fn anchor(validation_profile: ValidationProfileV1) -> AuthorityStateV1 {
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
        validation_profile,
    }
}

/// The four profiles, labelled.
fn profiles() -> [(&'static str, AuthorityStateV1); 4] {
    let v = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(v))),
        ("v2", anchor(ValidationProfileV1::schema_evolving(v))),
        ("v3", anchor(ValidationProfileV1::identity_keeping(v))),
        ("v4", anchor(ValidationProfileV1::supersession_exact(v))),
    ]
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

fn seed() -> SeedDocument {
    let mut document = SeedDocument::from_yaml(SEED).unwrap();
    let hash = ContentHash::of_bytes(TEXT.as_bytes());
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
        .insert(hash, std::sync::Arc::new(TEXT.as_bytes().to_vec()));
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

fn supersede(node: &str, by: NodeId) -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node: id(node),
        by,
        reason: REASON.to_owned(),
        evidence: BTreeSet::from([id(STATEMENT)]),
    })
}

fn retitle(node: &str) -> GraphOperation {
    GraphOperation::UpdateProperty(PropertyMutation {
        node: id(node),
        property: id(MERGE_REQUEST_TITLE),
        values: vec![Value::String("Fix the lexer".into())],
    })
}

fn create(node: NodeId, type_id: &str, aliases: &[&str]) -> GraphOperation {
    GraphOperation::CreateNode(NodeDraft {
        id: node,
        root_id: id(ROOT),
        type_id: id(type_id),
        canonical_name: "an invented name".to_owned(),
        properties: std::collections::BTreeMap::new(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
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

fn messages(verdict: &ValidationCommandResult, code: &str) -> Vec<String> {
    refused(verdict)
        .into_iter()
        .filter(|(_, issued, _)| issued == code)
        .map(|(_, _, message)| message)
        .collect()
}

/// A store at revision 1 under `authority`: DUPLICATE (MergeRequest, alias `group/app!18`)
/// superseded by KEPT (WorkItem) with its TAGGED edge retired, a TAGGED edge from KEPT, LATER
/// (WorkItem, alias `tracker/18`) and OTHER (MergeRequest) live.
fn store(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    let kernel = if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
    .unwrap();
    kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    let verdict = submit(
        &kernel,
        &transaction(
            vec![
                supersede(DUPLICATE, id(KEPT)),
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
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    kernel
}

/// Runs `cases` against a fresh store of every provider under `authority`.
fn each_store(authority: &AuthorityStateV1, mut run: impl FnMut(&str, &Runtime)) {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = store(directory.path(), file, authority.clone());
        run(if file { "file" } else { "sqlite" }, &kernel);
    }
}

fn v4() -> AuthorityStateV1 {
    profiles()[3].1.clone()
}

/// D8 under v4, `alias-already-exists` for an alias a committed-superseded node holds: when the
/// same transaction supersedes the holder's `by` (KEPT by LATER, which `Reference` admits),
/// commit repoints DUPLICATE's record to LATER, so the alias resolves to LATER once the
/// transaction commits. The refusal must not tell the proposer the alias resolves to KEPT and to
/// use KEPT: `node-superseded` in the very same rejection says "name node LATER instead", and an
/// operation naming KEPT beside this supersession is `superseded-node-named-in-transaction`.
#[test]
fn v4_alias_held_by_a_node_whose_by_this_transaction_supersedes_names_where_it_will_resolve() {
    each_store(&v4(), |at, kernel| {
        let created = NodeId::mint();
        let cases: Vec<(&str, GraphTransaction)> = vec![
            (
                "AddAlias beside KEPT by LATER and an update of DUPLICATE",
                transaction(
                    vec![
                        supersede(KEPT, id(LATER)),
                        retitle(DUPLICATE),
                        GraphOperation::AddAlias(AliasAddition {
                            node: id(OTHER),
                            alias: "group/app!18".into(),
                        }),
                    ],
                    &[id(STATEMENT)],
                ),
            ),
            (
                "CreateNode beside KEPT by LATER",
                transaction(
                    vec![
                        supersede(KEPT, id(LATER)),
                        create(created, MERGE_REQUEST, &["group/app!18"]),
                    ],
                    &[id(STATEMENT)],
                ),
            ),
        ];
        let mut wrong = Vec::new();
        for (offset, (what, tx)) in cases.into_iter().enumerate() {
            let verdict = submit(kernel, &tx, 2_000 + 10 * i64::try_from(offset).unwrap());
            let found = messages(&verdict, "alias-already-exists");
            if found.len() != 1 {
                wrong.push(format!(
                    "{at}: {what}: expected one alias-already-exists: {verdict:?}"
                ));
                continue;
            }
            let siblings = messages(&verdict, "node-superseded");
            if found[0].contains(&format!("resolves to node {KEPT}"))
                || !found[0].contains(&format!("node {LATER}"))
            {
                wrong.push(format!(
                    "{at}: {what}:\n  alias-already-exists: {:?}\n  node-superseded beside it: {siblings:?}",
                    found[0]
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "v4 says an alias resolves to a node the same transaction supersedes:\n{}",
            wrong.join("\n")
        );
    });
}

/// D8 under v4, `supersede-by-superseded-node` beside a supersession that closes a cycle: OTHER
/// by DUPLICATE (DUPLICATE superseded by KEPT) is refused, and KEPT by OTHER is admitted. F2's fix
/// falls back to the 0.0.41 wording, which advises "supersede node OTHER by node KEPT" — a node
/// this same transaction supersedes, by OTHER. Following the advice gives OTHER by KEPT beside
/// KEPT by OTHER, which v4 refuses twice as `superseded-node-named-in-transaction`.
#[test]
fn v4_supersede_by_superseded_node_does_not_advise_a_node_this_transaction_supersedes() {
    each_store(&v4(), |at, kernel| {
        let tx = transaction(
            vec![supersede(OTHER, id(DUPLICATE)), supersede(KEPT, id(OTHER))],
            &[id(STATEMENT)],
        );
        let verdict = submit(kernel, &tx, 3_000);
        let found = messages(&verdict, "supersede-by-superseded-node");
        assert_eq!(found.len(), 1, "{at}: {verdict:?}");
        let advised = transaction(
            vec![supersede(OTHER, id(KEPT)), supersede(KEPT, id(OTHER))],
            &[id(STATEMENT)],
        );
        let followed = submit(kernel, &advised, 3_010);
        assert!(
            !found[0].ends_with(&format!("supersede node {OTHER} by node {KEPT}")),
            "{at}: v4 advises superseding by KEPT, which this transaction supersedes by OTHER: \
             {:?}\n  the advice followed is refused: {:?}",
            found[0],
            refused(&followed)
        );
    });
}

/// D8 under v4, `node-superseded` beside a chain the same transaction makes: KEPT by LATER and
/// LATER by NEW (created here) are both admitted. The refusal of an update of DUPLICATE (superseded
/// by KEPT) follows one step and says "name node LATER instead", though LATER is superseded by
/// this transaction too; the chain's own refusal advises superseding both by the node kept (NEW),
/// after which DUPLICATE's record names NEW.
#[test]
fn v4_node_superseded_does_not_name_a_node_this_transaction_supersedes() {
    each_store(&v4(), |at, kernel| {
        let new = NodeId::mint();
        let tx = transaction(
            vec![
                create(new, WORK_ITEM, &[]),
                supersede(LATER, new),
                supersede(KEPT, id(LATER)),
                retitle(DUPLICATE),
            ],
            &[id(STATEMENT)],
        );
        let verdict = submit(kernel, &tx, 4_000);
        let found = messages(&verdict, "node-superseded");
        assert_eq!(found.len(), 1, "{at}: {verdict:?}");
        assert!(
            !found[0].ends_with(&format!("name node {LATER} instead")),
            "{at}: v4 names LATER, which this transaction supersedes by {new}: {:?}\n  all: {:?}",
            found[0],
            refused(&verdict)
        );
    });
}

/// D8 under v4, `node-superseded` of an edge retired with DUPLICATE when the same transaction
/// supersedes DUPLICATE's `by` (KEPT by LATER): the message follows KEPT to LATER (kernel.yaml
/// `ekr.kernel.NodeSupersession`, design § 112.2 item 2). No other case reaches that branch of
/// `Known::edge` (`reference.rs:527-532`): the mutant that always takes `(by, String::new())`
/// leaves every other test green. Frozen once released, so pinned literally.
#[test]
fn v4_an_edge_retired_with_a_node_whose_by_this_transaction_supersedes_names_the_kept_node() {
    each_store(&v4(), |at, kernel| {
        let tx = transaction(
            vec![
                supersede(KEPT, id(LATER)),
                GraphOperation::DeleteEdge(id(DUPLICATE_TAGGED)),
            ],
            &[id(STATEMENT)],
        );
        let verdict = submit(kernel, &tx, 5_000);
        assert_eq!(
            refused(&verdict),
            vec![(
                ValidatorName::Reference,
                "node-superseded".to_owned(),
                format!(
                    "edge {DUPLICATE_TAGGED} was retired with node {DUPLICATE}, which is \
                     superseded by node {KEPT}, which this transaction supersedes by node \
                     {LATER}; a superseded node's edges are retired, not moved: create the edge \
                     on node {LATER} and name that one instead"
                ),
            )],
            "{at}"
        );
    });
}

/// ADR 0102 decision 1 on this pass's inputs: v1, v2 and v3 give each the issue list 0.0.41
/// gives, messages written from the 0.0.41 format strings (`git show 6e687d99:` of
/// `validate/reference.rs`, `validate/structural.rs`, `validate/supersession.rs`).
#[test]
fn v1_to_v3_answer_this_pass_s_inputs_as_released() {
    let s = |validator, code: &str, message: String| (validator, code.to_owned(), message);
    let chain = |node: &str, by: &str| {
        s(
            ValidatorName::Structural,
            "superseded-node-named-in-transaction",
            format!(
                "node {node} is superseded by node {by}, which the same transaction supersedes; \
                 supersede both by the node that is kept"
            ),
        )
    };
    for (profile, authority) in profiles().into_iter().take(3) {
        each_store(&authority, |at, kernel| {
            let at = format!("{profile} {at}");
            let new = NodeId::mint();
            #[allow(clippy::type_complexity)]
            let cases: Vec<(GraphTransaction, Vec<(ValidatorName, String, String)>)> = vec![
                (
                    transaction(
                        vec![
                            supersede(KEPT, id(LATER)),
                            retitle(DUPLICATE),
                            GraphOperation::AddAlias(AliasAddition {
                                node: id(OTHER),
                                alias: "group/app!18".into(),
                            }),
                        ],
                        &[id(STATEMENT)],
                    ),
                    vec![
                        s(
                            ValidatorName::Structural,
                            "alias-already-exists",
                            format!(
                                "node {OTHER} is given alias \"group/app!18\", which node \
                                 {DUPLICATE} of the same type {MERGE_REQUEST} already holds; one \
                                 alias identifies one node of a type"
                            ),
                        ),
                        s(
                            ValidatorName::Reference,
                            "node-superseded",
                            format!(
                                "node {DUPLICATE} is superseded by node {KEPT}; name node {KEPT} \
                                 instead"
                            ),
                        ),
                    ],
                ),
                (
                    transaction(
                        vec![supersede(OTHER, id(DUPLICATE)), supersede(KEPT, id(OTHER))],
                        &[id(STATEMENT)],
                    ),
                    vec![
                        chain(KEPT, OTHER),
                        s(
                            ValidatorName::Reference,
                            "supersede-by-superseded-node",
                            format!(
                                "node {OTHER} is superseded by node {DUPLICATE}, which is itself \
                                 superseded by node {KEPT}; supersede node {OTHER} by node {KEPT}"
                            ),
                        ),
                    ],
                ),
                (
                    transaction(
                        vec![
                            create(new, WORK_ITEM, &[]),
                            supersede(LATER, new),
                            supersede(KEPT, id(LATER)),
                            retitle(DUPLICATE),
                        ],
                        &[id(STATEMENT)],
                    ),
                    vec![
                        chain(KEPT, LATER),
                        s(
                            ValidatorName::Reference,
                            "node-superseded",
                            format!(
                                "node {DUPLICATE} is superseded by node {KEPT}; name node {KEPT} \
                                 instead"
                            ),
                        ),
                    ],
                ),
                (
                    transaction(
                        vec![
                            supersede(KEPT, id(LATER)),
                            GraphOperation::DeleteEdge(id(DUPLICATE_TAGGED)),
                        ],
                        &[id(STATEMENT)],
                    ),
                    vec![s(
                        ValidatorName::Reference,
                        "node-superseded",
                        format!(
                            "edge {DUPLICATE_TAGGED} was retired with node {DUPLICATE}, which is \
                             superseded by node {KEPT}; name an edge of node {KEPT} instead"
                        ),
                    )],
                ),
            ];
            for (offset, (tx, expected)) in cases.into_iter().enumerate() {
                let verdict = submit(kernel, &tx, 6_000 + 10 * i64::try_from(offset).unwrap());
                assert_eq!(refused(&verdict), expected, "{at}: case {offset}");
            }
        });
    }
}
