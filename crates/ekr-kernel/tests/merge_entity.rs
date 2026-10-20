//! `story:merge-entity-applies`: `MergeEntity` joins duplicate nodes of one type or of related
//! types and rewrites every reference to the survivor
//! (`architecture-decision-record:0018-merge-and-split-identity`, option A), on both providers and
//! under every validation profile.
//!
//! A merge is a supersession whose references are rewritten in place: `!MergeEntity {absorbed,
//! into, reason, evidence}` retires `absorbed` into a supersession record naming `into`, as
//! `!SupersedeNode` does, and rewrites every edge end, assertion subject and object and `NodeRef`
//! value naming it to `into`, keeping every id; an edge `into` already holds absorbs its duplicate,
//! an edge joining the two is retired, and the record's `merge` lineage says what moved. The bare
//! `{absorbed, into}` shape is P1's, and keeps P1's refusal under every profile.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, PropertyId, RevisionNumber, Timestamp,
    TransactionId,
};
use ekr_graph::{
    AliasIndex, CanonicalGraph, CanonicalRef, CanonicalValue, Confidence, Evidence, EvidenceSource,
    MergeLineage, MergedEnds, Object, RewrittenAssertion, Root, Subject,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, EntityMerge,
    GraphOperation, GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Runtime,
    SeedDocument, TransactionDocument, TransactionState, ValidationCommandResult,
    ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

/// The seed: the node types Tag, WorkItem and its two children MergeRequest and Issue; the edge
/// types TAGGED (a work item to a tag, many), PRIMARY_TAG (a work item to a tag, one), BLOCKS (a
/// work item to a work item) and CLOSES (a merge request to an issue, which no other work item
/// may start); eight nodes and no edge.
const SEED: &str = include_str!("fixtures/merge-seed.yaml");
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const TAG: &str = "00000000-0000-4000-8000-000000000201";
const WORK_ITEM: &str = "00000000-0000-4000-8000-000000000202";
const MERGE_REQUEST: &str = "00000000-0000-4000-8000-000000000203";
const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
const PRIMARY_TAG: &str = "00000000-0000-4000-8000-000000000206";
const BLOCKS: &str = "00000000-0000-4000-8000-000000000207";
const CLOSES: &str = "00000000-0000-4000-8000-000000000208";
/// `title` of a WorkItem.
const TITLE: &str = "00000000-0000-4000-8000-000000000211";
/// `note` of a TAGGED edge.
const NOTE: &str = "00000000-0000-4000-8000-000000000212";
/// `branch` of a MergeRequest, which a WorkItem does not declare.
const BRANCH: &str = "00000000-0000-4000-8000-000000000213";
/// `related` of a WorkItem: a NodeRef to a work item, many.
const RELATED: &str = "00000000-0000-4000-8000-000000000214";
/// `follows` of a WorkItem: a NodeRef to a merge request only.
const FOLLOWS: &str = "00000000-0000-4000-8000-000000000215";
/// `link` of a WorkItem: a Record holding a NodeRef to a work item and a note.
const LINK: &str = "00000000-0000-4000-8000-000000000216";
/// `via` of a TAGGED edge: a NodeRef to a work item.
const VIA: &str = "00000000-0000-4000-8000-000000000217";
/// The WorkItem the store held first: the node kept, alias `group/app!18`.
const KEPT: &str = "00000000-0000-4000-8000-000000000301";
/// A WorkItem an import wrote beside it: the duplicate, alias `import/18`.
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
/// A Tag.
const PARSER: &str = "00000000-0000-4000-8000-000000000303";
/// A WorkItem, alias `tracker/18`, the kept node is later merged into.
const LATER: &str = "00000000-0000-4000-8000-000000000304";
/// A WorkItem without an alias.
const OTHER: &str = "00000000-0000-4000-8000-000000000305";
/// A MergeRequest, alias `group/app!18`: a child type of WorkItem.
const REQUEST: &str = "00000000-0000-4000-8000-000000000306";
/// An Issue, alias `group/app#7`: a sibling of MergeRequest.
const ISSUE: &str = "00000000-0000-4000-8000-000000000307";
/// A second Tag.
const LEXER: &str = "00000000-0000-4000-8000-000000000308";
/// The seeded evidence entry, a statement that the two are one.
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const STATEMENT_TEXT: &str = "group/app!18 was imported twice; import/18 is group/app!18.";
const REASON: &str = "the import wrote group/app!18 twice";

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

/// The three validation profiles: a merge applies under the profiles that apply a
/// `SupersedeNode`, which is every one (ADR 0018, open question 5).
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

/// `!MergeEntity` of `absorbed` into `into`, for the reason and on the statement.
fn merge(absorbed: &str, into: &str) -> GraphOperation {
    merge_with(id(absorbed), id(into), Some(REASON), Some(&[id(STATEMENT)]))
}

fn merge_with(
    absorbed: NodeId,
    into: NodeId,
    reason: Option<&str>,
    evidence: Option<&[EvidenceId]>,
) -> GraphOperation {
    GraphOperation::MergeEntity(EntityMerge {
        absorbed,
        into,
        reason: reason.map(str::to_owned),
        evidence: evidence.map(|ids| ids.iter().copied().collect()),
    })
}

/// The bare `{absorbed, into}` shape P1 wrote.
fn bare(absorbed: &str, into: &str) -> GraphOperation {
    merge_with(id(absorbed), id(into), None, None)
}

/// A transaction merging `absorbed` into `into`, declaring the statement.
fn merging(absorbed: &str, into: &str) -> GraphTransaction {
    transaction(vec![merge(absorbed, into)], &[id(STATEMENT)])
}

/// `!SupersedeNode` of `node` by `by`, for the reason and on the statement.
fn supersede(node: &str, by: &str) -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node: id(node),
        by: id(by),
        reason: REASON.to_owned(),
        evidence: BTreeSet::from([id(STATEMENT)]),
    })
}

