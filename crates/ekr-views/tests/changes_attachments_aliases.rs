//! The change feed lists evidence attached to a held assertion and aliases a node gained (review
//! of 0.0.42, finding 22): an `!AttachEvidence` changes what `ekr explain` answers for the
//! assertion and an `!AddAlias` changes what `ekr resolve` answers for the name, so a reader that
//! follows `ChangesSince` instead of diffing snapshots must see both.
//!
//! A store of its own, on both providers. The seed: node type `Subject` (declaring `label`), one
//! node `held`, two statements `E0` and `E1`, and assertion `C0` about `held` citing `E0`.
//!
//! * revision 1: `E1`, which the seed retained, attached to `C0`, and nothing else;
//! * revision 2: alias `nickname` added to `held`, and nothing else;
//! * revision 3: evidence `X2` and `X3` added and both attached to `C0`; aliases `b-alias` and
//!   `a-alias` added to `held`.
//!
//! Every expectation is written from `views.yaml` (`ekr.views.GraphChange`), never read back
//! from the code under test.

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, PropertyId, RevisionNumber, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    AliasAddition, CommitCommandResult, EvidenceAddition, EvidenceAttachment, GraphOperation,
    GraphTransaction, Runtime, SeedDocument, ValidationCommandResult,
};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{ChangesListed, ChangesRequest, Index, SinceKind};
use serde::Serialize;
use serde_json::{json, Value as Json};

use support::fixtures::{self, context, id, Provider};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

const SUBJECT: u64 = 0xe8_0001;
const LABEL: u64 = 0xe8_0010;
const HELD: u64 = 0xe8_0100;
const E0: u64 = 0xe8_0300;
const E1: u64 = 0xe8_0301;
const X2: u64 = 0xe8_0302;
const X3: u64 = 0xe8_0303;
const C0: u64 = 0xe8_0400;
const TRANSACTIONS: u64 = 0xe8_0500;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// When revision `n` was committed: the seed takes one tick, each commit three.
fn committed(n: i64) -> i64 {
    fixtures::CLOCK_START_MS + 1 + 3 * n
}

fn statement(n: u64) -> (Evidence, Vec<u8>) {
    let payload = format!("statement {n:x}").into_bytes();
    (
        Evidence {
            id: id(n),
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: ContentHash::of_bytes(&payload),
            extracted_by: context().operator,
            observed_at: Timestamp::from_millis(1_000),
            confidence: Confidence::from_basis_points(9_000).expect("basis points"),
        },
        payload,
    )
}

fn added(n: u64) -> GraphOperation {
    let (evidence, payload) = statement(n);
    GraphOperation::AddEvidence(Box::new(EvidenceAddition { evidence, payload }))
}

fn attached(evidence: u64) -> GraphOperation {
    GraphOperation::AttachEvidence(EvidenceAttachment {
        assertion: id(C0),
        evidence: id(evidence),
    })
}

fn alias(text: &str) -> GraphOperation {
    GraphOperation::AddAlias(AliasAddition {
        node: id::<NodeId>(HELD),
        alias: text.to_owned(),
    })
}

struct Writer<'a> {
    runtime: &'a Runtime,
    clock: i64,
    transactions: u64,
}

impl Writer<'_> {
    fn tick(&mut self) -> Timestamp {
        self.clock += 1;
        Timestamp::from_millis(self.clock)
    }

    fn commit(&mut self, operations: Vec<GraphOperation>) {
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let evidence: BTreeSet<EvidenceId> = operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::AttachEvidence(attachment) => Some(attachment.evidence),
                _ => None,
            })
            .collect();
        self.transactions += 1;
        let transaction = GraphTransaction {
            id: id::<TransactionId>(self.transactions),
            proposer: context().operator,
            operations,
            evidence,
            schema_version: None,
        };
        let bytes = serde_yaml_ng::to_string(&Wire {
            format: "ekr.transaction-document/2",
            transaction: &transaction,
        })
        .expect("a transaction document")
        .into_bytes();
        let (proposed, validated, committed) = (self.tick(), self.tick(), self.tick());
        self.runtime
            .propose(&bytes, context().operator, || proposed)
            .expect("retained");
        let head = self.runtime.head().expect("head").expect("seeded").revision;
        let verdict = self
            .runtime
            .validate(transaction.id, head, || validated)
            .expect("validation runs");
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{verdict:?}"
        );
        let result = self
            .runtime
            .commit(transaction.id, context().operator, || committed)
            .expect("commit runs");
        assert!(
            matches!(result, CommitCommandResult::Committed(_)),
            "{result:?}"
        );
    }
}

