//! Adversary pass 2, wave 2026-10-19a, `story:supersede-node`: the fix of pass 1's F1 beyond the
//! case that exposed it. Fix `35fe08743` says a `NodeRef` "at any depth (node and edge
//! properties, UpdateProperty values, a value object, Invoke arguments)" naming the node its own
//! transaction supersedes is `superseded-node-named-in-transaction`, as the same value one
//! transaction later is `node-superseded`. Pass 1's cases reach only a top-level `NodeRef` of
//! an `UpdateProperty` and a `CreateNode`; these reach every other position the fix names, nested
//! in a `List` and a `Record`, each with a control that the operation alone validates, so a
//! refusal is the supersession's and not a malformed operation's. On both providers.
//!
//! The seed is `supersession-seed.yaml` with three WorkItem properties (`refs`, a list of
//! MergeRequest references; `pair`, a record holding one; `relates`, one), a WorkItem operation
//! `link` taking a MergeRequest reference, and a TAGGED edge property `via`, a MergeRequest
//! reference.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, GraphOperation,
    GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Runtime, SeedDocument,
    ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use serde::Serialize;

const SEED: &str = include_str!("fixtures/supersession-seed.yaml");
const TENANT: &str = "test";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const WORK_ITEM: &str = "00000000-0000-4000-8000-000000000202";
const MERGE_REQUEST: &str = "00000000-0000-4000-8000-000000000203";
const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
const REFS: &str = "00000000-0000-4000-8000-000000000a15";
const PAIR: &str = "00000000-0000-4000-8000-000000000a16";
const RELATES: &str = "00000000-0000-4000-8000-000000000a17";
const VIA: &str = "00000000-0000-4000-8000-000000000a18";
/// MergeRequest `group/app!18`, the node superseded.
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
/// Tag `parser`.
const PARSER: &str = "00000000-0000-4000-8000-000000000303";
/// WorkItem `tracker/18`, the node every case writes on.
const LATER: &str = "00000000-0000-4000-8000-000000000304";
/// MergeRequest `group/app!19`, the node kept.
const OTHER: &str = "00000000-0000-4000-8000-000000000305";
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const STATEMENT_TEXT: &str = "group/app!18 is one merge request, tracked as one work item.";
const NAMED_IN_TRANSACTION: &str = "superseded-node-named-in-transaction";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
    }
}

fn anchor() -> AuthorityStateV1 {
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

fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

/// A MergeRequest reference type, as the seed writes one.
fn reference_type(indent: &str) -> String {
    format!(
        "{indent}value_kind: NodeRef\n{indent}parameters:\n{indent}  allowed_types:\n{indent}  - \
         {MERGE_REQUEST}\n"
    )
}

/// The fixture seed with the WorkItem properties and operation and the TAGGED property, and the
/// statement retained.
fn seed() -> SeedDocument {
    let work_item = "        constraints: []\n    abstract_type: false\n    lifecycle: null\n    \
                     operations: {}\n  - id: 00000000-0000-4000-8000-000000000203";
    assert_eq!(SEED.matches(work_item).count(), 1, "WorkItem ends once");
    let refs = format!(
        "        constraints: []
      {REFS}:
        id: {REFS}
        name: refs
        value_type:
          value_kind: List
          parameters:
{}        cardinality: One
        required: false
        constraints: []
      {PAIR}:
        id: {PAIR}
        name: pair
        value_type:
          value_kind: Record
          parameters:
            mr:
{}        cardinality: One
        required: false
        constraints: []
      {RELATES}:
        id: {RELATES}
        name: relates
        value_type:
{}        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations:
      link:
        name: link
        arguments:
          target:
{}        preconditions: []
        transition: null
        emits: []
  - id: 00000000-0000-4000-8000-000000000203",
        reference_type("            "),
        reference_type("              "),
        reference_type("          "),
        reference_type("            "),
    );
    let tagged = "        constraints: []\n    inverse: null";
    assert_eq!(
        SEED.matches(tagged).count(),
        1,
        "TAGGED's properties end once"
    );
    let via = format!(
        "        constraints: []
      {VIA}:
        id: {VIA}
        name: via
        value_type:
{}        cardinality: One
        required: false
        constraints: []
    inverse: null",
        reference_type("          "),
    );
    let text = SEED.replace(work_item, &refs).replace(tagged, &via);
    let mut document = SeedDocument::from_yaml(&text).unwrap();
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

fn transaction(operations: Vec<GraphOperation>, evidence: &[&str]) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: evidence.iter().map(|text| id(text)).collect(),
        schema_version: None,
    }
}

fn supersede() -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node: id(DUPLICATE),
        by: id(OTHER),
        reason: "recorded twice".to_owned(),
        evidence: BTreeSet::from([id::<EvidenceId>(STATEMENT)]),
    })
}

/// `node` referenced: a `NodeRef` to it.
fn to(node: &str) -> Value {
    Value::NodeRef(id(node))
}

