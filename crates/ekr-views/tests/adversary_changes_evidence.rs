//! Adversary pass 1 on `task:changes-since-lists-added-evidence` (wave reads-04, unit K).
//!
//! A store of its own, on both providers: a seed holding one node and one retained statement,
//! then
//!
//! * revision 1: three `AddEvidence` (listed out of id order, one statement recording no
//!   identity), a node and an assertion about it citing one of them;
//! * revision 2: an assertion citing evidence revision 1 added;
//! * revision 3: one `AddEvidence` alone, cited by nothing;
//! * revision 4: an assertion citing revision 3's evidence and the seed's.
//!
//! Every expectation is written from `views.yaml` (`ekr.views.GraphChange`,
//! `ekr.views.ChangesSince`), never read back from the code under test.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, PropertyId, RevisionNumber, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    CommitCommandResult, EvidenceAddition, GraphOperation, GraphTransaction, NodeDraft, Runtime,
    SeedDocument, ValidationCommandResult,
};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{ChangesListed, ChangesRequest, Index, SinceKind};
use serde::Serialize;
use serde_json::{json, Value as Json};
use support::fixtures::{self, id, Provider};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

const SUBJECT: u64 = 0x5;
const LABEL: u64 = 0x6;
const SEEDED_NODE: u64 = 0x10;
const ADDED_NODE: u64 = 0x11;
const SEEDED_STATEMENT: u64 = 0xe0;
/// Revision 1's evidence, by the order the transaction lists them: b, a, c.
const X_A: u64 = 0xe1;
const X_B: u64 = 0xe2;
const X_C: u64 = 0xe3;
/// Revision 3's evidence.
const X_D: u64 = 0xe4;
const CLAIM_1: u64 = 0xa1;
const CLAIM_2: u64 = 0xa2;
const CLAIM_4: u64 = 0xa4;
const SEEDED_AT_MS: i64 = 1_000;
const CLAIM_2_VALID_MS: i64 = 5_000;
const CLAIM_4_VALID_MS: i64 = 7_000;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// When revision `n` (1 or more) was committed.
fn committed(n: i64) -> i64 {
    SEEDED_AT_MS + 10 * n + 3
}

fn payload(n: u64) -> Vec<u8> {
    format!("adversary statement {n:x}").into_bytes()
}

fn hash_of(n: u64) -> String {
    ContentHash::of_bytes(&payload(n)).to_string()
}

fn statement(n: u64, identity: Option<&str>) -> Evidence {
    Evidence {
        id: id(n),
        source: EvidenceSource::HumanStatement {
            identity: identity.map(str::to_owned),
        },
        content_hash: ContentHash::of_bytes(&payload(n)),
        extracted_by: fixtures::context().operator,
        observed_at: Timestamp::from_millis(900),
        confidence: Confidence::CERTAIN,
    }
}

fn added(n: u64, identity: Option<&str>) -> GraphOperation {
    GraphOperation::AddEvidence(Box::new(EvidenceAddition {
        evidence: statement(n, identity),
        payload: payload(n),
    }))
}

fn claim(n: u64, about: u64, valid_from: Option<i64>, cited: &[u64]) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(Assertion {
        id: id::<AssertionId>(n),
        root_id: id(2),
        subject: Subject::Node(id::<NodeId>(about)),
        predicate: Predicate::Property(id::<PropertyId>(LABEL)),
        object: Object::Value(Value::String(format!("claim {n:x}"))),
        evidence: cited.iter().map(|n| id::<EvidenceId>(*n)).collect(),
        proposed_by: fixtures::context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: valid_from.map_or(TemporalRange::UNBOUNDED, |from| {
            TemporalRange::since(Timestamp::from_millis(from))
        }),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }))
}

const SEED: &str = "format: ekr-seed/2
ontology:
  version:
    id: 00000000-0000-4000-8000-000000000001
    number: 0
    parent: null
    created_at: 0
  node_types: []
  edge_types: []
graph:
  format: ekr.graph-document/2
  graph:
    root:
      id: 00000000-0000-4000-8000-000000000002
      space: Canonical
      schema_version_id: 00000000-0000-4000-8000-000000000001
      parent: null
      created_at: 0
    revision: 0
    nodes: {}
    edges: {}
    assertions: {}
    evidence: {}
evidence_payloads: {}
";