/// An assertion of `subject`, `predicate` and `object`, each written as YAML (a flow mapping for
/// a value), citing the statement.
fn assertion(subject: &str, predicate: &str, object: &str) -> (AssertionId, GraphOperation) {
    let assertion = AssertionId::mint();
    let yaml = format!(
        "id: {assertion}
root_id: {ROOT}
subject: {subject}
predicate: {predicate}
object: {object}
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

/// A `title` assertion about the node `subject`.
fn titled(subject: &str, title: &str) -> (AssertionId, GraphOperation) {
    assertion(
        &format!("!Node {subject}"),
        &format!("!Property {TITLE}"),
        &format!("!Value {{value_kind: String, value: {title:?}}}"),
    )
}

/// An edge of `edge_type` from `source` to `target`, under `edge`, with `properties`.
fn edge(
    edge: EdgeId,
    edge_type: &str,
    source: &str,
    target: &str,
    properties: &[(&str, Value)],
) -> GraphOperation {
    GraphOperation::CreateEdge(EdgeDraft {
        id: edge,
        root_id: id(ROOT),
        type_id: id(edge_type),
        source: id(source),
        target: id(target),
        properties: properties
            .iter()
            .map(|(property, value)| (id(property), vec![value.clone()]))
            .collect(),
    })
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

/// `UpdateProperty` of `property` of `node` to `values`.
fn update(node: &str, property: &str, values: Vec<Value>) -> GraphOperation {
    GraphOperation::UpdateProperty(PropertyMutation {
        node: id(node),
        property: id(property),
        values,
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

/// The codes of a rejection, as `(validator, code)`.
fn codes(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    refused(verdict)
        .into_iter()
        .map(|(validator, code, _)| (validator, code))
        .collect()
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

/// The lineage of `absorbed`'s record at the head.
fn lineage(head: &CanonicalGraph, absorbed: &str) -> MergeLineage {
    head.superseded[&id::<NodeId>(absorbed)]
        .merge
        .clone()
        .expect("a merged node's record carries its lineage")
}

/// Every provider and profile, labelled, each with a freshly seeded store.
fn each_store(mut run: impl FnMut(&str, &std::path::Path, bool, AuthorityStateV1, &Runtime)) {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file, authority.clone());
            kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
            run(&at, directory.path(), file, authority.clone(), &kernel);
        }
    }
}

/// Every node id an assertion names: its subject, its `!Node` object, and every `NodeRef` inside
/// its value object, at any depth.
fn named_by(assertion: &ekr_graph::Assertion) -> Vec<NodeId> {
    fn walk(value: &CanonicalValue, into: &mut Vec<NodeId>) {
        match value {
            CanonicalValue::NodeRef(node) => into.push(node.node()),
            CanonicalValue::List(items) => items.iter().for_each(|item| walk(item, into)),
            CanonicalValue::Record(fields) => fields.values().for_each(|field| walk(field, into)),
            _ => {}
        }
    }
    let mut named = Vec::new();
    if let Subject::Node(node) = assertion.subject {
        named.push(node.node());
    }
    match &assertion.object {
        Object::Node(node) => named.push(node.node()),
        Object::Value(value) => walk(value, &mut named),
        Object::Type(_) => {}
    }
    named
}

/// The references to the duplicate revision 1 writes for the rewrite cases: an assertion about
/// it, an assertion naming it as a `!Node` object, an edge from it, an edge into it, and an edge
/// joining it to the kept node.
struct Referenced {
    about: AssertionId,
    naming: AssertionId,
    from: EdgeId,
    into: EdgeId,
    joining: EdgeId,
}

fn referenced(kernel: &Runtime) -> Referenced {
    let (about, add_about) = titled(DUPLICATE, "Fix the parser");
    let (naming, add_naming) = assertion(
        &format!("!Node {OTHER}"),
        &format!("!Property {RELATED}"),
        &format!("!Node {DUPLICATE}"),
    );
    let (from, into, joining) = (EdgeId::mint(), EdgeId::mint(), EdgeId::mint());
    committed(
        kernel,
        &transaction(
            vec![
                add_about,
                add_naming,
                edge(from, TAGGED, DUPLICATE, LEXER, &[(NOTE, text("imported"))]),
                edge(into, BLOCKS, OTHER, DUPLICATE, &[]),
                edge(joining, BLOCKS, DUPLICATE, KEPT, &[]),
            ],
            &[id(STATEMENT)],
        ),
        20,
    );
    Referenced {
        about,
        naming,
        from,
        into,
        joining,
    }
}

/// `a-merge-of-same-type-duplicates-rewrites-every-reference`: no head edge or assertion names
/// the absorbed node, which leaves the head into a record naming the node kept; an operation
/// naming it afterwards is refused as `node-superseded`, naming the node kept.
#[test]
fn a_merge_of_same_type_duplicates_rewrites_every_reference() {
    each_store(|at, _, _, _, kernel| {
        let made = referenced(kernel);
        committed(kernel, &merging(DUPLICATE, KEPT), 30);
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(2), "{at}");
        let (duplicate, kept): (NodeId, NodeId) = (id(DUPLICATE), id(KEPT));
        assert!(!head.nodes.contains_key(&duplicate), "{at}");
        assert_eq!(head.superseded_by(duplicate), Some(kept), "{at}");
        for edge in head.edges.values() {
            assert!(
                edge.source.node() != duplicate && edge.target.node() != duplicate,
                "{at}: head edge {} names the absorbed node",
                edge.id
            );
        }
        for assertion in head.assertions.values() {
            assert!(
                !named_by(assertion).contains(&duplicate),
                "{at}: head assertion {} names the absorbed node",
                assertion.id
            );
        }
        assert_eq!(head.edges[&made.from].source.node(), kept, "{at}");
        assert_eq!(head.edges[&made.into].target.node(), kept, "{at}");
        assert!(!head.edges.contains_key(&made.joining), "{at}");
        assert_eq!(
            head.assertions[&made.about].subject,
            Subject::Node(CanonicalRef::new(kept)),
            "{at}"
        );
        assert_eq!(
            head.assertions[&made.naming].object,
            Object::Node(CanonicalRef::new(kept)),
            "{at}"
        );
        let later = submit(
            kernel,
            &transaction(vec![update(DUPLICATE, TITLE, vec![text("again")])], &[]),
            40,
        );
        assert_eq!(
            codes(&later),
            vec![(ValidatorName::Reference, "node-superseded".to_owned())],
            "{at}: {later:?}"
        );
        assert!(refused(&later)[0].2.contains(KEPT), "{at}: {later:?}");
    });
}

/// `a-merge-keeps-assertion-ids-and-bytes`: a rewritten assertion keeps its id, evidence,
/// assessment, lifecycle and both times — every byte but the reference rewritten — and at the
/// revision before the merge its subject is the absorbed node.
#[test]
fn a_merge_keeps_assertion_ids_and_bytes() {
    each_store(|at, _, _, _, kernel| {
        let made = referenced(kernel);
        committed(kernel, &merging(DUPLICATE, KEPT), 30);
        let (before, after) = (graph_at(kernel, 1), graph_at(kernel, 2));
        for assertion in [made.about, made.naming] {
            let (was, is) = (
                &before.assertions[&assertion],
                &after.assertions[&assertion],
            );
            assert_eq!(is.id, was.id, "{at}");
            assert_eq!(is.root_id, was.root_id, "{at}");
            assert_eq!(is.predicate, was.predicate, "{at}");
            assert_eq!(is.evidence, was.evidence, "{at}");
            assert_eq!(is.proposed_by, was.proposed_by, "{at}");
            assert_eq!(is.assessment, was.assessment, "{at}");
            assert_eq!(is.lifecycle, was.lifecycle, "{at}");
            assert_eq!(is.valid_time, was.valid_time, "{at}");
            assert_eq!(is.transaction_time, was.transaction_time, "{at}");
        }
        let duplicate = Subject::Node(CanonicalRef::new(id(DUPLICATE)));
        assert_eq!(before.assertions[&made.about].subject, duplicate, "{at}");
        let mut restored = after.assertions[&made.about].clone();
        restored.subject = duplicate;
        assert_eq!(
            ContentHash::of(&restored),
            ContentHash::of(&before.assertions[&made.about]),
            "{at}: only the subject moved"
        );
        assert_eq!(
            before.assertions[&made.naming].object,
            Object::Node(CanonicalRef::new(id(DUPLICATE))),
            "{at}"
        );
    });
}

/// `a-merge-collapses-a-duplicate-edge`: an edge from the absorbed node with the type and other
/// end of one the kept node holds, and equal properties, collapses onto it: it leaves the head
/// into the record, the lineage maps it to the kept edge, and an assertion about it is rewritten
/// to the kept edge.
#[test]
fn a_merge_collapses_a_duplicate_edge() {
    each_store(|at, _, _, _, kernel| {
        let (kept_edge, duplicate_edge) = (EdgeId::mint(), EdgeId::mint());
        let (about, add_about) = assertion(
            &format!("!Edge {duplicate_edge}"),
            &format!("!Property {NOTE}"),
            "!Value {value_kind: String, value: imported}",
        );
        committed(
            kernel,
            &transaction(
                vec![
                    edge(kept_edge, TAGGED, KEPT, PARSER, &[(NOTE, text("imported"))]),
                    edge(
                        duplicate_edge,
                        TAGGED,
                        DUPLICATE,
                        PARSER,
                        &[(NOTE, text("imported"))],
                    ),
                    add_about,
                ],
                &[id(STATEMENT)],
            ),
            20,
        );
        let before = graph_at(kernel, 1);
        committed(kernel, &merging(DUPLICATE, KEPT), 30);
        let head = kernel.snapshot().unwrap();
        assert!(head.edges.contains_key(&kept_edge), "{at}");
        assert!(!head.edges.contains_key(&duplicate_edge), "{at}");
        assert_eq!(
            head.edges
                .values()
                .filter(|edge| edge.type_id == id(TAGGED))
                .count(),
            1,
            "{at}: one TAGGED edge from the kept node to the tag"
        );
        let record = &head.superseded[&id::<NodeId>(DUPLICATE)];
        assert_eq!(
            record.edges.get(&duplicate_edge),
            Some(&before.edges[&duplicate_edge]),
            "{at}: the collapsed edge is held whole, as it was"
        );
        assert_eq!(
            lineage(&head, DUPLICATE).collapsed,
            BTreeMap::from([(duplicate_edge, kept_edge)]),
            "{at}"
        );
        assert_eq!(
            head.retired_with(duplicate_edge),
            Some((id(DUPLICATE), id(KEPT))),
            "{at}"
        );
        assert_eq!(
            head.assertions[&about].subject,
            Subject::Edge(CanonicalRef::new(kept_edge)),
            "{at}"
        );
    });
}

/// `a-merge-refuses-conflicting-duplicate-edges`: the same two edges with different properties
/// refuse the merge as `merge-duplicate-edge-conflict`, naming both, and nothing moves.
#[test]
fn a_merge_refuses_conflicting_duplicate_edges() {
    each_store(|at, _, _, _, kernel| {
        let (kept_edge, duplicate_edge) = (EdgeId::mint(), EdgeId::mint());
        committed(
            kernel,
            &transaction(
                vec![
                    edge(kept_edge, TAGGED, KEPT, PARSER, &[(NOTE, text("curated"))]),
                    edge(
                        duplicate_edge,
                        TAGGED,
                        DUPLICATE,
                        PARSER,
                        &[(NOTE, text("imported"))],
                    ),
                ],
                &[],
            ),
            20,
        );
        let head = kernel.head().unwrap().unwrap();
        let verdict = submit(kernel, &merging(DUPLICATE, KEPT), 30);
        assert_eq!(
            codes(&verdict),
            vec![(
                ValidatorName::Structural,
                "merge-duplicate-edge-conflict".to_owned()
            )],
            "{at}: {verdict:?}"
        );
        let message = &refused(&verdict)[0].2;
        for named in [kept_edge, duplicate_edge] {
            assert!(message.contains(&named.to_string()), "{at}: {message}");
        }
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// `a-merge-that-would-exceed-edge-cardinality-is-refused`: PRIMARY_TAG is One, the two nodes
/// each hold one to different tags, and the kept node would hold both: `edge-cardinality`, naming
/// both edges. Two to the same tag collapse instead, and the merge commits.
#[test]
fn a_merge_that_would_exceed_edge_cardinality_is_refused() {
    each_store(|at, _, _, _, kernel| {
        let (kept_edge, duplicate_edge) = (EdgeId::mint(), EdgeId::mint());
        committed(
            kernel,
            &transaction(
                vec![
                    edge(kept_edge, PRIMARY_TAG, KEPT, PARSER, &[]),
                    edge(duplicate_edge, PRIMARY_TAG, DUPLICATE, LEXER, &[]),
                ],
                &[],
            ),
            20,
        );
        let verdict = submit(kernel, &merging(DUPLICATE, KEPT), 30);
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Cardinality, "edge-cardinality".to_owned())],
            "{at}: {verdict:?}"
        );
        let message = &refused(&verdict)[0].2;
        for named in [kept_edge, duplicate_edge] {
            assert!(message.contains(&named.to_string()), "{at}: {message}");
        }
        // Moved to the same tag first, the duplicate's edge collapses and the count is one.
        let same = EdgeId::mint();
        committed(
            kernel,
            &transaction(
                vec![
                    GraphOperation::DeleteEdge(duplicate_edge),
                    edge(same, PRIMARY_TAG, DUPLICATE, PARSER, &[]),
                ],
                &[],
            ),
            40,
        );
        committed(kernel, &merging(DUPLICATE, KEPT), 50);
        let head = kernel.snapshot().unwrap();
        assert_eq!(
            lineage(&head, DUPLICATE).collapsed,
            BTreeMap::from([(same, kept_edge)]),
            "{at}"
        );
    });
}

/// `a-merge-into-a-parent-type-keeps-the-survivors-type`: a MergeRequest merged into a WorkItem,
/// its parent type, commits; the kept node stays a WorkItem and the record holds the merge request
/// as one. A WorkItem merged into a MergeRequest, the other way, keeps the MergeRequest.
#[test]
fn a_merge_into_a_parent_type_keeps_the_survivors_type() {
    each_store(|at, _, _, _, kernel| {
        let tagged = EdgeId::mint();
        committed(
            kernel,
            &transaction(vec![edge(tagged, TAGGED, REQUEST, PARSER, &[])], &[]),
            20,
        );
        committed(kernel, &merging(REQUEST, KEPT), 30);
        let head = kernel.snapshot().unwrap();
        assert_eq!(
            head.nodes[&id::<NodeId>(KEPT)].type_id,
            id(WORK_ITEM),
            "{at}"
        );
        assert_eq!(
            head.superseded[&id::<NodeId>(REQUEST)].node.type_id,
            id(MERGE_REQUEST),
            "{at}"
        );
        assert_eq!(head.edges[&tagged].source.node(), id(KEPT), "{at}");

        let child = NodeId::mint();
        committed(
            kernel,
            &transaction(
                vec![GraphOperation::CreateNode(NodeDraft {
                    id: child,
                    root_id: id(ROOT),
                    type_id: id(MERGE_REQUEST),
                    canonical_name: "group/app!20".into(),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                })],
                &[],
            ),
            50,
        );
        committed(
            kernel,
            &transaction(
                vec![merge_with(
                    id(LATER),
                    child,
                    Some(REASON),
                    Some(&[id(STATEMENT)]),
                )],
                &[id(STATEMENT)],
            ),
            60,
        );
        let head = kernel.snapshot().unwrap();
        assert_eq!(head.nodes[&child].type_id, id(MERGE_REQUEST), "{at}");
    });
}

/// `a-merge-into-a-parent-type-revalidates-endpoints`: CLOSES starts only at a MergeRequest, so
/// a merge request holding one cannot be merged into a WorkItem: `edge-endpoint-type`, naming the
/// edge. An active relation assertion through CLOSES about it is held to the same end.
#[test]
fn a_merge_into_a_parent_type_revalidates_endpoints() {
    each_store(|at, _, _, _, kernel| {
        let closes = EdgeId::mint();
        committed(
            kernel,
            &transaction(vec![edge(closes, CLOSES, REQUEST, ISSUE, &[])], &[]),
            20,
        );
        let verdict = submit(kernel, &merging(REQUEST, KEPT), 30);
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Type, "edge-endpoint-type".to_owned())],
            "{at}: {verdict:?}"
        );
        assert!(
            refused(&verdict)[0].2.contains(&closes.to_string()),
            "{at}: {verdict:?}"
        );
        committed(
            kernel,
            &transaction(vec![GraphOperation::DeleteEdge(closes)], &[]),
            40,
        );
        let (_, claim) = assertion(
            &format!("!Node {REQUEST}"),
            &format!("!Relation {CLOSES}"),
            &format!("!Node {ISSUE}"),
        );
        committed(kernel, &transaction(vec![claim], &[id(STATEMENT)]), 50);
        let verdict = submit(kernel, &merging(REQUEST, KEPT), 60);
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Type, "edge-endpoint-type".to_owned())],
            "{at}: an active relation assertion: {verdict:?}"
        );
    });
}

