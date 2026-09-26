//! Adversary pass 2, wave p1-14 unit p1-14-exit.
//!
//! Three questions the correction `8a22a2e` leaves to its own suite, asked from outside it:
//!
//! * `p1_exit_properties.rs::physical_events` is the only thing that reads "nothing is published"
//!   from the provider rather than from the runtime. It compares a count before the refused commit
//!   with one after, so a counter that reads the wrong namespace, tenant or directory answers 0 = 0
//!   and the assertion holds whatever the kernel does. What this asserts: the same counter, copied
//!   verbatim, reads a non-zero count after the seed on both providers and a larger one after a
//!   commit — so the comparison it is used in can fail. And a runtime freshly opened over the
//!   directory after a refused commit reads the seed's head and revision count, on both providers.
//! * The three assertion references this correction typed (`Assessment::Disputed`,
//!   `AssertionLifecycle::Superseded`, `EvidenceSource::GraphAssertion`) are held by no vector in
//!   their many-member or non-default shape. What this asserts: each encodes, serialises and
//!   decodes exactly as the bare-id form it replaced — the transient instantiation
//!   `Assessment<AssertionId>` / `AssertionLifecycle<AssertionId>` for the two generic ones, and
//!   the frozen `ekr_graph::legacy::EvidenceSource` for the third.
//! * `a_transient_identity_is_minted_into_every_canonical_reference_kind` compiles, which is its
//!   evidence: the public mint `CanonicalRef::new` takes a candidate's id for every kind.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::Canonical;
use ekr_core::{
    AssertionId, EdgeId, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, Edge, Evidence, EvidenceSource,
    LocalRef, Node, Subject,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, GraphOperation,
    GraphTransaction, NodeDraft, Runtime, SeedDocument, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::NodeType;
use serde::Serialize;

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

fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor())
    }
    .unwrap()
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

/// Copied verbatim from `tests/p1_exit_properties.rs::physical_events`.
fn physical_events(path: &std::path::Path, file: bool) -> usize {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        let provider: Box<dyn eventlog_core::EventStore> = if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        };
        let tenant = eventlog_core::TenantId::new("test").unwrap();
        let (mut count, mut after) = (0, 0);
        loop {
            let page = provider.read_feed(&tenant, after, 100).await.unwrap();
            count += page.events.len();
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        count
    })
}

fn seed() -> (SeedDocument, TypeId) {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let node_type = TypeId::mint();
    seed.ontology
        .node_types
        .push(NodeType::new(node_type, "Subject"));
    (seed, node_type)
}

fn create(
    seed: &SeedDocument,
    node_type: TypeId,
    root_id: ekr_core::GraphRootId,
) -> GraphTransaction {
    let _ = seed;
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id,
            type_id: node_type,
            canonical_name: "created".into(),
            properties: BTreeMap::new(),
        })],
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}

#[test]
fn the_physical_event_counter_the_exit_property_relies_on_can_see_a_publication() {
    for file in [false, true] {
        let (seed, node_type) = seed();
        let root = seed.graph.root.id;
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let seeded = physical_events(directory.path(), file);
        assert!(seeded > 0, "file={file}: the counter reads no seed event");

        let tx = create(&seed, node_type, root);
        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        let verdict = kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
            .unwrap();
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "file={file}: {verdict:?}"
        );
        let before = physical_events(directory.path(), file);
        let committed = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(40))
            .unwrap();
        assert!(matches!(committed, CommitCommandResult::Committed(_)));
        let after = physical_events(directory.path(), file);
        assert!(
            after > before,
            "file={file}: a commit appended nothing the counter sees ({before} -> {after})"
        );
    }
}

#[test]
fn a_refused_commit_publishes_nothing_a_fresh_runtime_reads_on_either_provider() {
    for file in [false, true] {
        let (seed, node_type) = seed();
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let head = kernel.head().unwrap();
        let revisions = kernel.read(None).unwrap().revisions.len();
        // A graph root nothing holds: refused by the reference validator.
        let tx = create(&seed, node_type, ekr_core::GraphRootId::mint());
        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        let verdict = kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
            .unwrap();
        assert!(
            matches!(verdict, ValidationCommandResult::Rejected(_)),
            "file={file}: {verdict:?}"
        );
        assert!(kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(40))
            .is_err());
        drop(kernel);
        let fresh = open(directory.path(), file);
        assert_eq!(fresh.head().unwrap(), head, "file={file}");
        assert_eq!(
            fresh.read(None).unwrap().revisions.len(),
            revisions,
            "file={file}"
        );
    }
}

