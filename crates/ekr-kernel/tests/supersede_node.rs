//! `story:supersede-node`: a duplicate node is superseded by the node that replaces it
//! (`architecture-decision-record:0017-a-node-is-superseded-by-another`), on both providers and
//! under every validation profile.
//!
//! A consumer's ingestion wrote a `MergeRequest` beside the `WorkItem` the store already held for
//! the tracker key `group/app!18`. `!SupersedeNode {node, by, reason, evidence}` retires the
//! duplicate in its own revision: the node and every edge at it leave the head and are held,
//! whole, in a supersession record naming the kept node, the reason and the evidence; the
//! assertions about it keep their bytes; its aliases stay taken and resolve to the kept node; and
//! every earlier revision still reads it.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId,
};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, Confidence, Evidence, EvidenceSource, Root, SupersededNode,
};
use ekr_kernel::{
    Agent, AliasAddition, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft,
    GraphOperation, GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Retraction,
    Runtime, SeedDocument, ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

/// The seed: the node types Tag, WorkItem and MergeRequest and the edge type TAGGED (a work item
/// or merge request to a tag), five nodes and one edge from the duplicate to its tag.
const SEED: &str = include_str!("fixtures/supersession-seed.yaml");
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const TAG: &str = "00000000-0000-4000-8000-000000000201";
const MERGE_REQUEST: &str = "00000000-0000-4000-8000-000000000203";
const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
/// `title` of a MergeRequest.
const MERGE_REQUEST_TITLE: &str = "00000000-0000-4000-8000-000000000213";
/// `note` of a TAGGED edge.
const NOTE: &str = "00000000-0000-4000-8000-000000000212";
/// The WorkItem the store held first: the node kept.
const KEPT: &str = "00000000-0000-4000-8000-000000000301";
/// The MergeRequest the ingestion wrote beside it: the duplicate, alias `group/app!18`.
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
/// The Tag both are tagged with.
const PARSER: &str = "00000000-0000-4000-8000-000000000303";
/// A second WorkItem, alias `tracker/18`, that the kept node is later superseded by.
const LATER: &str = "00000000-0000-4000-8000-000000000304";
/// A MergeRequest without an alias.
const OTHER: &str = "00000000-0000-4000-8000-000000000305";
/// The seeded TAGGED edge from the duplicate to the tag.
const DUPLICATE_TAGGED: &str = "00000000-0000-4000-8000-000000000501";
/// The seeded evidence entry, a statement that the two are one.
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const STATEMENT_TEXT: &str = "group/app!18 is one merge request, tracked as one work item.";
const REASON: &str = "the merge request duplicates the work item group/app!18";

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

/// The three validation profiles: a supersession is data, not a schema change, so each applies
/// it.
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

/// The seed document, with the statement retained as its evidence.
fn seed() -> SeedDocument {
    let mut document = SeedDocument::from_yaml(SEED).unwrap();
    let hash = ContentHash::of_bytes(STATEMENT_TEXT.as_bytes());
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
    document.evidence_payloads.insert(
        hash,
        std::sync::Arc::new(STATEMENT_TEXT.as_bytes().to_vec()),
    );
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

/// `!SupersedeNode` of `node` by `by`, for the reason and on the statement.
fn supersede(node: NodeId, by: NodeId) -> GraphOperation {
    supersede_with(node, by, REASON, &[id(STATEMENT)])
}

fn supersede_with(
    node: NodeId,
    by: NodeId,
    reason: &str,
    evidence: &[EvidenceId],
) -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node,
        by,
        reason: reason.to_owned(),
        evidence: evidence.iter().copied().collect(),
    })
}

