//! A v1 store written by the base kernel of wave p5-01 (`cee0cae`) still replays: correction 1 on
//! unit `p5-01-kernel`, finding 1.
//!
//! `fixtures/base-cee0cae-v1-ownerless-modify-property.sqlite` is a SQLite-provider store written
//! by the kernel at `cee0cae`, under validation profile v1, by the documented P1 workflow: seed
//! `fixtures/seed-minimal-v2.yaml`, propose one `!ModifyProperty` in the P1 shape (the bare
//! declaration, no owner), validate it and retain `unsupported-operation` /
//! `ModifyProperty is not supported in P1`. It was written with the adversary's base writer
//! (`adversary_p5_01_kernel.rs` holds the file-provider twin), run on SQLite from a
//! `git archive cee0cae` copy. The base kernel reopened it before it was copied here.
//!
//! The P1 shape therefore still parses, encodes to exactly the bytes it had at `cee0cae`, and is
//! refused under v1 exactly as it was; profile v2 refuses it with its own issue.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, PropertyId, Timestamp, TransactionId};
use ekr_graph::CanonicalValue;
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, GraphOperation, GraphTransaction,
    PropertyModification, Runtime, TransactionDocument, TransactionState, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::{PropertyDefinition, ValueType};

const BASE_SQLITE_STORE: &[u8] =
    include_bytes!("fixtures/base-cee0cae-v1-ownerless-modify-property.sqlite");

/// The P1-shape `ModifyProperty` proposal the store retains.
const P1_PROPOSAL: &str = "00000000-0000-4000-8000-0000000000f1";

/// The proposal carrying all twelve operation kinds in their base-era document shapes.
const ALL_KINDS_PROPOSAL: &str = "00000000-0000-4000-8000-0000000003f0";

/// The document the base writer proposed, byte for byte.
const BASE_DOCUMENT: &str = "format: ekr.transaction-document/1
transaction:
  id: 00000000-0000-4000-8000-0000000000f1
  proposer: 00000000-0000-4000-8000-000000000003
  operations:
  - !ModifyProperty
    id: 00000000-0000-4000-8000-0000000000f2
    name: note
    value_type:
      value_kind: String
    cardinality: One
    required: false
    constraints: []
  evidence: []
";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn v1() -> AuthorityStateV1 {
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

fn base_store() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("state.db"), BASE_SQLITE_STORE).unwrap();
    directory
}

fn open(directory: &tempfile::TempDir) -> Runtime {
    Runtime::sqlite(&directory.path().join("state.db"), "test", context(), v1())
        .expect("the base-era SQLite store opens")
}

/// The base-era SQLite store reopens under v1 with its one retained rejection, unchanged, and a
/// second P1-shape proposal is refused the same way and survives a further reopening.
#[test]
fn a_base_era_v1_sqlite_store_holding_an_ownerless_modify_property_rejection_still_reopens() {
    let directory = base_store();
    let kernel = open(&directory);
    let head = kernel.head().expect("the base-era SQLite store replays");
    let records = kernel.transactions().unwrap();
    assert_eq!(records.len(), 2);
    let record = &records[&P1_PROPOSAL.parse().unwrap()];
    assert_eq!(record.state(), TransactionState::Rejected);
    let issues = &record.rejection.as_ref().unwrap().issues;
    assert_eq!(
        issues
            .iter()
            .map(|issue| (issue.code.as_str(), issue.message.as_str()))
            .collect::<Vec<_>>(),
        vec![(
            "unsupported-operation",
            "ModifyProperty is not supported in P1"
        )]
    );
    assert_eq!(record.proposal.document_bytes, BASE_DOCUMENT.as_bytes());

    let again = BASE_DOCUMENT.replace(
        "00000000-0000-4000-8000-0000000000f1",
        "00000000-0000-4000-8000-0000000000f3",
    );
    kernel
        .propose(again.as_bytes(), context().operator, || {
            Timestamp::from_millis(30)
        })
        .unwrap();
    let revision = head.unwrap().revision;
    let verdict = kernel
        .validate(
            "00000000-0000-4000-8000-0000000000f3".parse().unwrap(),
            revision,
            || Timestamp::from_millis(31),
        )
        .unwrap();
    let ValidationCommandResult::Rejected(rejection) = verdict else {
        panic!("v1 refuses a P1-shape ModifyProperty")
    };
    assert_eq!(rejection.issues[0].code, "unsupported-operation");
    let records = kernel.transactions().unwrap();
    drop(kernel);
    assert_eq!(open(&directory).transactions().unwrap(), records);
}

