//! Current-format seed behavior through the real kernel and both persistent providers.
use std::collections::BTreeMap;

use ekr_core::{
    AgentId, ContentHash, GraphRootId, RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{GraphRoot, Space};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Commit, SeedDocument, ValidationProfileV1,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, GraphDocument, Initialize, ObjectStore, RevisionLog, SqliteStore};

fn input() -> (SeedDocument, BootstrapContext) {
    let schema = SchemaVersionId::mint();
    (
        SeedDocument {
            format: "ekr-seed/2".into(),
            ontology: OntologyDocument {
                version: SchemaVersion::seed(schema, Timestamp::EPOCH),
                node_types: vec![NodeType::new(TypeId::mint(), "UnusedDeclaration")],
                edge_types: Vec::new(),
            },
            graph: GraphDocument {
                root: GraphRoot {
                    id: GraphRootId::mint(),
                    space: Space::Canonical,
                    schema_version_id: schema,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::SEED,
                nodes: BTreeMap::new(),
                edges: BTreeMap::new(),
                assertions: BTreeMap::new(),
                evidence: BTreeMap::new(),
            },
            evidence_payloads: BTreeMap::new(),
        },
        BootstrapContext {
            operator: AgentId::mint(),
            validator: AgentId::mint(),
        },
    )
}

fn authority(context: BootstrapContext) -> AuthorityStateV1 {
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [
            (context.operator, "operator"),
            (context.validator, "validator"),
        ]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                Agent {
                    id,
                    name: name.into(),
                    capabilities: Default::default(),
                },
            )
        })
        .collect(),
        validation_profile: ValidationProfileV1::deterministic(context.validator),
    }
}

#[test]
fn current_graph_document_has_an_explicit_versioned_outer_envelope() {
    let (document, _) = input();
    let bytes = document.graph.to_bytes().unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["format"], "ekr.graph-document/2");
    assert_eq!(wire.as_object().unwrap().len(), 2);
    assert!(wire["graph"]["root"].is_object());
    assert_eq!(GraphDocument::from_bytes(&bytes).unwrap(), document.graph);
}

#[test]
fn seed_two_decodes_only_with_the_complete_graph_two_envelope() {
    let (document, _) = input();
    let mut wire = serde_json::to_value(&document).unwrap();
    wire["format"] = "ekr-seed/2".into();
    if wire["graph"].get("format").is_none() {
        let graph = wire["graph"].clone();
        wire["graph"] = serde_json::json!({"format":"ekr.graph-document/2", "graph":graph});
    }
    assert!(SeedDocument::from_yaml(&wire.to_string()).is_ok());
}