/// A property assertion about `subject` (a `!Node` or `!Edge` YAML subject), citing the
/// statement.
fn claim(subject: &str, property: &str, value: &str) -> (AssertionId, GraphOperation) {
    let assertion = AssertionId::mint();
    let yaml = format!(
        "id: {assertion}
root_id: {ROOT}
subject: {subject}
predicate: !Property {property}
object: !Value
  value_kind: String
  value: {value:?}
evidence:
- {STATEMENT}
proposed_by: {}
assessment: Proposed
lifecycle: Active
valid_time:
  from: 1577836800000
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

/// A TAGGED edge from `source` to `target`, under `edge`.
fn tagged(edge: EdgeId, source: NodeId, target: NodeId) -> GraphOperation {
    GraphOperation::CreateEdge(EdgeDraft {
        id: edge,
        root_id: id(ROOT),
        type_id: id(TAGGED),
        source,
        target,
        properties: BTreeMap::new(),
    })
}

/// A node of `type_id` named `name` with `aliases`.
fn node(node: NodeId, type_id: &str, name: &str, aliases: &[&str]) -> GraphOperation {
    GraphOperation::CreateNode(NodeDraft {
        id: node,
        root_id: id(ROOT),
        type_id: id(type_id),
        canonical_name: name.to_owned(),
        properties: BTreeMap::new(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
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

/// Each issue of a rejection as `(validator, code, message)`.
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

/// Every revision's root, as the replay of each revision computes it.
fn roots(kernel: &Runtime) -> Vec<Root> {
    let head = kernel.head().unwrap().unwrap();
    (0..=head.revision.get())
        .map(|number| kernel.read(Some(RevisionNumber::new(number))).unwrap().root)
        .collect()
}

fn graph_at(kernel: &Runtime, revision: u64) -> CanonicalGraph {
    (*kernel
        .read(Some(RevisionNumber::new(revision)))
        .unwrap()
        .graph)
        .clone()
}

/// The assertions revision 1 adds: one about the duplicate, one about its edge.
struct Claims {
    about_node: AssertionId,
    about_edge: AssertionId,
}

/// Seeds the store, commits revision 1 (an assertion about the duplicate and one about its edge)
/// and revision 2: the duplicate superseded by the kept node, with a TAGGED edge from the kept
/// node to the tag in the same transaction.
fn superseded_store(kernel: &Runtime) -> Claims {
    kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    let (about_node, add_node) = claim(
        &format!("!Node {DUPLICATE}"),
        MERGE_REQUEST_TITLE,
        "Fix the parser",
    );
    let (about_edge, add_edge) = claim(&format!("!Edge {DUPLICATE_TAGGED}"), NOTE, "imported");
    committed(
        kernel,
        &transaction(vec![add_node, add_edge], &[id(STATEMENT)]),
        20,
    );
    committed(
        kernel,
        &transaction(
            vec![
                supersede(id(DUPLICATE), id(KEPT)),
                tagged(EdgeId::mint(), id(KEPT), id(PARSER)),
            ],
            &[id(STATEMENT)],
        ),
        30,
    );
    Claims {
        about_node,
        about_edge,
    }
}

/// Every provider and profile, labelled.
fn each_store(mut run: impl FnMut(&str, &std::path::Path, bool, AuthorityStateV1)) {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            run(&at, directory.path(), file, authority.clone());
        }
    }
}

/// `a-supersession-is-its-own-revision`: the head moves by one revision, and its record names the
/// kept node, the reason and the evidence; the duplicate is held there whole, as revision 1 held
/// it; the assertions about it keep their bytes; the knowledge root moves and the evidence root
/// does not.
#[test]
fn a_supersession_is_its_own_revision() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        let claims = superseded_store(&kernel);
        let before = graph_at(&kernel, 1);
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(2), "{at}");
        assert!(before.superseded.is_empty(), "{at}");
        let duplicate: NodeId = id(DUPLICATE);
        assert!(!head.nodes.contains_key(&duplicate), "{at}");
        let record = &head.superseded[&duplicate];
        let expected = SupersededNode {
            node: before.nodes[&duplicate].clone(),
            edges: [(
                id(DUPLICATE_TAGGED),
                before.edges[&id(DUPLICATE_TAGGED)].clone(),
            )]
            .into_iter()
            .collect(),
            by: CanonicalRef::new(id(KEPT)),
            revision: RevisionNumber::new(2),
            reason: REASON.to_owned(),
            evidence: BTreeSet::from([CanonicalRef::new(id(STATEMENT))]),
            merge: None,
        };
        assert_eq!(*record, expected, "{at}");
        assert_eq!(head.superseded.len(), 1, "{at}");
        assert_eq!(head.superseded_by(duplicate), Some(id(KEPT)), "{at}");
        assert_eq!(head.superseded_by(id(KEPT)), None, "{at}");
        assert_eq!(
            head.retired_with(id(DUPLICATE_TAGGED)),
            Some((duplicate, id(KEPT))),
            "{at}"
        );
        // The one-pass index answers each retired edge as the scan does, and nothing else.
        let retired: ekr_graph::RetiredEdges<'_> = head.retired_edges();
        assert_eq!(
            retired.retired_with(id(DUPLICATE_TAGGED)),
            head.retired_with(id(DUPLICATE_TAGGED)),
            "{at}"
        );
        assert_eq!(
            retired
                .record(id(DUPLICATE_TAGGED))
                .map(|record| record.id()),
            Some(duplicate),
            "{at}"
        );
        assert_eq!(
            retired.edge(id(DUPLICATE_TAGGED)),
            Some(&before.edges[&id(DUPLICATE_TAGGED)]),
            "{at}"
        );
        let live = *head.edges.keys().next().expect("the kept node's edge");
        assert_eq!(retired.retired_with(live), None, "{at}");
        for assertion in [claims.about_node, claims.about_edge] {
            assert_eq!(
                ContentHash::of(&head.assertions[&assertion]),
                ContentHash::of(&before.assertions[&assertion]),
                "{at}: an assertion about the duplicate keeps its bytes"
            );
        }
        let recorded = roots(&kernel);
        assert_ne!(
            recorded[1].knowledge_root, recorded[2].knowledge_root,
            "{at}"
        );
        assert_eq!(recorded[1].evidence_root, recorded[2].evidence_root, "{at}");
    });
}

