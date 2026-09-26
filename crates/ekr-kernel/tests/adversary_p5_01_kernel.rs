//! Adversary pass 1 on unit K `p5-01-kernel` (wave p5-01, `story:schema-evolution-transactions`
//! parts B and C).
//!
//! The pass's base-era file-store case, which embedded a v1 file store written by the kernel at
//! `cee0cae`, is kept out of this tree: the organization's secret scan reads three idempotency
//! keys in that store as API keys and refuses the commit. It passed at `468162c` and is retained
//! with the wave's review record. `base_era_v1_replay.rs` holds the same claim in-tree on SQLite.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, SchemaVersionId, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EntityMerge, GraphOperation,
    GraphTransaction, NodeDraft, Runtime, SeedDocument, TransactionState, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
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

fn open(
    path: &std::path::Path,
    file: bool,
    authority: AuthorityStateV1,
) -> Result<Runtime, ekr_kernel::PersistenceError> {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

fn transaction(
    operations: Vec<GraphOperation>,
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version,
    }
}

/// Propose, validate against the head and, if validated, commit.
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

fn rejected(verdict: &ValidationCommandResult) -> Option<Vec<String>> {
    match verdict {
        ValidationCommandResult::Rejected(record) => Some(
            record
                .issues
                .iter()
                .map(|issue| issue.code.clone())
                .collect(),
        ),
        ValidationCommandResult::Validated(_) => None,
    }
}

/// After a schema change under v2, data that uses the new type validates and applies, and data
/// that violates it is refused; a schema change mixed with `MergeEntity` is refused as mixed; and
/// the store reopens with the same head and records, on both providers.
#[test]
fn data_after_a_schema_change_is_held_to_it_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let v2 = || anchor(ValidationProfileV1::schema_evolving(context().validator));
        let kernel = open(directory.path(), file, v2()).unwrap();
        let seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
        let root_id = seed.graph.root.id;
        kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();

        let (observation, summary, count) =
            (TypeId::mint(), PropertyId::mint(), PropertyId::mint());
        let mut declared = NodeType::new(observation, "Observation");
        let mut required = PropertyDefinition::new(summary, "summary", ValueType::String);
        required.required = true;
        declared.properties.insert(summary, required);
        declared.properties.insert(
            count,
            PropertyDefinition::new(count, "count", ValueType::Integer),
        );
        let schema = transaction(
            vec![GraphOperation::DefineNodeType(Box::new(declared))],
            Some(SchemaVersionId::mint()),
        );
        assert_eq!(rejected(&submit(&kernel, &schema, 20)), None);

        let node = |properties: BTreeMap<PropertyId, Vec<Value>>| NodeDraft {
            id: NodeId::mint(),
            root_id,
            type_id: observation,
            canonical_name: "an observation".into(),
            properties,
        };
        let good = node(BTreeMap::from([
            (summary, vec![Value::String("held".into())]),
            (count, vec![Value::Integer(3)]),
        ]));
        let (first, second) = (good.id, NodeId::mint());
        assert_eq!(
            rejected(&submit(
                &kernel,
                &transaction(vec![GraphOperation::CreateNode(good)], None),
                30
            )),
            None,
            "data using the new type validates"
        );
        let mut other = node(BTreeMap::from([(
            summary,
            vec![Value::String("also held".into())],
        )]));
        other.id = second;
        assert_eq!(
            rejected(&submit(
                &kernel,
                &transaction(vec![GraphOperation::CreateNode(other)], None),
                40
            )),
            None
        );

        for (at, what, draft) in [
            (50, "missing the required property", node(BTreeMap::new())),
            (
                60,
                "the wrong value kind",
                node(BTreeMap::from([(summary, vec![Value::Integer(1)])])),
            ),
        ] {
            assert!(
                rejected(&submit(
                    &kernel,
                    &transaction(vec![GraphOperation::CreateNode(draft)], None),
                    at
                ))
                .is_some(),
                "data {what} is refused"
            );
        }

        let mixed = transaction(
            vec![
                GraphOperation::DefineNodeType(Box::new(NodeType::new(TypeId::mint(), "Other"))),
                GraphOperation::MergeEntity(EntityMerge {
                    absorbed: second,
                    into: first,
                }),
            ],
            Some(SchemaVersionId::mint()),
        );
        let codes = rejected(&submit(&kernel, &mixed, 70)).expect("a mixed transaction");
        assert!(
            codes.iter().any(|code| code == "mixed-schema-transaction"),
            "{codes:?}"
        );

        let head = kernel.head().unwrap();
        let records = kernel.transactions().unwrap();
        drop(kernel);
        let reopened = open(directory.path(), file, v2()).unwrap();
        assert_eq!(reopened.head().unwrap(), head);
        assert_eq!(reopened.transactions().unwrap(), records);
    }
}