#[test]
fn seed_roots_bind_unused_ontology_declarations_on_both_providers() {
    let (document, context) = input();
    let mut failures = Vec::new();
    for file in [false, true] {
        let mut roots = Vec::new();
        for suffix in ["first", "changed"] {
            let directory = tempfile::tempdir().unwrap();
            let mut changed = document.clone();
            changed.ontology.node_types[0].name = suffix.into();
            let ontology = Ontology::load(changed.ontology.clone()).unwrap();
            let root = if file {
                Commit::over_with_authority(context, authority(context), |authority| {
                    FileStore::file(directory.path(), "ekr", ontology)
                        .map(|store| store.under(authority))
                })
                .unwrap()
                .seed(changed, || Timestamp::EPOCH)
                .unwrap()
            } else {
                Commit::over_with_authority(context, authority(context), |authority| {
                    SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", ontology)
                        .map(|store| store.under(authority))
                })
                .unwrap()
                .seed(changed, || Timestamp::EPOCH)
                .unwrap()
            };
            roots.push(root.result.ontology_root);
        }
        if roots[0] == roots[1] || roots.contains(&ContentHash::from_bytes([0; 32])) {
            failures.push(format!("file={file}: {roots:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "ontology content disappeared: {}",
        failures.join("; ")
    );
}

#[test]
fn identical_seed_retry_returns_the_retained_result_on_both_providers() {
    fn check<S: RevisionLog + ObjectStore + Initialize>(
        kernel: Commit<S>,
        document: SeedDocument,
    ) -> Result<(), String> {
        let first = kernel
            .seed(document.clone(), || Timestamp::EPOCH)
            .map_err(|error| error.to_string())?;
        let second = kernel
            .seed(document, || panic!("own-result retry sampled time"))
            .map_err(|error| error.to_string())?;
        if first != second {
            return Err("seed retry returned a different root".into());
        }
        Ok(())
    }
    let (document, context) = input();
    let mut failures = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let ontology = Ontology::load(document.ontology.clone()).unwrap();
        let outcome = if file {
            check(
                Commit::over_with_authority(context, authority(context), |authority| {
                    FileStore::file(directory.path(), "ekr", ontology)
                        .map(|store| store.under(authority))
                })
                .unwrap(),
                document.clone(),
            )
        } else {
            check(
                Commit::over_with_authority(context, authority(context), |authority| {
                    SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", ontology)
                        .map(|store| store.under(authority))
                })
                .unwrap(),
                document.clone(),
            )
        };
        if let Err(error) = outcome {
            failures.push(format!("file={file}: {error}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

#[test]
fn kernel_opening_facade_preserves_many_values_and_one_empty_list() {
    use ekr_core::{NodeId, PropertyId};
    use ekr_graph::{CanonicalValue, Node};
    use ekr_ontology::{Cardinality, PropertyDefinition, Value, ValueType};
    let (mut document, context) = input();
    let many = PropertyId::mint();
    let list = PropertyId::mint();
    let node_type = &mut document.ontology.node_types[0];
    let mut definition = PropertyDefinition::new(many, "many", ValueType::String);
    definition.cardinality = Cardinality::Many;
    node_type.properties.insert(many, definition);
    node_type.properties.insert(
        list,
        PropertyDefinition::new(list, "list", ValueType::List(Box::new(ValueType::String))),
    );
    let mut node = Node::new(
        NodeId::mint(),
        document.graph.root.id,
        node_type.id,
        "values",
    );
    node.properties.insert(
        many,
        vec![
            Value::String("first".into()),
            Value::String("first".into()),
            Value::String("last".into()),
        ],
    );
    node.properties.insert(list, vec![Value::List(Vec::new())]);
    document.graph.nodes.insert(node.id, node.clone());
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let ontology = Ontology::load(document.ontology.clone()).unwrap();
        let open = || {
            if file {
                ekr_kernel::Runtime::file(
                    directory.path(),
                    "ekr",
                    ontology.clone(),
                    context,
                    authority(context),
                )
            } else {
                ekr_kernel::Runtime::sqlite(
                    &directory.path().join("state.db"),
                    "ekr",
                    ontology.clone(),
                    context,
                    authority(context),
                )
            }
            .unwrap()
        };
        let kernel = open();
        let result = kernel
            .seed(document.clone(), || Timestamp::from_millis(17))
            .unwrap();
        assert_eq!(result.committed_at, Timestamp::from_millis(17));
        assert_eq!(kernel.head().unwrap(), Some(result.result));
        assert_eq!(
            kernel.snapshot().unwrap().nodes[&node.id].properties[&many],
            vec![
                CanonicalValue::String("first".into()),
                CanonicalValue::String("first".into()),
                CanonicalValue::String("last".into())
            ]
        );
        assert_eq!(
            kernel.snapshot().unwrap().nodes[&node.id].properties[&list],
            vec![CanonicalValue::List(Vec::new())]
        );
        assert_eq!(
            kernel.replay(RevisionNumber::SEED).unwrap(),
            kernel.snapshot().unwrap()
        );
        assert!(kernel.content(&result.seed_hash).unwrap().is_some());
        drop(kernel);
        assert_eq!(
            open()
                .seed(document.clone(), || panic!("reopen retry sampled time"))
                .unwrap(),
            result
        );
    }
}

#[test]
fn graph_outer_empty_values_and_unversioned_documents_are_refused() {
    let (mut document, _) = input();
    let property = ekr_core::PropertyId::mint();
    let mut node = ekr_graph::Node::new(
        ekr_core::NodeId::mint(),
        document.graph.root.id,
        document.ontology.node_types[0].id,
        "empty",
    );
    node.properties.insert(property, Vec::new());
    document.graph.nodes.insert(node.id, node);
    assert!(GraphDocument::from_bytes(&document.graph.to_bytes().unwrap()).is_err());
    let wire = serde_json::to_value(document.graph).unwrap();
    assert!(GraphDocument::from_bytes(&serde_json::to_vec(&wire["graph"]).unwrap()).is_err());
}

#[test]
fn exact_host_profile_and_complete_agent_registry_are_required() {
    let (document, context) = input();
    for fault in 0..7 {
        let mut anchor = authority(context);
        match fault {
            0 => anchor.validation_profile.checks.reverse(),
            1 => {
                anchor.validation_profile.checks.pop();
            }
            2 => anchor.validation_profile.provenance = "unknown/1".into(),
            3 => anchor.validation_profile.validator = AgentId::mint(),
            4 => {
                anchor.agents.remove(&context.operator);
            }
            5 => anchor.agents.get_mut(&context.validator).unwrap().id = AgentId::mint(),
            6 => anchor.format = "ekr.authority-state/999".into(),
            _ => unreachable!(),
        }
        let directory = tempfile::tempdir().unwrap();
        assert!(ekr_kernel::Runtime::sqlite(
            &directory.path().join("state.db"),
            "ekr",
            Ontology::load(document.ontology.clone()).unwrap(),
            context,
            anchor
        )
        .is_err());
        assert!(!directory.path().join("state.db").exists());
    }
}

#[test]
fn missing_or_substituted_native_seed_and_receipt_blobs_refuse_reopen() {
    use eventlog_core::{EventStore, StreamId, TenantId};
    let (document, context) = input();
    for file in [false, true] {
        for receipt in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let ontology = Ontology::load(document.ontology.clone()).unwrap();
            let open = || {
                if file {
                    ekr_kernel::Runtime::file(
                        directory.path(),
                        "ekr",
                        ontology.clone(),
                        context,
                        authority(context),
                    )
                } else {
                    ekr_kernel::Runtime::sqlite(
                        &directory.path().join("state.db"),
                        "ekr",
                        ontology.clone(),
                        context,
                        authority(context),
                    )
                }
                .unwrap()
            };
            let kernel = open();
            let result = kernel.seed(document.clone(), || Timestamp::EPOCH).unwrap();
            drop(kernel);
            let executor = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let tenant = TenantId::new("ekr").unwrap();
            let stream = StreamId::new(tenant.clone(), "ekr.revision", "canonical").unwrap();
            let hash = executor.block_on(async {
                let provider: Box<dyn EventStore> = if file {
                    Box::new(
                        eventlog_file::FileEventStore::open(directory.path())
                            .await
                            .unwrap(),
                    )
                } else {
                    Box::new(
                        eventlog_sqlite::SqliteEventStore::open(
                            directory.path().join("state.db").to_str().unwrap(),
                            "ekr",
                        )
                        .await
                        .unwrap(),
                    )
                };
                let original = provider.read_stream(&stream, 0, 100).await.unwrap().events;
                assert_eq!(original.len(), 1);
                assert_eq!(original[0].schema_version, 2);
                let hash: ContentHash = if receipt {
                    serde_json::from_value(original[0].data["record_hash"].clone()).unwrap()
                } else {
                    result.seed_hash
                };
                provider.delete_blob(&tenant, &hash.to_hex()).await.unwrap();
                assert_eq!(
                    provider.read_stream(&stream, 0, 100).await.unwrap().events,
                    original
                );
                hash
            });
            assert!(
                open().snapshot().is_err(),
                "missing required blob was admitted"
            );
            executor.block_on(async {
                let provider: Box<dyn EventStore> = if file {
                    Box::new(
                        eventlog_file::FileEventStore::open(directory.path())
                            .await
                            .unwrap(),
                    )
                } else {
                    Box::new(
                        eventlog_sqlite::SqliteEventStore::open(
                            directory.path().join("state.db").to_str().unwrap(),
                            "ekr",
                        )
                        .await
                        .unwrap(),
                    )
                };
                provider
                    .put_blob(&tenant, &hash.to_hex(), b"different actual bytes")
                    .await
                    .unwrap();
            });
            assert!(
                open().snapshot().is_err(),
                "wrong content under an opaque binding was admitted"
            );
        }
    }
}

#[test]
fn kernel_facade_refuses_an_entered_runtime_before_opening_provider_paths() {
    let (document, context) = input();
    let directory = tempfile::tempdir().unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let file = directory.path().join("file");
        let sqlite = directory.path().join("state.db");
        assert!(matches!(
            ekr_kernel::Runtime::file(
                &file,
                "ekr",
                Ontology::load(document.ontology.clone()).unwrap(),
                context,
                authority(context)
            ),
            Err(ekr_store::StoreError::RuntimeContext)
        ));
        assert!(matches!(
            ekr_kernel::Runtime::sqlite(
                &sqlite,
                "ekr",
                Ontology::load(document.ontology.clone()).unwrap(),
                context,
                authority(context)
            ),
            Err(ekr_store::StoreError::RuntimeContext)
        ));
        assert!(!file.exists());
        assert!(!sqlite.exists());
    });
}