fn commit(runtime: &Runtime, revision: u64, operations: Vec<GraphOperation>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let evidence: BTreeSet<EvidenceId> = operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let transaction = GraphTransaction {
        id: id::<TransactionId>(0x7000 + revision),
        proposer: fixtures::context().operator,
        operations,
        evidence,
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .expect("a transaction document");
    let base = SEEDED_AT_MS + 10 * i64::try_from(revision).expect("small");
    runtime
        .propose(bytes.as_bytes(), fixtures::context().operator, || {
            Timestamp::from_millis(base + 1)
        })
        .expect("proposed");
    let head = runtime.head().expect("head").expect("seeded").revision;
    let verdict = runtime
        .validate(transaction.id, head, || Timestamp::from_millis(base + 2))
        .expect("validation runs");
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "revision {revision} validates: {verdict:?}"
    );
    let result = runtime
        .commit(transaction.id, fixtures::context().operator, || {
            Timestamp::from_millis(base + 3)
        })
        .expect("commit runs");
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}

fn build(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    let mut document = SeedDocument::from_yaml(SEED).expect("the seed parses");
    let mut subject = NodeType::new(id::<TypeId>(SUBJECT), "Subject");
    subject.properties.insert(
        id(LABEL),
        PropertyDefinition::new(id(LABEL), "label", ValueType::String),
    );
    document.ontology.node_types.push(subject);
    let node = Node::<Value>::new(
        id(SEEDED_NODE),
        document.graph.root.id,
        id(SUBJECT),
        "seeded",
    );
    document.graph.nodes.insert(node.id, node);
    let seeded = statement(SEEDED_STATEMENT, Some("seeder"));
    document
        .evidence_payloads
        .insert(seeded.content_hash, payload(SEEDED_STATEMENT).into());
    document.graph.evidence.insert(seeded.id, seeded);
    runtime
        .seed(document, || Timestamp::from_millis(SEEDED_AT_MS))
        .expect("the seed is admitted");

    commit(
        &runtime,
        1,
        vec![
            added(X_B, Some("b-stater")),
            claim(CLAIM_1, ADDED_NODE, None, &[X_A]),
            added(X_A, None),
            GraphOperation::CreateNode(NodeDraft {
                id: id(ADDED_NODE),
                root_id: id(2),
                type_id: id(SUBJECT),
                canonical_name: "added".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            added(X_C, Some("C \"quoted\" stäter")),
        ],
    );
    commit(
        &runtime,
        2,
        vec![claim(CLAIM_2, SEEDED_NODE, Some(CLAIM_2_VALID_MS), &[X_B])],
    );
    commit(&runtime, 3, vec![added(X_D, Some("later"))]);
    commit(
        &runtime,
        4,
        vec![claim(
            CLAIM_4,
            ADDED_NODE,
            Some(CLAIM_4_VALID_MS),
            &[X_D, SEEDED_STATEMENT],
        )],
    );
    (work, runtime)
}

fn read(
    runtime: &Runtime,
    at: u64,
    kind: SinceKind,
    since: i64,
    limit: Option<i64>,
    after: Option<i64>,
) -> (Vec<u8>, Json, ChangesListed) {
    let index = Index::load(runtime, Some(RevisionNumber::new(at))).expect("the revision indexes");
    let request = ChangesRequest::new(kind, since, limit, after).expect("within bounds");
    let answer = index.changes(runtime, &request).expect("the changes read");
    let value: Json = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

fn listed(value: &Json) -> Vec<Json> {
    value["changes"].as_array().expect("changes").clone()
}

fn evidence_added(revision: i64, n: u64, locator: &str) -> Json {
    json!({"revision": revision, "recorded_at": committed(revision), "change": "EvidenceAdded",
           "id": uuid(n), "locator": locator, "content_hash": hash_of(n), "evidence": []})
}

/// Every change after the seed, as `views.yaml` orders them: by revision, then kind in the
/// declared order (EvidenceAdded last), then id as text — not the order the transaction lists
/// its operations.
fn after_the_seed() -> Vec<Json> {
    vec![
        json!({"revision": 1, "recorded_at": committed(1), "change": "NodeCreated",
               "id": uuid(ADDED_NODE), "type": uuid(SUBJECT), "name": "added",
               "evidence": [uuid(X_A)]}),
        json!({"revision": 1, "recorded_at": committed(1), "change": "AssertionAdded",
               "id": uuid(CLAIM_1), "subject_kind": "Node", "subject": uuid(ADDED_NODE),
               "evidence": [uuid(X_A)]}),
        evidence_added(1, X_A, ""),
        evidence_added(1, X_B, "b-stater"),
        evidence_added(1, X_C, "C \"quoted\" stäter"),
        json!({"revision": 2, "recorded_at": committed(2), "change": "AssertionAdded",
               "id": uuid(CLAIM_2), "subject_kind": "Node", "subject": uuid(SEEDED_NODE),
               "valid_time": CLAIM_2_VALID_MS, "evidence": [uuid(X_B)]}),
        evidence_added(3, X_D, "later"),
        json!({"revision": 4, "recorded_at": committed(4), "change": "AssertionAdded",
               "id": uuid(CLAIM_4), "subject_kind": "Node", "subject": uuid(ADDED_NODE),
               "valid_time": CLAIM_4_VALID_MS,
               "evidence": [uuid(SEEDED_STATEMENT), uuid(X_D)]}),
    ]
}

/// Several `AddEvidence` in one transaction, listed out of id order, each appear once after
/// the revision's assertion changes in id order; evidence cited only in a later revision is an
/// EvidenceAdded of the revision that added it and nowhere else; a statement recording no
/// identity has the empty locator `views.yaml` names; the seed's statement is no change.
#[test]
fn several_add_evidence_in_one_revision_follow_its_assertions_once_each_in_id_order() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (_, value, summary) = read(&runtime, 4, SinceKind::Revision, 0, None, None);
        assert_eq!(listed(&value), after_the_seed(), "{provider:?}: {value}");
        assert_eq!(summary.evidence_added, 4, "{provider:?}");
        assert_eq!(summary.assertions_added, 3, "{provider:?}");
        assert_eq!(summary.nodes_created, 1, "{provider:?}");

        let (_, whole, _) = read(
            &runtime,
            4,
            SinceKind::TransactionTime,
            i64::MIN,
            None,
            None,
        );
        let whole = listed(&whole);
        assert!(
            whole
                .iter()
                .all(|change| change["id"] != uuid(SEEDED_STATEMENT)),
            "{provider:?}: the seed's statement is listed as a change: {whole:?}"
        );
        assert_eq!(whole[1..], after_the_seed()[..], "{provider:?}");
    }
}

/// Every page size from 1 to the total, paged by `next`, delivers every change once and in
/// order, however a page boundary splits revision 1's five changes; each page's summary counts
/// exactly its own EvidenceAdded and names its own first and last revision.
#[test]
fn every_page_size_splits_a_revision_of_added_evidence_without_loss_or_repetition() {
    let expected = after_the_seed();
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        for limit in 1..=i64::try_from(expected.len()).expect("small") {
            let mut paged = Vec::new();
            let mut after = 0;
            loop {
                let (_, page, summary) = read(
                    &runtime,
                    4,
                    SinceKind::Revision,
                    0,
                    Some(limit),
                    Some(after),
                );
                let changes = listed(&page);
                let counted = changes
                    .iter()
                    .filter(|change| change["change"] == "EvidenceAdded")
                    .count() as u64;
                assert_eq!(
                    summary.evidence_added, counted,
                    "{provider:?}: limit {limit} after {after}"
                );
                assert_eq!(
                    summary.first_revision,
                    changes.first().and_then(|c| c["revision"].as_u64())
                );
                assert_eq!(
                    summary.last_revision,
                    changes.last().and_then(|c| c["revision"].as_u64())
                );
                assert_eq!(page["meta"]["total"], expected.len());
                paged.extend(changes);
                match page["next"].as_i64() {
                    Some(next) => after = next,
                    None => break,
                }
            }
            assert_eq!(paged, expected, "{provider:?}: limit {limit}");
        }
    }
}