/// The P1 shape parses to an ownerless modification whose canonical transaction hash is the one
/// the base kernel retained, and it serializes back to the P1 shape.
#[test]
fn the_p1_shape_encodes_exactly_as_the_base_kernel_encoded_it() {
    let directory = base_store();
    let retained = open(&directory)
        .transactions()
        .unwrap()
        .remove(&P1_PROPOSAL.parse().unwrap())
        .unwrap()
        .proposal;

    let parsed = TransactionDocument::parse(BASE_DOCUMENT.as_bytes()).unwrap();
    let GraphOperation::ModifyProperty(modification) = &parsed.transaction().operations[0] else {
        panic!("a ModifyProperty")
    };
    assert_eq!(
        modification,
        &PropertyModification {
            owner: None,
            property: PropertyDefinition::new(
                "00000000-0000-4000-8000-0000000000f2"
                    .parse::<PropertyId>()
                    .unwrap(),
                "note",
                ValueType::String,
            ),
        }
    );
    let canonical =
        GraphTransaction::<CanonicalValue>::try_from(parsed.transaction().clone()).unwrap();
    assert_eq!(
        Some(ContentHash::of(&canonical)),
        retained.canonical_transaction_hash
    );
    assert_eq!(
        Some(ContentHash::of(&canonical.operations)),
        retained.canonical_operations_hash
    );
    assert_eq!(
        canonical.id,
        "00000000-0000-4000-8000-0000000000f1"
            .parse::<TransactionId>()
            .unwrap()
    );

    let written = serde_yaml_ng::to_string(&parsed.transaction().operations).unwrap();
    assert!(!written.contains("owner"), "{written}");
    let reread: Vec<GraphOperation> = serde_yaml_ng::from_str(&written).unwrap();
    assert_eq!(reread, parsed.transaction().operations);
}

/// One `ModifyProperty` is either shape, never both: the P1 fields beside `owner` or `property`
/// are refused, as is an unknown key and a P1 shape missing its value type.
#[test]
fn a_modify_property_mixing_the_two_shapes_is_refused() {
    let document = |operation: &str| {
        format!(
            "format: ekr.transaction-document/1\ntransaction:\n  id: \
             00000000-0000-4000-8000-0000000000f1\n  proposer: \
             00000000-0000-4000-8000-000000000003\n  operations:\n  - {operation}\n  evidence: \
             []\n"
        )
    };
    const ID: &str = "00000000-0000-4000-8000-0000000000f2";
    for operation in [
        format!(
            "!ModifyProperty {{owner: {ID}, id: {ID}, name: note, value_type: {{value_kind: \
             String}}}}"
        ),
        format!(
            "!ModifyProperty {{property: {{id: {ID}, name: note, value_type: {{value_kind: \
             String}}}}, name: note}}"
        ),
        format!("!ModifyProperty {{owner: {ID}}}"),
        format!("!ModifyProperty {{id: {ID}, name: note}}"),
        format!("!ModifyProperty {{id: {ID}, name: note, value_type: {{value_kind: String}}, unknown: 1}}"),
    ] {
        assert!(
            TransactionDocument::parse(document(&operation).as_bytes()).is_err(),
            "{operation}"
        );
    }
}

/// The class of finding 1 is every operation kind, not `ModifyProperty` alone: a base-era document
/// shape that no longer parses, or that encodes differently, breaks replay of any v1 store that
/// retains it. The fixture retains one proposal carrying all twelve kinds in the shapes the base
/// kernel wrote, and its rejection; each still parses, re-derives the canonical hashes the base
/// kernel retained, and revalidates to the same issues (the reopening above compares them).
#[test]
fn every_operation_kind_in_its_base_era_shape_re_derives_its_retained_hashes() {
    let directory = base_store();
    let kernel = open(&directory);
    let record = kernel
        .transactions()
        .unwrap()
        .remove(&ALL_KINDS_PROPOSAL.parse().unwrap())
        .expect("the all-kinds proposal is retained");
    assert_eq!(record.state(), TransactionState::Rejected);
    let parsed = TransactionDocument::parse(&record.proposal.document_bytes).unwrap();
    let kinds: BTreeSet<&str> = parsed
        .transaction()
        .operations
        .iter()
        .map(|operation| match operation {
            GraphOperation::CreateNode(_) => "CreateNode",
            GraphOperation::UpdateProperty(_) => "UpdateProperty",
            GraphOperation::CreateEdge(_) => "CreateEdge",
            GraphOperation::DeleteEdge(_) => "DeleteEdge",
            GraphOperation::AddAssertion(_) => "AddAssertion",
            GraphOperation::RetractAssertion(_) => "RetractAssertion",
            GraphOperation::DefineNodeType(_) => "DefineNodeType",
            GraphOperation::DefineEdgeType(_) => "DefineEdgeType",
            GraphOperation::ModifyProperty(_) => "ModifyProperty",
            GraphOperation::MergeEntity(_) => "MergeEntity",
            GraphOperation::Invoke { .. } => "Invoke",
            GraphOperation::SupersedeAssertion(_) => "SupersedeAssertion",
        })
        .collect();
    assert_eq!(kinds.len(), 12, "{kinds:?}");
    let canonical =
        GraphTransaction::<CanonicalValue>::try_from(parsed.transaction().clone()).unwrap();
    assert_eq!(
        Some(ContentHash::of(&canonical)),
        record.proposal.canonical_transaction_hash
    );
    assert_eq!(
        Some(ContentHash::of(&canonical.operations)),
        record.proposal.canonical_operations_hash
    );
    let codes: Vec<&str> = record
        .rejection
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect();
    assert!(codes.contains(&"unsupported-operation"), "{codes:?}");
}