/// `a-merge-across-sibling-types-is-refused`: a MergeRequest and an Issue share a parent and
/// neither conforms to the other: `merge-types-unrelated`, naming `SupersedeNode` as the fix
/// available today. A Tag and a WorkItem, unrelated, are refused the same way.
#[test]
fn a_merge_across_sibling_types_is_refused() {
    each_store(|at, _, _, _, kernel| {
        let head = kernel.head().unwrap().unwrap();
        for (absorbed, into) in [(REQUEST, ISSUE), (PARSER, KEPT)] {
            let verdict = submit(kernel, &merging(absorbed, into), 30);
            assert_eq!(
                codes(&verdict),
                vec![(ValidatorName::Type, "merge-types-unrelated".to_owned())],
                "{at}: {verdict:?}"
            );
            assert!(
                refused(&verdict)[0].2.contains("SupersedeNode"),
                "{at}: {verdict:?}"
            );
        }
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// `a-merge-is-refused-while-absorbed-holds-an-undeclared-property`: a merge request holding a
/// `branch`, which a WorkItem does not declare, is refused as `merge-undeclared-property`, naming
/// it; cleared by an `UpdateProperty` in an earlier revision, the merge commits.
#[test]
fn a_merge_is_refused_while_absorbed_holds_an_undeclared_property() {
    each_store(|at, _, _, _, kernel| {
        committed(
            kernel,
            &transaction(vec![update(REQUEST, BRANCH, vec![text("fix/parser")])], &[]),
            20,
        );
        let verdict = submit(kernel, &merging(REQUEST, KEPT), 30);
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Type, "merge-undeclared-property".to_owned())],
            "{at}: {verdict:?}"
        );
        assert!(refused(&verdict)[0].2.contains(BRANCH), "{at}: {verdict:?}");
        committed(
            kernel,
            &transaction(vec![update(REQUEST, BRANCH, vec![])], &[]),
            40,
        );
        committed(kernel, &merging(REQUEST, KEPT), 50);
        assert_eq!(
            kernel.snapshot().unwrap().superseded_by(id(REQUEST)),
            Some(id(KEPT)),
            "{at}"
        );
    });
}

