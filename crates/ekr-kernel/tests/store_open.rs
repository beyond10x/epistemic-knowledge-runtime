//! `story:store-open-semantics` at the kernel facade, on both providers: the existing-only
//! constructors create nothing where no store is, and the two checks a host runs before it creates
//! a store for a seed — `Runtime::check_anchor` and `Runtime::admit_seed` — refuse exactly what
//! opening a provider and seeding it would refuse.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{AgentId, GraphRootId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{GraphRoot, Space};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Runtime, SeedDocument, SeedError,
    ValidationProfileV1,
};
use ekr_ontology::{NodeType, OntologyDocument, SchemaVersion};
use ekr_store::GraphDocument;

const PROVIDERS: [bool; 2] = [true, false];

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

fn store(directory: &Path, file: bool) -> std::path::PathBuf {
    directory.join(if file { "store" } else { "state.db" })
}

fn open(path: &Path, file: bool, anchor: AuthorityStateV1) -> Result<Runtime, String> {
    if file {
        Runtime::file(path, "test", context(), anchor)
    } else {
        Runtime::sqlite(path, "test", context(), anchor)
    }
    .map_err(|error| error.to_string())
}

fn open_existing(path: &Path, file: bool) -> Result<Runtime, String> {
    if file {
        Runtime::file_existing(path, "test", context(), anchor())
    } else {
        Runtime::sqlite_existing(path, "test", context(), anchor())
    }
    .map_err(|error| error.to_string())
}

#[test]
fn the_existing_only_constructors_create_nothing_and_open_what_a_seed_created() {
    for file in PROVIDERS {
        let directory = tempfile::tempdir().unwrap();
        for missing in [
            store(directory.path(), file),
            directory.path().join("a/b/c"),
        ] {
            assert!(open_existing(&missing, file).is_err(), "file={file}");
        }
        assert_eq!(
            std::fs::read_dir(directory.path()).unwrap().count(),
            0,
            "file={file}: an existing-only open created something"
        );
        let path = store(directory.path(), file);
        let seeded = open(&path, file, anchor())
            .unwrap()
            .seed(seed(), || Timestamp::from_millis(1))
            .unwrap();
        let reopened = open_existing(&path, file).expect("the seeded store opens");
        assert_eq!(reopened.head().unwrap(), Some(seeded.result), "file={file}");
    }
}

#[test]
fn admit_seed_refuses_exactly_what_seeding_a_new_store_refuses() {
    let mut lineage = seed();
    lineage.graph.revision = RevisionNumber::new(1);
    let mut ontology_lineage = seed();
    ontology_lineage.ontology.version.number = 1;
    for file in PROVIDERS {
        assert_eq!(Runtime::admit_seed(&seed(), context()), Ok(()));
        for refused in [&lineage, &ontology_lineage] {
            let admitted = Runtime::admit_seed(refused, context());
            assert!(
                matches!(admitted, Err(SeedError::Invalid(_))),
                "{admitted:?}"
            );
            let directory = tempfile::tempdir().unwrap();
            let seeded = open(&store(directory.path(), file), file, anchor())
                .unwrap()
                .seed(refused.clone(), || Timestamp::from_millis(1))
                .map(|_| ());
            assert_eq!(seeded, admitted, "file={file}");
        }
    }
}

#[test]
fn check_anchor_refuses_exactly_what_every_constructor_refuses_first() {
    assert_eq!(Runtime::check_anchor(context(), &anchor()), Ok(()));
    let mut foreign = anchor();
    foreign.validation_profile = ValidationProfileV1::deterministic(AgentId::mint());
    let refused = Runtime::check_anchor(context(), &foreign).map_err(|error| error.to_string());
    assert!(
        refused.is_err(),
        "a profile for another validator is refused"
    );
    for file in PROVIDERS {
        let directory = tempfile::tempdir().unwrap();
        let path = store(directory.path(), file);
        let opened = open(&path, file, foreign.clone()).map(|_| ());
        assert_eq!(opened, refused, "file={file}");
        assert!(
            !path.exists(),
            "file={file}: the refused open created the store"
        );
    }
}