/// `a-superseded-nodes-edges-are-retired-with-it`: no head edge names the duplicate, its edge is
/// in the record, and the TAGGED edge from the kept node created in the same transaction is held.
#[test]
fn a_superseded_nodes_edges_are_retired_with_it() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        superseded_store(&kernel);
        let head = kernel.snapshot().unwrap();
        let duplicate: NodeId = id(DUPLICATE);
        assert!(
            head.edges
                .values()
                .all(|edge| edge.source.node() != duplicate && edge.target.node() != duplicate),
            "{at}: a head edge names the superseded node"
        );
        assert!(!head.edges.contains_key(&id(DUPLICATE_TAGGED)), "{at}");
        assert!(
            head.superseded[&duplicate]
                .edges
                .contains_key(&id(DUPLICATE_TAGGED)),
            "{at}"
        );
        let kept: NodeId = id(KEPT);
        assert_eq!(
            head.edges
                .values()
                .filter(|edge| edge.source.node() == kept && edge.target.node() == id(PARSER))
                .count(),
            1,
            "{at}: the CreateEdge from the kept node commits beside the supersession"
        );
    });
}

/// `a-superseded-node-reads-at-an-earlier-revision`: at revision 1 the duplicate is held with its
/// edge, its assertions and their evidence; at the head it is not.
#[test]
fn a_superseded_node_reads_at_an_earlier_revision() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        let claims = superseded_store(&kernel);
        let earlier = graph_at(&kernel, 1);
        let duplicate: NodeId = id(DUPLICATE);
        assert_eq!(
            earlier.nodes[&duplicate].canonical_name, "group/app!18",
            "{at}"
        );
        assert_eq!(
            earlier.edges[&id(DUPLICATE_TAGGED)].source.node(),
            duplicate,
            "{at}"
        );
        for assertion in [claims.about_node, claims.about_edge] {
            assert!(
                earlier.assertions[&assertion]
                    .evidence
                    .contains(&CanonicalRef::new(id(STATEMENT))),
                "{at}"
            );
        }
        assert!(earlier.evidence.contains_key(&id(STATEMENT)), "{at}");
        assert!(!graph_at(&kernel, 2).nodes.contains_key(&duplicate), "{at}");
    });
}

