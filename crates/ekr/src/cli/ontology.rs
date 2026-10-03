//! `ekr ontology`: the node types, edge types and properties at the head or at one committed
//! revision, by name and id, with the schema version and supporting evidence from one verified read.

use std::collections::BTreeMap;

use ekr_core::{RevisionNumber, SchemaVersionId, TypeId};
use ekr_kernel::Runtime;
use ekr_ontology::{Cardinality, PropertyDefinition};
use serde::Serialize;

use crate::exit::Failure;

#[derive(Serialize)]
pub(super) struct Ontology {
    revision: u64,
    schema_version: SchemaVersionId,
    /// The version's place in the lineage: 0 at the seed, one more per committed schema change.
    schema_version_number: u64,
    /// The version it was derived from; `null` at the seed.
    schema_version_parent: Option<SchemaVersionId>,
    node_types: Vec<NodeType>,
    edge_types: Vec<EdgeType>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supporting_evidence: Vec<ekr_views::SchemaEvidenceEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{inbox, upgrade_fixture as fixture};
    use ekr_core::{EvidenceId, Timestamp};
    use ekr_kernel::{human_review, CommitCommandResult, ValidationCommandResult};
    use ekr_sdk::document::{
        Confidence, EvidenceAddition, EvidenceSource, NodeType, TransactionBuilder,
    };