/// `a-merge-moves-absent-properties-and-refuses-conflicting-ones`: a property only the absorbed
/// node holds moves to the kept node, one both hold equally stays, and one both hold differently
/// refuses the merge as `merge-property-conflict`, naming it. The record keeps every value.
#[test]
fn a_merge_moves_absent_properties_and_refuses_conflicting_ones() {
    each_store(|at, _, _, _, kernel| {
        committed(
            kernel,
            &transaction(
                vec![
                    update(DUPLICATE, TITLE, vec![text("Fix the parser")]),
                    update(KEPT, TITLE, vec![text("Fix the lexer")]),
                ],
                &[],
            ),
            20,
        );
        let verdict = submit(kernel, &merging(DUPLICATE, KEPT), 30);
        assert_eq!(
            codes(&verdict),
            vec![(
                ValidatorName::Structural,
                "merge-property-conflict".to_owned()
            )],
            "{at}: {verdict:?}"
        );
        assert!(refused(&verdict)[0].2.contains(TITLE), "{at}: {verdict:?}");

        committed(
            kernel,
            &transaction(vec![update(KEPT, TITLE, vec![])], &[]),
            40,
        );
        committed(kernel, &merging(DUPLICATE, KEPT), 50);
        let head = kernel.snapshot().unwrap();
        let title: PropertyId = id(TITLE);
        assert_eq!(
            head.nodes[&id::<NodeId>(KEPT)].properties[&title],
            vec![CanonicalValue::String("Fix the parser".into())],
            "{at}: the absent property moved"
        );
        assert_eq!(
            lineage(&head, DUPLICATE).moved_properties,
            BTreeSet::from([title]),
            "{at}"
        );
        assert_eq!(
            head.superseded[&id::<NodeId>(DUPLICATE)].node.properties[&title],
            vec![CanonicalValue::String("Fix the parser".into())],
            "{at}: the record keeps the absorbed node's values"
        );

        // Equal values stay, and nothing is recorded as moved.
        committed(
            kernel,
            &transaction(
                vec![update(LATER, TITLE, vec![text("Fix the parser")])],
                &[],
            ),
            60,
        );
        committed(kernel, &merging(LATER, KEPT), 70);
        let head = kernel.snapshot().unwrap();
        assert!(lineage(&head, LATER).moved_properties.is_empty(), "{at}");
    });
}