/// `superseding-the-kept-node-repoints-earlier-records`: after the kept node is itself superseded
/// by a later one, the duplicate's record names the later node, so a reader follows one pointer.
#[test]
fn superseding_the_kept_node_repoints_earlier_records() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        superseded_store(&kernel);
        committed(
            &kernel,
            &transaction(vec![supersede(id(KEPT), id(LATER))], &[id(STATEMENT)]),
            40,
        );
        let head = kernel.snapshot().unwrap();
        let later: NodeId = id(LATER);
        assert_eq!(head.superseded[&id(DUPLICATE)].by.node(), later, "{at}");
        assert_eq!(head.superseded[&id(KEPT)].by.node(), later, "{at}");
        assert_eq!(
            head.superseded[&id(DUPLICATE)].revision,
            RevisionNumber::new(2),
            "{at}: a repointed record keeps the revision that made it"
        );
        assert_eq!(
            head.superseded[&id(KEPT)].revision,
            RevisionNumber::new(3),
            "{at}"
        );
        let kept_edges = &head.superseded[&id(KEPT)].edges;
        assert_eq!(
            kept_edges.len(),
            1,
            "{at}: the kept node's edge retires with it"
        );
        assert_eq!(
            graph_at(&kernel, 2).superseded[&id(DUPLICATE)].by.node(),
            id::<NodeId>(KEPT),
            "{at}: revision 2 still names the node it named"
        );
    });
}

/// `a-superseded-alias-is-not-taken-again`: a later `CreateNode` of the duplicate's type with its
/// alias, and a later `AddAlias` of it to a node of that type, are `alias-already-exists`.
#[test]
fn a_superseded_alias_is_not_taken_again() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        superseded_store(&kernel);
        let head = kernel.head().unwrap().unwrap();
        let alias_taken = vec![(ValidatorName::Structural, "alias-already-exists".to_owned())];
        let codes = |verdict: &ValidationCommandResult| {
            refused(verdict)
                .into_iter()
                .map(|(validator, code, _)| (validator, code))
                .collect::<Vec<_>>()
        };
        let created = submit(
            &kernel,
            &transaction(
                vec![node(
                    NodeId::mint(),
                    MERGE_REQUEST,
                    "group/app!18",
                    &["group/app!18"],
                )],
                &[],
            ),
            50,
        );
        assert_eq!(codes(&created), alias_taken, "{at}: CreateNode");
        assert!(
            refused(&created)[0].2.contains(DUPLICATE),
            "{at}: the refusal names the superseded holder: {created:?}"
        );
        let added = submit(
            &kernel,
            &transaction(
                vec![GraphOperation::AddAlias(AliasAddition {
                    node: id(OTHER),
                    alias: "group/app!18".into(),
                })],
                &[],
            ),
            60,
        );
        assert_eq!(codes(&added), alias_taken, "{at}: AddAlias");
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
        // The alias under another type is not the superseded node's.
        committed(
            &kernel,
            &transaction(
                vec![node(NodeId::mint(), TAG, "group/app!18", &["group/app!18"])],
                &[],
            ),
            70,
        );
    });
}

