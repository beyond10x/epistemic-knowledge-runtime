//! Adversary case for `story:schema-transaction-cites-evidence`, views side.
//!
//! The story's Build says "the schema reads (`ekr ontology --at`, the projection's schema
//! lineage) show it", and its second acceptance bullet says "`ekr ontology --at <that revision>`
//! and the lineage name the evidence". This drives the projection (`ekr.graph-projection/1`,
//! `schema.versions`) of a store whose version 1 cites one evidence entry, on both providers.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EvidenceId, SchemaVersionId, Timestamp, TransactionId, TypeId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EvidenceAddition,
    GraphOperation, GraphTransaction, Runtime, SeedDocument, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::NodeType;
use serde::Serialize;
use serde_json::Value;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn v2() -> AuthorityStateV1 {
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
        validation_profile: ValidationProfileV1::schema_evolving(c.validator),
    }
}

fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes();
    kernel
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let committed = kernel
        .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(committed, CommitCommandResult::Committed(_)),
        "{committed:?}"
    );
}

#[test]
fn the_projections_schema_lineage_names_the_evidence_a_version_cites() {
    for file in [false, true] {
        let at = if file { "file" } else { "sqlite" };
        let directory = tempfile::tempdir().unwrap();
        let kernel = if file {
            Runtime::file(directory.path(), "test", context(), v2())
        } else {
            Runtime::sqlite(&directory.path().join("state.db"), "test", context(), v2())
        }
        .unwrap();
        let seed = SeedDocument::from_yaml(include_str!(
            "../../ekr-kernel/tests/fixtures/seed-minimal-v2.yaml"
        ))
        .unwrap();
        kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();

        let text = "Topics are their own type.";
        let evidence = Evidence {
            id: EvidenceId::mint(),
            source: EvidenceSource::HumanStatement {
                identity: Some("a consumer's message".into()),
            },
            content_hash: ContentHash::of_bytes(text.as_bytes()),
            extracted_by: context().operator,
            observed_at: Timestamp::from_millis(1_773_273_600_000),
            confidence: Confidence::CERTAIN,
        };
        let cited = evidence.id;
        let topic = TypeId::mint();
        submit(
            &kernel,
            &GraphTransaction {
                id: TransactionId::mint(),
                proposer: context().operator,
                operations: vec![
                    GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                        evidence,
                        payload: text.as_bytes().to_vec(),
                    })),
                    GraphOperation::DefineNodeType(Box::new(NodeType::new(topic, "Topic"))),
                    GraphOperation::CiteEvidence(cited),
                ],
                evidence: [cited].into_iter().collect(),
                schema_version: Some(SchemaVersionId::mint()),
            },
            20,
        );
        assert_eq!(
            kernel.snapshot().unwrap().ontology.version().evidence,
            BTreeSet::from([cited]),
            "{at}: the kernel records the citation"
        );

        let rendered = ekr_views::project(&kernel, None).unwrap();
        let document: Value = serde_json::from_slice(&rendered.bytes).unwrap();
        let versions = document["schema"]["versions"].as_array().unwrap();
        let version = versions
            .iter()
            .find(|version| version["number"] == 1)
            .unwrap_or_else(|| panic!("{at}: no version 1 in {versions:?}"));
        assert!(
            version.to_string().contains(&cited.to_string()),
            "{at}: the lineage entry of the citing version does not name the evidence {cited}: \
             {version}"
        );
    }
}