/// `a-merge-records-its-lineage`: each rewritten edge end, collapsed edge, retired joining edge,
/// rewritten assertion field with the reference it held before, and moved property.
#[test]
fn a_merge_records_its_lineage() {
    each_store(|at, _, _, _, kernel| {
        let made = referenced(kernel);
        let (kept_edge, duplicate_edge) = (EdgeId::mint(), EdgeId::mint());
        committed(
            kernel,
            &transaction(
                vec![
                    edge(kept_edge, TAGGED, KEPT, PARSER, &[]),
                    edge(duplicate_edge, TAGGED, DUPLICATE, PARSER, &[]),
                    update(DUPLICATE, TITLE, vec![text("Fix the parser")]),
                ],
                &[],
            ),
            25,
        );
        let before = graph_at(kernel, 2);
        committed(kernel, &merging(DUPLICATE, KEPT), 30);
        let head = kernel.snapshot().unwrap();
        let duplicate: NodeId = id(DUPLICATE);
        let expected = MergeLineage {
            edges: BTreeMap::from([
                (made.from, MergedEnds::Source),
                (made.into, MergedEnds::Target),
            ]),
            collapsed: BTreeMap::from([(duplicate_edge, kept_edge)]),
            retired: BTreeSet::from([made.joining]),
            assertions: BTreeMap::from([
                (
                    made.about,
                    RewrittenAssertion {
                        subject: Some(Subject::Node(CanonicalRef::new(duplicate))),
                        object: None,
                    },
                ),
                (
                    made.naming,
                    RewrittenAssertion {
                        subject: None,
                        object: Some(Object::Node(CanonicalRef::new(duplicate))),
                    },
                ),
            ]),
            moved_properties: BTreeSet::from([id(TITLE)]),
            node_values: BTreeMap::new(),
            edge_values: BTreeMap::new(),
        };
        assert_eq!(lineage(&head, DUPLICATE), expected, "{at}");
        let record = &head.superseded[&duplicate];
        assert_eq!(record.node, before.nodes[&duplicate], "{at}");
        assert_eq!(
            record.edges.keys().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([duplicate_edge, made.joining]),
            "{at}: the record holds the collapsed and the joining edge whole"
        );
        assert_eq!(
            record.merged_edges(),
            BTreeSet::from([made.from, made.into, kept_edge]),
            "{at}: the head's edges the merge rewrote or collapsed an edge onto"
        );
        assert_eq!(
            record.merged_assertions(),
            BTreeSet::from([made.about, made.naming]),
            "{at}"
        );
        assert_eq!(record.by.node(), id::<NodeId>(KEPT), "{at}");
        assert_eq!(record.revision, RevisionNumber::new(3), "{at}");
        assert_eq!(record.reason, REASON, "{at}");
        assert_eq!(
            record.evidence,
            BTreeSet::from([CanonicalRef::new(id(STATEMENT))]),
            "{at}"
        );
        assert_eq!(
            head.retired_with(made.joining),
            Some((duplicate, id(KEPT))),
            "{at}"
        );
    });
}