/// `a-supersession-is-refused-by-name`: one case per refusal of ADR 0017 decision 7, each its own
/// code, none moving the head; a retraction of an assertion about the superseded node still
/// commits.
#[test]
#[allow(clippy::too_many_lines)]
fn a_supersession_is_refused_by_name() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        let claims = superseded_store(&kernel);
        let head = kernel.head().unwrap().unwrap();
        let unknown_evidence = EvidenceId::mint();
        let statement: EvidenceId = id(STATEMENT);
        let (on_superseded, add_on_superseded) = claim(
            &format!("!Node {DUPLICATE}"),
            MERGE_REQUEST_TITLE,
            "Fix the lexer",
        );
        let _ = on_superseded;
        let (on_retired, add_on_retired) =
            claim(&format!("!Edge {DUPLICATE_TAGGED}"), NOTE, "re-imported");
        let _ = on_retired;
        let (on_retiring, add_on_retiring) = claim(
            &format!("!Node {OTHER}"),
            MERGE_REQUEST_TITLE,
            "Fix the lexer",
        );
        let _ = on_retiring;
        let reference = |code: &str| (ValidatorName::Reference, code.to_owned());
        let structural = |code: &str| (ValidatorName::Structural, code.to_owned());
        type Case = (
            &'static str,
            GraphTransaction,
            Vec<(ValidatorName, String)>,
            Option<&'static str>,
        );
        let cases: Vec<Case> = vec![
            (
                "a node no revision holds",
                transaction(vec![supersede(NodeId::mint(), id(KEPT))], &[statement]),
                vec![reference("supersede-unknown-node")],
                None,
            ),
            (
                "a node already superseded",
                transaction(vec![supersede(id(DUPLICATE), id(LATER))], &[statement]),
                vec![reference("node-already-superseded")],
                Some(KEPT),
            ),
            (
                "a node superseded by itself",
                transaction(vec![supersede(id(OTHER), id(OTHER))], &[statement]),
                vec![structural("supersede-into-itself")],
                None,
            ),
            (
                "a node superseded by one no revision holds",
                transaction(vec![supersede(id(OTHER), NodeId::mint())], &[statement]),
                vec![reference("supersede-by-unknown-node")],
                None,
            ),
            (
                "a node superseded by a superseded node",
                transaction(vec![supersede(id(OTHER), id(DUPLICATE))], &[statement]),
                vec![reference("supersede-by-superseded-node")],
                Some(KEPT),
            ),
            (
                "a supersession without a reason",
                transaction(
                    vec![supersede_with(id(OTHER), id(KEPT), " \n", &[statement])],
                    &[statement],
                ),
                vec![structural("supersession-without-reason")],
                None,
            ),
            (
                "a supersession without evidence",
                transaction(vec![supersede_with(id(OTHER), id(KEPT), REASON, &[])], &[]),
                vec![structural("supersession-without-evidence")],
                None,
            ),
            (
                "a supersession on evidence no revision retains",
                transaction(
                    vec![supersede_with(
                        id(OTHER),
                        id(KEPT),
                        REASON,
                        &[unknown_evidence],
                    )],
                    &[unknown_evidence],
                ),
                vec![reference("supersession-evidence-unknown")],
                None,
            ),
            (
                "a supersession the evidence set leaves out",
                transaction(vec![supersede(id(OTHER), id(KEPT))], &[]),
                vec![structural("evidence-set-mismatch")],
                None,
            ),
            (
                "the superseded node updated in the same transaction",
                transaction(
                    vec![
                        supersede(id(OTHER), id(KEPT)),
                        GraphOperation::UpdateProperty(PropertyMutation {
                            node: id(OTHER),
                            property: id(MERGE_REQUEST_TITLE),
                            values: vec![Value::String("Fix the lexer".into())],
                        }),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "an edge to the superseded node in the same transaction",
                transaction(
                    vec![
                        supersede(id(OTHER), id(KEPT)),
                        tagged(EdgeId::mint(), id(OTHER), id(PARSER)),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "an assertion about the superseded node in the same transaction",
                transaction(
                    vec![supersede(id(OTHER), id(KEPT)), add_on_retiring],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "one node superseded twice in a transaction",
                transaction(
                    vec![
                        supersede(id(OTHER), id(KEPT)),
                        supersede(id(OTHER), id(LATER)),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "a node superseded by one the same transaction supersedes",
                transaction(
                    vec![
                        supersede(id(OTHER), id(KEPT)),
                        supersede(id(KEPT), id(LATER)),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "a later update of the superseded node",
                transaction(
                    vec![GraphOperation::UpdateProperty(PropertyMutation {
                        node: id(DUPLICATE),
                        property: id(MERGE_REQUEST_TITLE),
                        values: vec![Value::String("Fix the lexer".into())],
                    })],
                    &[],
                ),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
            (
                "a later edge to the superseded node",
                transaction(vec![tagged(EdgeId::mint(), id(DUPLICATE), id(PARSER))], &[]),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
            (
                "a later alias for the superseded node",
                transaction(
                    vec![GraphOperation::AddAlias(AliasAddition {
                        node: id(DUPLICATE),
                        alias: "group/app!18-mr".into(),
                    })],
                    &[],
                ),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
            (
                "a later assertion about the superseded node",
                transaction(vec![add_on_superseded], &[statement]),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
            (
                "a later assertion about an edge retired with it",
                transaction(vec![add_on_retired], &[statement]),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
            (
                "a later deletion of an edge retired with it",
                transaction(vec![GraphOperation::DeleteEdge(id(DUPLICATE_TAGGED))], &[]),
                vec![reference("node-superseded")],
                Some(KEPT),
            ),
        ];
        for (offset, (what, tx, expected, names)) in cases.into_iter().enumerate() {
            let verdict = submit(&kernel, &tx, 100 + 10 * i64::try_from(offset).unwrap());
            let issues = refused(&verdict);
            assert_eq!(
                issues
                    .iter()
                    .map(|(validator, code, _)| (*validator, code.clone()))
                    .collect::<Vec<_>>(),
                expected,
                "{at}: {what}: {issues:?}"
            );
            if let Some(by) = names {
                assert!(
                    issues[0].2.contains(by),
                    "{at}: {what}: the refusal names the node it is superseded by: {issues:?}"
                );
            }
        }
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");

        // A retraction of an assertion about the superseded node is not a use of it.
        committed(
            &kernel,
            &transaction(
                vec![GraphOperation::RetractAssertion(Retraction {
                    assertion: claims.about_node,
                    reason: ekr_graph::RetractionReason::new("the duplicate's title"),
                })],
                &[],
            ),
            1_000,
        );
    });
}

/// The codes of a rejection, as `(validator, code)`.
fn codes(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    refused(verdict)
        .into_iter()
        .map(|(validator, code, _)| (validator, code))
        .collect()
}

/// `story:supersede-node-review-fixes` T1: the standalone structural validator — the public
/// `Pipeline::validate`, which scans the graph for alias holders rather than reading a store's
/// lookup — counts a superseded node among them, under each profile's pipeline.
#[test]
fn the_standalone_alias_scan_counts_a_superseded_holder() {
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false, profiles()[0].1.clone());
    superseded_store(&kernel);
    let graph = kernel.snapshot().unwrap();
    let snapshot = ekr_graph::GraphSnapshot::of(&graph);
    let tx = transaction(
        vec![node(
            NodeId::mint(),
            MERGE_REQUEST,
            "group/app!18",
            &["group/app!18"],
        )],
        &[],
    );
    let validator = context().validator;
    for (profile, pipeline) in [
        ("v1", ekr_kernel::Pipeline::deterministic(validator)),
        (
            "v2",
            ekr_kernel::Pipeline::schema_evolving(validator, BTreeSet::new()),
        ),
        (
            "v3",
            ekr_kernel::Pipeline::identity_keeping(
                validator,
                BTreeSet::new(),
                ekr_kernel::validate::HeldIdentities::of([&graph]),
            ),
        ),
    ] {
        let issues = pipeline
            .validate(&snapshot, &tx)
            .expect_err("the alias is taken");
        assert_eq!(
            issues
                .iter()
                .map(|issue| (issue.validator, issue.code.as_str()))
                .collect::<Vec<_>>(),
            [(ValidatorName::Structural, "alias-already-exists")],
            "{profile}: {issues:?}"
        );
        assert!(
            issues[0].message.contains(DUPLICATE),
            "{profile}: {issues:?}"
        );
    }
}

/// `story:supersede-node-review-fixes` T2 and T3: an edge into a node, not only out of one, names
/// it — a `CreateEdge` whose target the same transaction supersedes, and an `AddAlias` of a node
/// the same transaction supersedes, are `superseded-node-named-in-transaction`; a later
/// `CreateEdge` whose target is superseded is `node-superseded`, naming its `by`.
#[test]
fn an_edge_target_and_an_alias_name_the_node_they_reach() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        superseded_store(&kernel);
        let statement: EvidenceId = id(STATEMENT);
        let kept_tag = NodeId::mint();
        let named = vec![(
            ValidatorName::Structural,
            "superseded-node-named-in-transaction".to_owned(),
        )];
        let into = submit(
            &kernel,
            &transaction(
                vec![
                    node(kept_tag, TAG, "parser, kept", &[]),
                    supersede(id(PARSER), kept_tag),
                    tagged(EdgeId::mint(), id(KEPT), id(PARSER)),
                ],
                &[statement],
            ),
            5_000,
        );
        assert_eq!(codes(&into), named, "{at}: an edge into it: {into:?}");
        let aliased = submit(
            &kernel,
            &transaction(
                vec![
                    supersede(id(OTHER), id(KEPT)),
                    GraphOperation::AddAlias(AliasAddition {
                        node: id(OTHER),
                        alias: "group/app!19".into(),
                    }),
                ],
                &[statement],
            ),
            5_010,
        );
        assert_eq!(codes(&aliased), named, "{at}: an alias of it: {aliased:?}");

        committed(
            &kernel,
            &transaction(
                vec![
                    node(kept_tag, TAG, "parser, kept", &[]),
                    supersede(id(PARSER), kept_tag),
                ],
                &[statement],
            ),
            5_020,
        );
        let later = submit(
            &kernel,
            &transaction(vec![tagged(EdgeId::mint(), id(KEPT), id(PARSER))], &[]),
            5_030,
        );
        assert_eq!(
            codes(&later),
            vec![(ValidatorName::Reference, "node-superseded".to_owned())],
            "{at}: a later edge into it: {later:?}"
        );
        assert!(
            refused(&later)[0].2.contains(&kept_tag.to_string()),
            "{at}: {later:?}"
        );
    });
}

/// `story:supersede-node-review-fixes` T4: a node the same transaction creates is not held yet,
/// so superseding it is `supersede-unknown-node`, and nothing commits.
#[test]
fn a_node_the_transaction_creates_is_not_superseded_by_it() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        superseded_store(&kernel);
        let head = kernel.head().unwrap().unwrap();
        let created = NodeId::mint();
        let verdict = submit(
            &kernel,
            &transaction(
                vec![
                    node(created, MERGE_REQUEST, "group/app!20", &[]),
                    supersede(created, id(KEPT)),
                ],
                &[id(STATEMENT)],
            ),
            6_000,
        );
        assert_eq!(
            codes(&verdict),
            vec![(
                ValidatorName::Reference,
                "supersede-unknown-node".to_owned()
            )],
            "{at}: {verdict:?}"
        );
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// `story:supersede-node-review-fixes` D9: a validation looks every retired edge up in one index
/// of the supersession records, built once, rather than scanning the records per lookup. Two
/// records, the duplicate's holding three edges with an assertion about each; a transaction
/// creating three edges, each id held against the retired ones, is validated reading each record
/// once, under every profile. Counted, not timed.
#[test]
fn a_validation_reads_each_supersession_record_once() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
        let (first, second) = (EdgeId::mint(), EdgeId::mint());
        let mut operations = vec![
            tagged(first, id(DUPLICATE), id(PARSER)),
            tagged(second, id(DUPLICATE), id(PARSER)),
        ];
        for edge in [id::<EdgeId>(DUPLICATE_TAGGED), first, second] {
            operations.push(claim(&format!("!Edge {edge}"), NOTE, "imported").1);
        }
        committed(&kernel, &transaction(operations, &[id(STATEMENT)]), 20);
        committed(
            &kernel,
            &transaction(
                vec![
                    supersede(id(DUPLICATE), id(KEPT)),
                    supersede(id(OTHER), id(KEPT)),
                ],
                &[id(STATEMENT)],
            ),
            30,
        );
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.superseded.len(), 2, "{at}");
        assert_eq!(head.superseded[&id(DUPLICATE)].edges.len(), 3, "{at}");
        let tx = transaction(
            (0..3)
                .map(|_| tagged(EdgeId::mint(), id(KEPT), id(PARSER)))
                .collect(),
            &[],
        );
        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(40)
            })
            .unwrap();
        let before = ekr_graph::supersession_records_read();
        let verdict = kernel
            .validate(tx.id, head.revision, || Timestamp::from_millis(41))
            .unwrap();
        let reads = ekr_graph::supersession_records_read() - before;
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{at}: {verdict:?}"
        );
        assert_eq!(
            reads, 2,
            "{at}: supersession records read for one validation"
        );
    });
}