/// A `refs` assertion about LATER whose object is the list `[NodeRef node]`, citing the
/// statement.
fn asserted(node: &str) -> GraphOperation {
    let yaml = format!(
        "id: {}
root_id: {ROOT}
subject: !Node {LATER}
predicate: !Property {REFS}
object: !Value
  value_kind: List
  value:
  - value_kind: NodeRef
    value: {node}
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
        AssertionId::mint(),
        context().operator
    );
    GraphOperation::AddAssertion(Box::new(serde_yaml_ng::from_str(&yaml).unwrap()))
}

/// Every position the fix names, each holding a reference to `node`, with the evidence its
/// transaction lists.
fn positions(node: &str) -> Vec<(&'static str, GraphOperation, &'static [&'static str])> {
    let mut created = NodeDraft {
        id: NodeId::mint(),
        root_id: id(ROOT),
        type_id: id(WORK_ITEM),
        canonical_name: "created".to_owned(),
        properties: BTreeMap::new(),
        aliases: Vec::new(),
    };
    created.properties.insert(
        id(PAIR),
        vec![Value::Record(BTreeMap::from([("mr".to_owned(), to(node))]))],
    );
    let mut edge = EdgeDraft {
        id: EdgeId::mint(),
        root_id: id(ROOT),
        type_id: id(TAGGED),
        source: id(LATER),
        target: id(PARSER),
        properties: BTreeMap::new(),
    };
    edge.properties.insert(id(VIA), vec![to(node)]);
    vec![
        (
            "an UpdateProperty list element",
            GraphOperation::UpdateProperty(PropertyMutation {
                node: id(LATER),
                property: id(REFS),
                values: vec![Value::List(vec![to(node)])],
            }),
            &[],
        ),
        (
            "an UpdateProperty record field",
            GraphOperation::UpdateProperty(PropertyMutation {
                node: id(LATER),
                property: id(PAIR),
                values: vec![Value::Record(BTreeMap::from([("mr".to_owned(), to(node))]))],
            }),
            &[],
        ),
        (
            "a CreateNode record field",
            GraphOperation::CreateNode(created),
            &[],
        ),
        (
            "a CreateEdge property",
            GraphOperation::CreateEdge(edge),
            &[],
        ),
        (
            "an Invoke argument",
            GraphOperation::Invoke {
                node: id(LATER),
                operation: "link".into(),
                arguments: BTreeMap::from([("target".to_owned(), to(node))]),
            },
            &[],
        ),
        (
            "an AddAssertion value object's list element",
            asserted(node),
            &[STATEMENT],
        ),
    ]
}

/// Proposes and validates `tx` at the head, committing nothing.
fn verdict(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap()
}

fn codes(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => Vec::new(),
    }
}

fn each(mut run: impl FnMut(&str, &Runtime)) {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
        run(if file { "file" } else { "sqlite" }, &kernel);
    }
}

/// Each position's operation validates alone when its reference names a node the head holds
/// (the control); beside a `SupersedeNode` of the node it names it is refused as
/// `superseded-node-named-in-transaction`; and once that supersession has committed, the same
/// operation alone is refused as `node-superseded`.
#[test]
fn adv2_a_node_ref_in_every_value_position_and_depth_names_the_node_it_references() {
    each(|provider, kernel| {
        let mut at = 100;
        for (what, operation, evidence) in positions(DUPLICATE) {
            at += 10;
            let alone = verdict(kernel, &transaction(vec![operation.clone()], evidence), at);
            assert!(
                matches!(alone, ValidationCommandResult::Validated(_)),
                "{provider}: {what}: the control does not validate: {:?}",
                codes(&alone)
            );
            at += 10;
            let mut listed: Vec<&str> = evidence.to_vec();
            if !listed.contains(&STATEMENT) {
                listed.push(STATEMENT);
            }
            let together = verdict(
                kernel,
                &transaction(vec![supersede(), operation], &listed),
                at,
            );
            assert!(
                codes(&together)
                    .contains(&(ValidatorName::Structural, NAMED_IN_TRANSACTION.into())),
                "{provider}: {what} beside the supersession of the node it references: {:?}",
                codes(&together)
            );
        }
        let tx = transaction(vec![supersede()], &[STATEMENT]);
        let committed = verdict(kernel, &tx, 1_000);
        assert!(
            matches!(committed, ValidationCommandResult::Validated(_)),
            "{provider}: {:?}",
            codes(&committed)
        );
        let receipt = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(1_002))
            .unwrap();
        assert!(
            matches!(receipt, CommitCommandResult::Committed(_)),
            "{receipt:?}"
        );
        let mut at = 2_000;
        for (what, operation, evidence) in positions(DUPLICATE) {
            at += 10;
            let later = verdict(kernel, &transaction(vec![operation], evidence), at);
            assert!(
                codes(&later).contains(&(ValidatorName::Reference, "node-superseded".into())),
                "{provider}: {what} after the supersession: {:?}",
                codes(&later)
            );
        }
    });
}