/// `a-merge-rewrites-node-ref-values`: a `NodeRef` naming the absorbed node in a node's property,
/// in an edge's property and inside an assertion's value object, at any depth, names the kept
/// node afterwards, and the lineage holds each with the values it had before.
#[test]
fn a_merge_rewrites_node_ref_values() {
    each_store(|at, _, _, _, kernel| {
        let tagged = EdgeId::mint();
        let (nested, add_nested) = assertion(
            &format!("!Node {OTHER}"),
            &format!("!Property {LINK}"),
            &format!(
                "!Value {{value_kind: Record, value: {{note: {{value_kind: String, value: \
                 imported}}, target: {{value_kind: NodeRef, value: {DUPLICATE}}}}}}}"
            ),
        );
        let duplicate_ref = Value::NodeRef(id(DUPLICATE));
        committed(
            kernel,
            &transaction(
                vec![
                    update(
                        OTHER,
                        RELATED,
                        vec![duplicate_ref.clone(), Value::NodeRef(id(LATER))],
                    ),
                    edge(tagged, TAGGED, LATER, PARSER, &[(VIA, duplicate_ref)]),
                    add_nested,
                ],
                &[id(STATEMENT)],
            ),
            20,
        );
        let before = graph_at(kernel, 1);
        committed(kernel, &merging(DUPLICATE, KEPT), 30);
        let head = kernel.snapshot().unwrap();
        let kept = CanonicalValue::NodeRef(CanonicalRef::new(id(KEPT)));
        let (other, related, via): (NodeId, PropertyId, PropertyId) =
            (id(OTHER), id(RELATED), id(VIA));
        assert_eq!(
            head.nodes[&other].properties[&related],
            vec![
                kept.clone(),
                CanonicalValue::NodeRef(CanonicalRef::new(id(LATER)))
            ],
            "{at}"
        );
        assert_eq!(
            head.edges[&tagged].properties[&via],
            vec![kept.clone()],
            "{at}"
        );
        assert!(
            !named_by(&head.assertions[&nested]).contains(&id(DUPLICATE)),
            "{at}"
        );
        assert!(
            named_by(&head.assertions[&nested]).contains(&id(KEPT)),
            "{at}"
        );
        let lineage = lineage(&head, DUPLICATE);
        assert_eq!(
            lineage.node_values,
            BTreeMap::from([(
                other,
                BTreeMap::from([(related, before.nodes[&other].properties[&related].clone())])
            )]),
            "{at}"
        );
        assert_eq!(
            lineage.edge_values,
            BTreeMap::from([(
                tagged,
                BTreeMap::from([(via, before.edges[&tagged].properties[&via].clone())])
            )]),
            "{at}"
        );
        assert_eq!(
            lineage.assertions[&nested],
            RewrittenAssertion {
                subject: None,
                object: Some(before.assertions[&nested].object.clone()),
            },
            "{at}"
        );
    });
}

/// `a-merge-of-a-survivor-repoints-earlier-records`: after the kept node is itself merged into a
/// later one, the absorbed node's record names the later node, so a reader follows one pointer,
/// and keeps the revision that made it.
#[test]
fn a_merge_of_a_survivor_repoints_earlier_records() {
    each_store(|at, _, _, _, kernel| {
        committed(kernel, &merging(DUPLICATE, KEPT), 20);
        committed(
            kernel,
            &transaction(vec![supersede(OTHER, KEPT)], &[id(STATEMENT)]),
            30,
        );
        committed(kernel, &merging(KEPT, LATER), 40);
        let head = kernel.snapshot().unwrap();
        let later: NodeId = id(LATER);
        for record in [DUPLICATE, OTHER, KEPT] {
            assert_eq!(
                head.superseded[&id::<NodeId>(record)].by.node(),
                later,
                "{at}: {record}"
            );
        }
        assert_eq!(
            head.superseded[&id::<NodeId>(DUPLICATE)].revision,
            RevisionNumber::new(1),
            "{at}"
        );
        assert_eq!(
            graph_at(kernel, 2).superseded[&id::<NodeId>(DUPLICATE)]
                .by
                .node(),
            id::<NodeId>(KEPT),
            "{at}: revision 2 still names the node it named"
        );
    });
}

/// `a-merge-of-an-absorbed-node-is-refused`, and a merge into one: each `node-superseded`,
/// naming the node it was merged into.
#[test]
fn a_merge_of_an_absorbed_node_is_refused() {
    each_store(|at, _, _, _, kernel| {
        committed(kernel, &merging(DUPLICATE, KEPT), 20);
        let head = kernel.head().unwrap().unwrap();
        for (absorbed, into) in [(DUPLICATE, OTHER), (OTHER, DUPLICATE)] {
            let verdict = submit(kernel, &merging(absorbed, into), 30);
            assert_eq!(
                codes(&verdict),
                vec![(ValidatorName::Reference, "node-superseded".to_owned())],
                "{at}: {absorbed} into {into}: {verdict:?}"
            );
            assert!(refused(&verdict)[0].2.contains(KEPT), "{at}: {verdict:?}");
        }
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// `a-merged-alias-resolves-to-the-survivor`: the absorbed node's alias, under its type, indexes
/// the kept node, and a later node of that type and alias is `alias-already-exists`.
#[test]
fn a_merged_alias_resolves_to_the_survivor() {
    each_store(|at, _, _, _, kernel| {
        committed(kernel, &merging(DUPLICATE, KEPT), 20);
        let head = kernel.snapshot().unwrap();
        let aliases = AliasIndex::of(&head);
        assert_eq!(
            aliases.nodes(id(WORK_ITEM), "import/18"),
            &[id::<NodeId>(KEPT)],
            "{at}"
        );
        let verdict = submit(
            kernel,
            &transaction(
                vec![GraphOperation::CreateNode(NodeDraft {
                    id: NodeId::mint(),
                    root_id: id(ROOT),
                    type_id: id(WORK_ITEM),
                    canonical_name: "import/18".into(),
                    properties: BTreeMap::new(),
                    aliases: vec!["import/18".into()],
                })],
                &[],
            ),
            30,
        );
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Structural, "alias-already-exists".to_owned())],
            "{at}: {verdict:?}"
        );
    });
}

/// The base-era store retaining a rejected proposal that carries a bare `MergeEntity`.
const BASE_SQLITE_STORE: &[u8] =
    include_bytes!("fixtures/base-cee0cae-v1-ownerless-modify-property.sqlite");