/// `story:supersede-node-review-fixes` R1: an edge between two nodes one transaction supersedes
/// is retired once, into the record of its source, and the record of its target does not hold
/// it (design § 110.2, `ekr.graph.SupersededNode`).
#[test]
fn an_edge_between_two_nodes_superseded_together_is_held_by_its_sources_record() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority);
        kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
        let kept_tag = NodeId::mint();
        committed(
            &kernel,
            &transaction(
                vec![
                    node(kept_tag, TAG, "parser, kept", &[]),
                    supersede(id(DUPLICATE), id(KEPT)),
                    supersede(id(PARSER), kept_tag),
                ],
                &[id(STATEMENT)],
            ),
            20,
        );
        let head = kernel.snapshot().unwrap();
        let edge: EdgeId = id(DUPLICATE_TAGGED);
        assert!(
            head.superseded[&id(DUPLICATE)].edges.contains_key(&edge),
            "{at}: the source's record holds the edge"
        );
        assert!(
            !head.superseded[&id(PARSER)].edges.contains_key(&edge),
            "{at}: the target's record does not"
        );
    });
}

/// `a-supersession-replays-to-identical-roots`: a reopen from the replay checkpoint written after
/// the supersessions, a full replay and a migrated store each give every revision the same roots
/// and the same head graph, supersession records included.
#[test]
fn a_supersession_replays_to_identical_roots() {
    each_store(|at, path, file, authority| {
        let kernel = open(path, file, authority.clone());
        superseded_store(&kernel);
        committed(
            &kernel,
            &transaction(vec![supersede(id(KEPT), id(LATER))], &[id(STATEMENT)]),
            40,
        );
        // Revisions 4 to 6, so that revision 5 writes a replay checkpoint holding both records.
        for (n, at) in [(0_i64, 50), (1, 60), (2, 70)] {
            committed(
                &kernel,
                &transaction(
                    vec![node(NodeId::mint(), TAG, &format!("tag {n}"), &[])],
                    &[],
                ),
                at,
            );
        }
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(6), "{at}");
        assert_eq!(head.superseded.len(), 2, "{at}");
        let recorded = roots(&kernel);
        let records = kernel.transactions().unwrap();
        drop(kernel);

        let reopened = open(path, file, authority.clone());
        assert_eq!(
            roots(&reopened),
            recorded,
            "{at}: reopened from the checkpoint"
        );
        assert_eq!(reopened.snapshot().unwrap(), head, "{at}");
        assert_eq!(reopened.transactions().unwrap(), records, "{at}");
        drop(reopened);

        let mut replayed = open(path, file, authority.clone());
        replayed.set_full_replay(true);
        assert_eq!(roots(&replayed), recorded, "{at}: full replay");
        assert_eq!(replayed.snapshot().unwrap(), head, "{at}");

        let migrated_directory = tempfile::tempdir().unwrap();
        let migrated = open(migrated_directory.path(), file, authority.clone());
        replayed.migrate_into(&migrated).unwrap();
        assert_eq!(migrated.snapshot().unwrap(), head, "{at}: migrated");
        for (before, after) in recorded.iter().zip(roots(&migrated)) {
            assert_eq!(before.knowledge_root, after.knowledge_root, "{at}");
            assert_eq!(before.evidence_root, after.evidence_root, "{at}");
        }
    });
}