/// A since exactly at the revision that added evidence chooses the revisions after it, so not
/// that evidence; one below chooses it. The same holds for its commit time, and for `at` from
/// the other side.
#[test]
fn a_since_or_at_on_the_add_evidence_revision_chooses_by_its_boundary() {
    let kinds = |value: &Json| -> Vec<(u64, String)> {
        listed(value)
            .iter()
            .filter(|change| change["change"] == "EvidenceAdded")
            .map(|change| {
                (
                    change["revision"].as_u64().expect("revision"),
                    change["id"].as_str().expect("id").to_owned(),
                )
            })
            .collect()
    };
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (_, value, _) = read(&runtime, 4, SinceKind::Revision, 3, None, None);
        assert_eq!(kinds(&value), vec![], "{provider:?}: since 3");
        let (_, value, _) = read(&runtime, 4, SinceKind::Revision, 2, None, None);
        assert_eq!(kinds(&value), vec![(3, uuid(X_D))], "{provider:?}: since 2");
        let (_, value, _) = read(
            &runtime,
            4,
            SinceKind::TransactionTime,
            committed(3),
            None,
            None,
        );
        assert_eq!(
            kinds(&value),
            vec![],
            "{provider:?}: since revision 3's time"
        );
        let (_, value, _) = read(
            &runtime,
            4,
            SinceKind::TransactionTime,
            committed(3) - 1,
            None,
            None,
        );
        assert_eq!(
            kinds(&value),
            vec![(3, uuid(X_D))],
            "{provider:?}: since just before revision 3's time"
        );
        let (_, value, _) = read(&runtime, 2, SinceKind::Revision, 0, None, None);
        assert_eq!(
            kinds(&value),
            vec![(1, uuid(X_A)), (1, uuid(X_B)), (1, uuid(X_C))],
            "{provider:?}: at 2"
        );
        let (_, value, _) = read(&runtime, 3, SinceKind::Revision, 1, None, None);
        assert_eq!(kinds(&value), vec![(3, uuid(X_D))], "{provider:?}: 1 to 3");
    }
}