/// Its proposal carrying all twelve base-era kinds, `MergeEntity` among them.
const ALL_KINDS_PROPOSAL: &str = "00000000-0000-4000-8000-0000000003f0";

/// `a-bare-merge-keeps-its-retained-verdict`: the base-era store, whose retained rejection carries
/// a bare `MergeEntity`, reopens unedited — replay revalidates that rejection and compares its
/// issues — and the bare merge it carries re-derives its retained hash; a bare merge of two held
/// nodes is refused with P1's one issue under every profile.
#[test]
fn a_bare_merge_keeps_its_retained_verdict() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("state.db"), BASE_SQLITE_STORE).unwrap();
    let base = BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    };
    let mut authority = anchor(ValidationProfileV1::deterministic(base.validator));
    authority.agents = [(base.operator, "operator"), (base.validator, "validator")]
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
        .collect();
    let kernel = Runtime::sqlite(&directory.path().join("state.db"), "test", base, authority)
        .expect("the base-era store opens");
    kernel.head().expect("and replays");
    let record = kernel
        .transactions()
        .unwrap()
        .get(&ALL_KINDS_PROPOSAL.parse().unwrap())
        .cloned()
        .expect("the all-kinds proposal is retained");
    assert_eq!(record.state(), TransactionState::Rejected);
    let parsed = TransactionDocument::parse(&record.proposal.document_bytes).unwrap();
    let merges: Vec<&EntityMerge> = parsed
        .transaction()
        .operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::MergeEntity(merge) => Some(merge),
            _ => None,
        })
        .collect();
    assert_eq!(merges.len(), 1);
    assert_eq!(
        (merges[0].reason.as_ref(), merges[0].evidence.as_ref()),
        (None, None)
    );
    assert!(
        merges[0].is_bare(),
        "the retained merge reads as the bare P1 shape"
    );
    let canonical =
        GraphTransaction::<CanonicalValue>::try_from(parsed.transaction().clone()).unwrap();
    assert_eq!(
        Some(ContentHash::of(&canonical)),
        record.proposal.canonical_transaction_hash
    );

    each_store(|at, _, _, _, kernel| {
        let verdict = submit(kernel, &transaction(vec![bare(DUPLICATE, KEPT)], &[]), 20);
        assert_eq!(
            refused(&verdict),
            vec![(
                ValidatorName::Structural,
                "unsupported-operation".to_owned(),
                "MergeEntity is not supported in P1".to_owned()
            )],
            "{at}"
        );
    });
}