    #[test]
    fn schema_change_exposes_supporting_evidence() {
        for file in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            let open = || {
                if file {
                    Runtime::file(
                        directory.path(),
                        fixture::TENANT,
                        fixture::context(),
                        fixture::anchor(),
                    )
                } else {
                    Runtime::sqlite(
                        &directory.path().join("store.db"),
                        fixture::TENANT,
                        fixture::context(),
                        fixture::anchor(),
                    )
                }
                .unwrap()
            };
            let runtime = open();
            let seed = fixture::seed(false);
            let retained = *seed.graph.evidence.keys().next().unwrap();
            let seeded = runtime.seed(seed, || Timestamp::EPOCH).unwrap();
            let human = fixture::Human::new(seeded.seed_hash);
            let runtime = runtime
                .with_review_authority(human.binding.clone())
                .unwrap();
            let preview = runtime.preview_upgrade(&human.policy).unwrap();
            runtime
                .apply_upgrade(
                    &preview,
                    &human.policy,
                    &human_review::proof_from_document(&human.proof(&preview)).unwrap(),
                    fixture::STATEMENT,
                    || Timestamp::from_millis(1),
                )
                .unwrap();
            let mut expected = Vec::new();
            for ordinal in 0..2 {
                let mut builder = TransactionBuilder::new(fixture::context().operator)
                    .with_schema_evidence([retained])
                    .push(NodeType::new(format!("SupportedType{ordinal}")).into());
                let mut support = vec![retained.to_string()];
                if ordinal == 1 {
                    let addition = EvidenceAddition::new(
                        EvidenceSource::human("fixture"),
                        fixture::context().operator,
                        Timestamp::from_millis(2),
                        Confidence::CERTAIN,
                        b"<script>schema support</script>".to_vec(),
                    );
                    support.push(addition.evidence.id.to_string());
                    builder = builder.push(addition.into());
                }
                support.sort();
                let document = builder.build().unwrap();
                let tx = &document.transaction;
                let time = 10 + ordinal * 10;
                runtime
                    .propose(
                        document.to_yaml().unwrap().as_bytes(),
                        fixture::context().operator,
                        || Timestamp::from_millis(time),
                    )
                    .unwrap();
                assert!(matches!(
                    runtime
                        .validate(tx.id, runtime.head().unwrap().unwrap().revision, || {
                            Timestamp::from_millis(time + 1)
                        })
                        .unwrap(),
                    ValidationCommandResult::Validated(_)
                ));
                assert!(matches!(
                    runtime
                        .commit(tx.id, fixture::context().operator, || {
                            Timestamp::from_millis(time + 2)
                        })
                        .unwrap(),
                    CommitCommandResult::Committed(_)
                ));
                expected.push(serde_json::json!({"revision": runtime.head().unwrap().unwrap().revision.get(),
                    "schema_version": tx.schema_version, "transaction_id": tx.id, "evidence": support}));
            }
            drop(runtime);
            for full in [false, true] {
                let mut runtime = open().with_review_authority(human.binding.clone()).unwrap();
                runtime.set_full_replay(full);
                for revision in 0..=runtime.head().unwrap().unwrap().revision.get() {
                    let ontology =
                        serde_json::to_value(run(&runtime, Some(revision)).unwrap()).unwrap();
                    let sdk: ekr_sdk::read::Ontology =
                        serde_json::from_value(ontology.clone()).unwrap();
                    let wanted = serde_json::Value::Array(
                        expected
                            .iter()
                            .filter(|entry| entry["revision"].as_u64().unwrap() <= revision)
                            .cloned()
                            .collect(),
                    );
                    assert_eq!(
                        serde_json::to_value(&sdk.supporting_evidence).unwrap(),
                        wanted
                    );
                    let loaded =
                        ekr_views::load(&runtime, Some(RevisionNumber::new(revision))).unwrap();
                    let projection: serde_json::Value =
                        serde_json::from_slice(&ekr_views::render(&loaded).unwrap().bytes).unwrap();
                    let overview = ekr_views::Index::build(loaded)
                        .unwrap()
                        .overview(&ekr_views::OverviewRequest::new(None).unwrap())
                        .unwrap();
                    let sdk_overview: ekr_sdk::read::Overview =
                        serde_json::from_slice(&overview.bytes).unwrap();
                    assert_eq!(
                        serde_json::to_value(&sdk_overview.schema.supporting_evidence).unwrap(),
                        wanted
                    );
                    if wanted.as_array().unwrap().is_empty() {
                        assert!(ontology.get("supporting_evidence").is_none());
                        assert!(projection["schema"].get("supporting_evidence").is_none());
                    } else {
                        assert_eq!(ontology["supporting_evidence"], wanted);
                        assert_eq!(projection["schema"]["supporting_evidence"], wanted);
                    }
                    for entry in sdk.supporting_evidence {
                        for id in entry.evidence {
                            let id: EvidenceId = id.0.parse().unwrap();
                            let evidence = &runtime
                                .read(Some(RevisionNumber::new(revision)))
                                .unwrap()
                                .graph
                                .evidence[&id];
                            assert!(runtime.content(&evidence.content_hash).unwrap().is_some());
                        }
                    }
                }
                let html = String::from_utf8(inbox::render(&runtime).unwrap()).unwrap();
                assert!(html.contains("Schema evidence history"));
                assert!(html.contains("&lt;script&gt;schema support&lt;/script&gt;"));
                assert!(!html.contains("<script>schema support</script>"));
                for entry in &expected {
                    assert!(html.contains(entry["transaction_id"].as_str().unwrap()));
                }
                if full {
                    if let Some(output) = std::env::var_os("EKR_INBOX_INSPECTION_DIR") {
                        let output = std::path::Path::new(&output);
                        std::fs::create_dir_all(output).unwrap();
                        std::fs::write(
                            output.join(if file {
                                "file-schema-inbox.html"
                            } else {
                                "sqlite-schema-inbox.html"
                            }),
                            html,
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}

#[derive(Serialize)]
struct Named {
    id: TypeId,
    name: Option<String>,
}

#[derive(Serialize)]
struct NodeType {
    id: TypeId,
    name: String,
    parents: Vec<Named>,
    abstract_type: bool,
    properties: Vec<PropertyDefinition>,
}

#[derive(Serialize)]
struct EdgeType {
    id: TypeId,
    name: String,
    source_types: Vec<Named>,
    target_types: Vec<Named>,
    cardinality: Cardinality,
    properties: Vec<PropertyDefinition>,
}

/// The ontology of the requested (or newest) committed revision; a missing revision is the
/// kernel's `RevisionNotFound`, as for `ekr snapshot --at`.
pub(super) fn run(runtime: &Runtime, at: Option<u64>) -> Result<Ontology, Failure> {
    let revision = match at {
        Some(number) => RevisionNumber::new(number),
        None => {
            runtime
                .head()
                .map_err(Failure::store)?
                .ok_or(ekr_kernel::CommitError::NotSeeded)?
                .revision
        }
    };
    let read = runtime.schema_history(revision)?;
    let supporting_evidence = ekr_views::schema_evidence(&read).map_err(Failure::unread)?;
    let document = read.graph.ontology.to_document();
    let names: BTreeMap<TypeId, String> = document
        .node_types
        .iter()
        .map(|t| (t.id, t.name.clone()))
        .collect();
    let named = |ids: &std::collections::BTreeSet<TypeId>| -> Vec<Named> {
        ids.iter()
            .map(|id| Named {
                id: *id,
                name: names.get(id).cloned(),
            })
            .collect()
    };
    Ok(Ontology {
        revision: read.graph.revision.get(),
        supporting_evidence,
        schema_version: document.version.id,
        schema_version_number: document.version.number,
        schema_version_parent: document.version.parent,
        node_types: document
            .node_types
            .iter()
            .map(|t| NodeType {
                id: t.id,
                name: t.name.clone(),
                parents: named(&t.parents),
                abstract_type: t.abstract_type,
                properties: t.properties.values().cloned().collect(),
            })
            .collect(),
        edge_types: document
            .edge_types
            .iter()
            .map(|t| EdgeType {
                id: t.id,
                name: t.name.clone(),
                source_types: named(&t.source_types),
                target_types: named(&t.target_types),
                cardinality: t.cardinality,
                properties: t.properties.values().cloned().collect(),
            })
            .collect(),
    })
}