fn ids(size: usize) -> Vec<AssertionId> {
    (0..size).map(|_| AssertionId::mint()).collect()
}

#[test]
fn a_typed_dispute_keeps_the_bare_id_bytes_and_wire_shape() {
    for size in [0usize, 1, 2, 5, 33] {
        let bare: Assessment<AssertionId> = Assessment::Disputed {
            competing_assertions: ids(size),
        };
        let typed: Assessment = bare.clone().map_assertions(CanonicalRef::new);
        assert_eq!(
            typed.canonical_bytes(),
            bare.canonical_bytes(),
            "size {size}"
        );
        let json = serde_json::to_string(&bare).unwrap();
        assert_eq!(serde_json::to_string(&typed).unwrap(), json, "size {size}");
        assert_eq!(serde_json::from_str::<Assessment>(&json).unwrap(), typed);
    }
}

#[test]
fn a_typed_supersession_keeps_the_bare_id_bytes_and_wire_shape() {
    for _ in 0..64 {
        let bare: AssertionLifecycle<AssertionId> = AssertionLifecycle::Superseded {
            by: AssertionId::mint(),
            at_revision: RevisionNumber::new(7),
            effective_from: Timestamp::from_millis(1_500),
        };
        let typed: AssertionLifecycle = bare.clone().map_assertions(CanonicalRef::new);
        assert_eq!(typed.canonical_bytes(), bare.canonical_bytes());
        let json = serde_json::to_string(&bare).unwrap();
        assert_eq!(serde_json::to_string(&typed).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<AssertionLifecycle>(&json).unwrap(),
            typed
        );
    }
}

#[test]
fn a_typed_graph_assertion_source_keeps_the_frozen_bytes_and_wire_shape() {
    for _ in 0..64 {
        let assertion = AssertionId::mint();
        let frozen = ekr_graph::legacy::EvidenceSource::GraphAssertion(assertion);
        let typed = EvidenceSource::GraphAssertion(CanonicalRef::new(assertion));
        assert_eq!(
            typed.canonical_bytes(),
            frozen.canonical_bytes(),
            "{assertion}"
        );
        let json = serde_json::to_string(&frozen).unwrap();
        assert_eq!(serde_json::to_string(&typed).unwrap(), json, "{assertion}");
        assert_eq!(
            serde_json::from_str::<EvidenceSource>(&json).unwrap(),
            typed,
            "{assertion}"
        );
    }
}

#[test]
fn a_transient_identity_is_minted_into_every_canonical_reference_kind() {
    let node = LocalRef::<Node>::new(NodeId::mint());
    let edge = LocalRef::<Edge>::new(EdgeId::mint());
    let assertion = LocalRef::<Assertion>::new(AssertionId::mint());
    let evidence = LocalRef::<Evidence>::new(EvidenceId::mint());
    let subject: Subject = Subject::Node(CanonicalRef::new(node.id()));
    let about_edge: Subject = Subject::Edge(CanonicalRef::new(edge.id()));
    let lifecycle: AssertionLifecycle = AssertionLifecycle::Superseded {
        by: CanonicalRef::new(assertion.id()),
        at_revision: RevisionNumber::SEED,
        effective_from: Timestamp::EPOCH,
    };
    let cited: BTreeSet<CanonicalRef<Evidence>> =
        BTreeSet::from([CanonicalRef::new(evidence.id())]);
    assert!(matches!(subject, Subject::Node(r) if r.id() == node.id()));
    assert!(matches!(about_edge, Subject::Edge(r) if r.id() == edge.id()));
    assert!(
        matches!(lifecycle, AssertionLifecycle::Superseded { by, .. } if by.id() == assertion.id())
    );
    assert!(cited.iter().all(|r| r.id() == evidence.id()));
}