/// `a-merge-is-refused-by-name`: one case per refusal of ADR 0018 decision 6, each its own code,
/// none moving the head.
#[test]
#[allow(clippy::too_many_lines)]
fn a_merge_is_refused_by_name() {
    each_store(|at, _, _, _, kernel| {
        // Revision 1: an edge at the duplicate, a merge request closing an issue, a NodeRef to the
        // merge request a WorkItem allows only for merge requests; revision 2: OTHER superseded.
        let duplicate_edge = EdgeId::mint();
        committed(
            kernel,
            &transaction(
                vec![
                    edge(duplicate_edge, TAGGED, DUPLICATE, PARSER, &[]),
                    update(LATER, FOLLOWS, vec![Value::NodeRef(id(REQUEST))]),
                ],
                &[],
            ),
            20,
        );
        committed(
            kernel,
            &transaction(vec![supersede(OTHER, LATER)], &[id(STATEMENT)]),
            30,
        );
        let head = kernel.head().unwrap().unwrap();
        let statement: EvidenceId = id(STATEMENT);
        let unknown_evidence = EvidenceId::mint();
        let reference = |code: &str| (ValidatorName::Reference, code.to_owned());
        let structural = |code: &str| (ValidatorName::Structural, code.to_owned());
        let types = |code: &str| (ValidatorName::Type, code.to_owned());
        let created = NodeId::mint();
        let (_, about_duplicate) = titled(DUPLICATE, "Fix the lexer");
        type Case = (
            &'static str,
            GraphTransaction,
            Vec<(ValidatorName, String)>,
            Option<&'static str>,
        );
        let cases: Vec<Case> = vec![
            (
                "a node merged into itself",
                merging(KEPT, KEPT),
                vec![structural("merge-into-itself")],
                None,
            ),
            (
                "an absorbed node no revision holds",
                transaction(
                    vec![merge_with(
                        NodeId::mint(),
                        id(KEPT),
                        Some(REASON),
                        Some(&[statement]),
                    )],
                    &[statement],
                ),
                vec![reference("unresolved-node")],
                None,
            ),
            (
                "a kept node no revision holds",
                transaction(
                    vec![merge_with(
                        id(DUPLICATE),
                        NodeId::mint(),
                        Some(REASON),
                        Some(&[statement]),
                    )],
                    &[statement],
                ),
                vec![reference("unresolved-node")],
                None,
            ),
            (
                "an absorbed node the same transaction creates",
                transaction(
                    vec![
                        GraphOperation::CreateNode(NodeDraft {
                            id: created,
                            root_id: id(ROOT),
                            type_id: id(WORK_ITEM),
                            canonical_name: "group/app!21".into(),
                            properties: BTreeMap::new(),
                            aliases: Vec::new(),
                        }),
                        merge_with(created, id(KEPT), Some(REASON), Some(&[statement])),
                    ],
                    &[statement],
                ),
                vec![reference("unresolved-node")],
                None,
            ),
            (
                "an absorbed node already superseded",
                merging(OTHER, KEPT),
                vec![reference("node-superseded")],
                Some(LATER),
            ),
            (
                "a kept node already superseded",
                merging(DUPLICATE, OTHER),
                vec![reference("node-superseded")],
                Some(LATER),
            ),
            (
                "unrelated types",
                merging(PARSER, KEPT),
                vec![types("merge-types-unrelated")],
                None,
            ),
            (
                "a NodeRef value the kept node's type no longer satisfies",
                merging(REQUEST, KEPT),
                vec![types("wrong-type")],
                Some(FOLLOWS),
            ),
            (
                "a merge without a reason",
                transaction(
                    vec![merge_with(
                        id(DUPLICATE),
                        id(KEPT),
                        None,
                        Some(&[statement]),
                    )],
                    &[statement],
                ),
                vec![structural("merge-without-reason")],
                None,
            ),
            (
                "a merge whose reason is only whitespace",
                transaction(
                    vec![merge_with(
                        id(DUPLICATE),
                        id(KEPT),
                        Some(" \n"),
                        Some(&[statement]),
                    )],
                    &[statement],
                ),
                vec![structural("merge-without-reason")],
                None,
            ),
            (
                "a merge without evidence",
                transaction(
                    vec![merge_with(id(DUPLICATE), id(KEPT), Some(REASON), None)],
                    &[],
                ),
                vec![structural("merge-without-evidence")],
                None,
            ),
            (
                "a merge on an empty evidence list",
                transaction(
                    vec![merge_with(id(DUPLICATE), id(KEPT), Some(REASON), Some(&[]))],
                    &[],
                ),
                vec![structural("merge-without-evidence")],
                None,
            ),
            (
                "a merge on evidence no revision retains",
                transaction(
                    vec![merge_with(
                        id(DUPLICATE),
                        id(KEPT),
                        Some(REASON),
                        Some(&[unknown_evidence]),
                    )],
                    &[unknown_evidence],
                ),
                vec![reference("merge-evidence-unknown")],
                None,
            ),
            (
                "a merge the evidence set leaves out",
                transaction(vec![merge(DUPLICATE, KEPT)], &[]),
                vec![structural("evidence-set-mismatch")],
                None,
            ),
            (
                "the absorbed node updated in the same transaction",
                transaction(
                    vec![
                        merge(DUPLICATE, KEPT),
                        update(DUPLICATE, TITLE, vec![text("x")]),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "an assertion about the absorbed node in the same transaction",
                transaction(vec![merge(DUPLICATE, KEPT), about_duplicate], &[statement]),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "an edge at the absorbed node deleted in the same transaction",
                transaction(
                    vec![
                        merge(DUPLICATE, KEPT),
                        GraphOperation::DeleteEdge(duplicate_edge),
                    ],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "the absorbed node superseded in the same transaction",
                transaction(
                    vec![merge(DUPLICATE, KEPT), supersede(DUPLICATE, LATER)],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "the kept node superseded in the same transaction",
                transaction(
                    vec![merge(DUPLICATE, KEPT), supersede(KEPT, LATER)],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "two merges in one transaction",
                transaction(
                    vec![merge(DUPLICATE, KEPT), merge(ISSUE, LATER)],
                    &[statement],
                ),
                vec![structural("superseded-node-named-in-transaction")],
                None,
            ),
            (
                "a later bare merge of the absorbed node",
                transaction(vec![bare(OTHER, KEPT)], &[]),
                vec![reference("node-superseded")],
                Some(LATER),
            ),
        ];
        for (offset, (what, tx, expected, names)) in cases.into_iter().enumerate() {
            let verdict = submit(kernel, &tx, 100 + 10 * i64::try_from(offset).unwrap());
            let issues = refused(&verdict);
            assert_eq!(
                issues
                    .iter()
                    .map(|(validator, code, _)| (*validator, code.clone()))
                    .collect::<Vec<_>>(),
                expected,
                "{at}: {what}: {issues:?}"
            );
            if let Some(named) = names {
                assert!(
                    issues[0].2.contains(named),
                    "{at}: {what}: the refusal names {named}: {issues:?}"
                );
            }
        }
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// `a-merge-replays-to-identical-roots`: a reopen from the replay checkpoint written after the
/// merges, a full replay and a migrated store each give every revision the same roots and the
/// same head graph, the lineage included; and a supersession record without a merge encodes to
/// the six fields it encoded to before the lineage existed, so a store that never merges keeps
/// its roots byte for byte.
#[test]
fn a_merge_replays_to_identical_roots() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            replays_to_identical_roots(&at, directory.path(), file, &authority);
        }
    }

    // A record without a merge: its bytes are the six fields, as ADR 0017 wrote them.
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false, profiles()[0].1.clone());
    kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    committed(
        &kernel,
        &transaction(vec![supersede(DUPLICATE, KEPT)], &[id(STATEMENT)]),
        20,
    );
    let head = kernel.snapshot().unwrap();
    let record = &head.superseded[&id::<NodeId>(DUPLICATE)];
    assert!(record.merge.is_none());
    let mut six = ekr_core::canonical::Encoder::new();
    {
        use ekr_core::canonical::Canonical;
        record.node.encode(&mut six);
        six.map(record.edges.iter());
        record.by.encode(&mut six);
        record.revision.encode(&mut six);
        record.reason.encode(&mut six);
        six.set(record.evidence.iter());
    }
    assert_eq!(
        ekr_core::canonical::Canonical::canonical_bytes(record),
        six.finish(),
        "a supersession record without a merge encodes as before the lineage existed"
    );
}

/// One store of [`a_merge_replays_to_identical_roots`]: two merges, then revisions 4 to 6, so that
/// revision 5 writes a replay checkpoint holding both records; reopened, fully replayed and
/// migrated.
fn replays_to_identical_roots(
    at: &str,
    path: &std::path::Path,
    file: bool,
    authority: &AuthorityStateV1,
) {
    let kernel = open(path, file, authority.clone());
    kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    let _ = referenced(&kernel);
    committed(&kernel, &merging(DUPLICATE, KEPT), 30);
    committed(&kernel, &merging(KEPT, LATER), 40);
    for (n, time) in [(0_i64, 50), (1, 60), (2, 70)] {
        committed(
            &kernel,
            &transaction(
                vec![GraphOperation::CreateNode(NodeDraft {
                    id: NodeId::mint(),
                    root_id: id(ROOT),
                    type_id: id(TAG),
                    canonical_name: format!("tag {n}"),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                })],
                &[],
            ),
            time,
        );
    }
    let head = kernel.snapshot().unwrap();
    assert_eq!(head.revision, RevisionNumber::new(6), "{at}");
    assert_eq!(head.superseded.len(), 2, "{at}");
    assert!(
        head.superseded
            .values()
            .all(|record| record.merge.is_some()),
        "{at}"
    );
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
}
