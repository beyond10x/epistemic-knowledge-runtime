//! `Runtime::published_events`: the provider log a consumer of the runtime may read, on both
//! providers, without declaring `ekr-store` or naming an eventlog item.
//!
//! Wave p1-14, adversary pass 1 finding 1 on `story:ess-conformance-kernel`. The conformance target
//! in `ekr` reports the store's own events from this read. So it must be the whole log, and in
//! order: the kernel occurrence, the preparation and the stored objects. It must add nothing to
//! the log, so a retained retry and a reread leave it unchanged. And it must agree with the retained
//! history: every kernel occurrence it lists is one the kernel admits, and every stored object it
//! lists is content the runtime returns.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, ContentHash, GraphRootId, RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{GraphRoot, Space};
use ekr_kernel::runtime::PublishedEvent;
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Runtime, SeedDocument, ValidationProfileV1,
};
use ekr_ontology::{NodeType, OntologyDocument, SchemaVersion};
use ekr_store::GraphDocument;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name): (AgentId, &str)| {
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

fn seed() -> SeedDocument {
    let schema = SchemaVersionId::mint();
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
    }
}

fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(&path.join("store"), "test", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor())
    }
    .expect("the provider opens")
}

/// The file provider's log as its own read-only history inspector reads it, independently of the
/// store: every event, with the fields [`PublishedEvent`] carries, in log order.
fn inspected(root: &std::path::Path) -> Vec<PublishedEvent> {
    use eventlog_core::{InspectHistory, InspectionLimits, TenantId};
    let limits = InspectionLimits {
        source_bytes: 1 << 30,
        events: 1 << 20,
        envelope_bytes: 1 << 30,
    };
    let tenant = TenantId::new("test").unwrap();
    let history = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(eventlog_file::FileHistoryInspector::new(root).inspect_history(&tenant, limits))
        .expect("the file log inspects");
    let mut events: Vec<PublishedEvent> = history
        .events
        .into_iter()
        .map(|e| PublishedEvent {
            position: e.global_seq,
            stream_type: e.stream_type,
            stream_id: e.stream_id,
            version: e.version,
            event_id: e.event_id,
            name: e.name,
            schema_version: e.schema_version,
            data: e.data,
        })
        .collect();
    events.sort_by_key(|e| e.position);
    events
}

#[test]
fn a_seeded_runtime_publishes_its_whole_log_in_order_on_both_providers() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let runtime = open(directory.path(), file);
        assert_eq!(
            runtime.published_events().expect("an empty log reads"),
            Vec::<PublishedEvent>::new()
        );

        let document = seed();
        let result = runtime
            .seed(document.clone(), || Timestamp::from_millis(10))
            .expect("seeded");
        let events: Vec<PublishedEvent> = runtime.published_events().expect("the log reads");

        assert!(
            events.windows(2).all(|w| w[0].position < w[1].position),
            "file={file}: positions ascend: {events:?}"
        );
        let seeded: Vec<&PublishedEvent> = events
            .iter()
            .filter(|e| e.name == "ekr.kernel.Seeded")
            .collect();
        assert_eq!(seeded.len(), 1, "file={file}: one Seeded occurrence");
        assert_eq!(
            seeded[0].data["event_id"],
            result.event_id.to_string(),
            "file={file}: the logged Seeded is the one the seed handler returned"
        );
        let prepared = events
            .iter()
            .filter(|e| e.name == "ekr.store.PublicationPrepared")
            .count();
        assert_eq!(prepared, 1, "file={file}: one publication preparation");
        let stored: Vec<&PublishedEvent> = events
            .iter()
            .filter(|e| e.name == "ekr.store.ObjectStored")
            .collect();
        assert!(
            !stored.is_empty(),
            "file={file}: the seed stores its records"
        );
        for object in &stored {
            let hash: ContentHash = object.data["content_hash"]
                .as_str()
                .expect("content_hash is text")
                .parse()
                .expect("a content hash");
            let bytes = runtime
                .content(&hash)
                .expect("content reads")
                .unwrap_or_else(|| panic!("file={file}: logged object {hash} is not retained"));
            assert_eq!(object.data["byte_len"], bytes.len(), "file={file}: {hash}");
        }
        let names: BTreeSet<&str> = events.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            BTreeSet::from([
                "ekr.kernel.Seeded",
                "ekr.store.ObjectStored",
                "ekr.store.PublicationPrepared"
            ]),
            "file={file}: a seed publishes these and nothing else"
        );
        if file {
            assert_eq!(
                inspected(&directory.path().join("store")),
                events,
                "the read is exactly the log the provider's own inspector reads"
            );
        }

        let retry = runtime
            .seed(document, || panic!("a retained retry samples no time"))
            .expect("retained seed");
        assert_eq!(retry, result);
        assert_eq!(
            runtime.published_events().expect("the log reads"),
            events,
            "file={file}: a retained retry and a reread leave the log unchanged"
        );
        drop(runtime);
        assert_eq!(
            open(directory.path(), file)
                .published_events()
                .expect("the log reads"),
            events,
            "file={file}: a reopened runtime reads the same log"
        );
    }
}