/// A valid-time since chooses assertion changes only: here exactly the two with a valid time
/// after it, and no evidence, whatever revisions added some.
#[test]
fn a_valid_time_since_chooses_no_evidence_over_revisions_that_add_it() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (_, value, summary) = read(&runtime, 4, SinceKind::ValidTime, i64::MIN, None, None);
        let ids: Vec<&str> = value["changes"]
            .as_array()
            .expect("changes")
            .iter()
            .map(|change| change["id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids, [uuid(CLAIM_2), uuid(CLAIM_4)], "{provider:?}: {value}");
        assert_eq!(summary.evidence_added, 0, "{provider:?}");
    }
}

/// The file and SQLite providers answer the same bytes for every revision since and every
/// transaction-time since, at every revision.
#[test]
fn the_providers_answer_the_same_bytes_for_every_range_holding_added_evidence() {
    let (_file_work, file) = build(Provider::File);
    let (_sqlite_work, sqlite) = build(Provider::Sqlite);
    for at in 0..=4_u64 {
        let mut sinces: Vec<(SinceKind, i64)> = (0..=4).map(|n| (SinceKind::Revision, n)).collect();
        sinces.extend((1..=4).map(|n| (SinceKind::TransactionTime, committed(n) - 1)));
        sinces.push((SinceKind::TransactionTime, i64::MIN));
        sinces.push((SinceKind::ValidTime, i64::MIN));
        for (kind, since) in sinces {
            for limit in [None, Some(2)] {
                let (left, ..) = read(&file, at, kind, since, limit, None);
                let (right, ..) = read(&sqlite, at, kind, since, limit, None);
                assert_eq!(left, right, "at {at}, {kind:?} {since}, limit {limit:?}");
            }
        }
    }
}

/// A range holding no `AddEvidence` answers the bytes `views.yaml` gave it before EvidenceAdded
/// existed, written out here by hand: no `locator` or `content_hash` key on any other kind, and
/// the seed's statement no change.
#[test]
fn a_range_without_an_add_evidence_answers_its_documented_bytes_exactly() {
    let second = format!(
        "{{\"meta\":{{\"format\":\"ekr.graph-changes/1\",\"revision\":2,\"since_revision\":1,\
         \"limit\":500,\"after\":0,\"total\":1}},\"changes\":[{{\"revision\":2,\
         \"recorded_at\":{},\"change\":\"AssertionAdded\",\"id\":\"{}\",\"subject_kind\":\"Node\",\
         \"subject\":\"{}\",\"valid_time\":{CLAIM_2_VALID_MS},\"evidence\":[\"{}\"]}}],\
         \"remaining\":0}}",
        committed(2),
        uuid(CLAIM_2),
        uuid(SEEDED_NODE),
        uuid(X_B)
    );
    let seed = format!(
        "{{\"meta\":{{\"format\":\"ekr.graph-changes/1\",\"revision\":0,\"since_recorded\":0,\
         \"limit\":500,\"after\":0,\"total\":1}},\"changes\":[{{\"revision\":0,\
         \"recorded_at\":{SEEDED_AT_MS},\"change\":\"NodeCreated\",\"id\":\"{}\",\"type\":\"{}\",\
         \"name\":\"seeded\",\"evidence\":[]}}],\"remaining\":0}}",
        uuid(SEEDED_NODE),
        uuid(SUBJECT)
    );
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (bytes, ..) = read(&runtime, 2, SinceKind::Revision, 1, None, None);
        assert_eq!(
            String::from_utf8(bytes).expect("UTF-8"),
            second,
            "{provider:?}"
        );
        let (bytes, ..) = read(&runtime, 0, SinceKind::TransactionTime, 0, None, None);
        assert_eq!(
            String::from_utf8(bytes).expect("UTF-8"),
            seed,
            "{provider:?}"
        );
    }
}