fn build(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir"))
        .join("tests/fixtures/seed-empty.yaml");
    let mut document =
        SeedDocument::from_yaml(&std::fs::read_to_string(path).expect("the empty seed"))
            .expect("the empty seed parses");
    let root = document.graph.root.id;
    let mut subject = NodeType::new(id::<TypeId>(SUBJECT), "Subject");
    subject.properties.insert(
        id(LABEL),
        PropertyDefinition::new(id(LABEL), "label", ValueType::String),
    );
    document.ontology.node_types.push(subject);
    let node = Node::<Value>::new(id(HELD), root, id(SUBJECT), "held");
    document.graph.nodes.insert(node.id, node);
    for n in [E0, E1] {
        let (record, payload) = statement(n);
        document
            .evidence_payloads
            .insert(record.content_hash, payload.into());
        document.graph.evidence.insert(record.id, record);
    }
    let claim = Assertion::<Value> {
        id: id::<AssertionId>(C0),
        root_id: root,
        subject: Subject::Node(id(HELD)),
        predicate: Predicate::Property(id::<PropertyId>(LABEL)),
        object: Object::Value(Value::String("the label".into())),
        evidence: [id::<EvidenceId>(E0)].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(500)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    document.graph.assertions.insert(claim.id, claim);
    let mut writer = Writer {
        runtime: &runtime,
        clock: fixtures::CLOCK_START_MS,
        transactions: TRANSACTIONS,
    };
    let now = writer.tick();
    runtime
        .seed(document, || now)
        .expect("the seed is admitted");
    writer.commit(vec![attached(E1)]);
    writer.commit(vec![alias("nickname")]);
    writer.commit(vec![
        added(X3),
        attached(X3),
        alias("b-alias"),
        added(X2),
        attached(X2),
        alias("a-alias"),
    ]);
    (work, runtime)
}

fn read(runtime: &Runtime, at: u64, kind: SinceKind, since: i64) -> (Json, ChangesListed) {
    let index = Index::load(runtime, Some(RevisionNumber::new(at))).expect("the revision indexes");
    let request = ChangesRequest::new(kind, since, None, None).expect("in bounds");
    let answer = index.changes(runtime, &request).expect("answered");
    (
        serde_json::from_slice(&answer.bytes).expect("JSON"),
        answer.summary,
    )
}

fn attachment(revision: i64, evidence: &[u64]) -> Json {
    json!({
        "revision": revision,
        "recorded_at": committed(revision),
        "change": "EvidenceAttached",
        "id": uuid(C0),
        "subject_kind": "Node",
        "subject": uuid(HELD),
        "evidence": evidence.iter().map(|n| uuid(*n)).collect::<Vec<_>>(),
    })
}

fn aliases(revision: i64, added: &[&str]) -> Json {
    json!({
        "revision": revision,
        "recorded_at": committed(revision),
        "change": "AliasAdded",
        "id": uuid(HELD),
        "type": uuid(SUBJECT),
        "evidence": [],
        "aliases": added,
    })
}

fn evidence_added(revision: i64, n: u64) -> Json {
    json!({
        "revision": revision,
        "recorded_at": committed(revision),
        "change": "EvidenceAdded",
        "id": uuid(n),
        "locator": "operator",
        "content_hash": ContentHash::of_bytes(&statement(n).1).to_string(),
        "evidence": [],
    })
}

#[test]
fn an_attachment_of_retained_evidence_alone_is_listed_as_evidence_attached() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (value, summary) = read(&runtime, 1, SinceKind::Revision, 0);
        assert_eq!(
            value["changes"],
            json!([attachment(1, &[E1])]),
            "{provider:?}"
        );
        assert_eq!(
            (
                summary.changes,
                summary.evidence_attached,
                summary.aliases_added
            ),
            (1, 1, 0),
            "{provider:?}"
        );
    }
}

#[test]
fn an_alias_alone_is_listed_as_alias_added() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (value, summary) = read(&runtime, 2, SinceKind::Revision, 1);
        assert_eq!(
            value["changes"],
            json!([aliases(2, &["nickname"])]),
            "{provider:?}"
        );
        assert_eq!(
            (
                summary.changes,
                summary.evidence_attached,
                summary.aliases_added
            ),
            (1, 0, 1),
            "{provider:?}"
        );
    }
}

#[test]
fn one_revision_lists_one_change_per_assertion_and_per_node_with_their_ids_sorted() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (value, summary) = read(&runtime, 3, SinceKind::Revision, 2);
        assert_eq!(
            value["changes"],
            json!([
                evidence_added(3, X2),
                evidence_added(3, X3),
                attachment(3, &[X2, X3]),
                aliases(3, &["a-alias", "b-alias"]),
            ]),
            "{provider:?}"
        );
        assert_eq!(
            (
                summary.evidence_added,
                summary.evidence_attached,
                summary.aliases_added
            ),
            (2, 1, 1),
            "{provider:?}"
        );
        // Neither has a valid time, so a valid-time since chooses neither; the seed's claim, valid
        // from 500, is not after 500.
        let (valid, _) = read(&runtime, 3, SinceKind::ValidTime, 500);
        assert_eq!(valid["changes"], json!([]), "{provider:?}");
        // The lists are the same when a later index reads them.
        let (whole, _) = read(&runtime, 3, SinceKind::Revision, 0);
        assert_eq!(
            whole["changes"],
            json!([
                attachment(1, &[E1]),
                aliases(2, &["nickname"]),
                evidence_added(3, X2),
                evidence_added(3, X3),
                attachment(3, &[X2, X3]),
                aliases(3, &["a-alias", "b-alias"]),
            ]),
            "{provider:?}"
        );
    }
}
