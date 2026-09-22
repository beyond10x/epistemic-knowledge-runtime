//! The commit path: AGENTS.md invariant 1's second half, which nothing carried until ADR 0007.
//!
//! `crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs` reads the same rule from the
//! other side, in a process that cannot link this crate: no `ValidatedTransaction` exists there and
//! no commit lands. This file is the positive half — a `ValidatedTransaction` exists here, and it
//! is the only thing that moves the lineage.
//!
//! Every case drives the SQLite provider through [`Commit`], which is the only way to a writer
//! after `architecture-decision-record:0007-the-commit-path-is-the-kernels`.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId,
};
use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, RevisionEvent, Space};
use ekr_kernel::{
    BootstrapContext, Commit, CommitError, GraphOperation, GraphTransaction, NodeDraft, Pipeline,
    SeedDocument, ValidatedTransaction,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    Appended, CommitAuthority, GraphDocument, RecordedValidation, RevisionLog, SqliteStore,
};
use tempfile::TempDir;

/// One declared node type, and the empty canonical state a lineage is seeded from.
struct World {
    ontology: Ontology,
    document: OntologyDocument,
    graph: CanonicalGraph,
    thing: ekr_core::TypeId,
    /// Who proposes.
    proposer: AgentId,
    /// Who validates, and never the proposer: design § 6.10, and the authorization validator's
    /// `proposer-is-validator`.
    validator: AgentId,
}

fn world() -> World {
    let (schema, thing, root_id) = (
        SchemaVersionId::mint(),
        ekr_core::TypeId::mint(),
        GraphRootId::mint(),
    );
    let document = OntologyDocument {
        version: SchemaVersion::seed(schema, Timestamp::EPOCH),
        node_types: vec![NodeType::new(thing, "Thing")],
        edge_types: Vec::new(),
    };
    let ontology = Ontology::load(document.clone()).expect("one node type coheres");
    let graph = CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: schema,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    World {
        ontology,
        document,
        graph,
        thing,
        proposer: AgentId::mint(),
        validator: AgentId::mint(),
    }
}

/// The real kernel seed path, then ordinary commits.
fn seeded(directory: &TempDir, world: &World) -> Commit<SqliteStore> {
    let path = directory.path().join("revisions.db");
    let ontology = world.ontology.clone();
    let context = BootstrapContext {
        operator: world.proposer,
        validator: world.validator,
    };
    let commit = Commit::over_with_bootstrap(context, move |validations| {
        SqliteStore::sqlite(&path, "ekr", ontology).map(|store| store.under(validations))
    })
    .expect("the SQLite provider opens");
    commit
        .seed(SeedDocument {
            format: "ekr-seed/1".to_owned(),
            ontology: world.document.clone(),
            graph: GraphDocument::of(&world.graph),
            evidence_payloads: BTreeMap::new(),
        })
        .expect("the seed is validated");
    commit
}

/// A proposal creating one node of the declared type, from `world.agent`.
fn proposal(world: &World) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: world.proposer,
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: world.graph.root.id,
            type_id: world.thing,
            canonical_name: "a-thing".to_owned(),
            properties: BTreeMap::new(),
        })],
        evidence: BTreeSet::new(),
    }
}

/// The pipeline's verdict on `proposal`, against the state `world` was seeded from.
fn validated(world: &World, proposal: &GraphTransaction) -> ValidatedTransaction {
    Pipeline::deterministic(world.validator)
        .validate(&GraphSnapshot::of(&world.graph), proposal)
        .expect("nothing is wrong with this proposal")
}

/// **The acceptance, from the side that has one.** A `ValidatedTransaction` commits, and the
/// lineage advances under it.
#[test]
fn a_validated_transaction_commits_and_the_lineage_advances() {
    let directory = TempDir::new().expect("a temporary directory");
    let world = world();
    let commit = seeded(&directory, &world);

    let head = commit
        .commit(validated(&world, &proposal(&world)))
        .expect("a transaction validated against the head commits");

    assert_eq!(head.revision, RevisionNumber::new(1));
    assert_eq!(
        commit
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head"),
        head,
        "the answer is the head the fold reaches and not one this path composed"
    );
}

/// And the same three events, appended by hand, do not.
///
/// The difference is one thing: `Commit::commit` recorded the validation with the [`Validations`]
/// the store was opened under, and a caller writing the events itself has nothing to record with.
/// This is the workspace-side reading of the review's case — which runs where a
/// `ValidatedTransaction` cannot exist at all.
///
/// [`Validations`]: ekr_kernel::Validations
#[test]
fn the_same_lineage_written_by_hand_does_not_advance_anything() {
    let directory = TempDir::new().expect("a temporary directory");
    let world = world();
    let commit = seeded(&directory, &world);
    let proposal = proposal(&world);
    let validated = validated(&world, &proposal);
    let raw = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        world.ontology.clone(),
    )
    .unwrap();

    for event in [
        RevisionEvent::TransactionProposed {
            transaction_id: proposal.id,
            proposer: proposal.proposer,
            operations_hash: ekr_core::ContentHash::of(&validated.transaction().operations),
        },
        RevisionEvent::TransactionValidated {
            transaction_id: proposal.id,
            against: validated.validated_against(),
            // The kernel's own validation hash, off a real `ValidatedTransaction`: the events are
            // not approximations of the ones `commit` writes, they are the same events.
            validation_hash: validated.validation_hash(),
        },
        RevisionEvent::RevisionCommitted {
            transaction_id: proposal.id,
            revision_id: ekr_core::RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: ekr_store::knowledge_root(&world.graph),
        },
    ] {
        assert_eq!(
            raw.append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    assert_eq!(
        commit
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::SEED,
        "the events are the kernel's own and the path that records the validation was not taken, \
         so nothing stands behind the commit and the lineage did not move"
    );
}

/// Design § 72 at the kernel: a transaction validated against a revision the lineage has moved past
/// is refused before anything is written.
#[test]
fn a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let world = world();
    let commit = seeded(&directory, &world);

    // Both are validated against the seed, before either commits.
    let first = validated(&world, &proposal(&world));
    let second = validated(&world, &proposal(&world));
    let stale = first.transaction().id;

    commit.commit(second).expect("the first commit lands");
    let refused = commit
        .commit(first)
        .expect_err("the second was validated against a revision the lineage has moved past");

    assert_eq!(
        refused,
        CommitError::Stale {
            transaction: stale,
            validated_against: RevisionNumber::SEED,
            head: RevisionNumber::new(1),
        }
    );
    assert_eq!(
        commit
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::new(1),
        "a refused commit writes nothing"
    );
}

/// Committing into a lineage that was never seeded is refused rather than started.
#[test]
fn a_commit_into_an_unseeded_lineage_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let world = world();
    let path = directory.path().join("revisions.db");
    let ontology = world.ontology.clone();
    let commit = Commit::over(move |validations| {
        SqliteStore::sqlite(&path, "ekr", ontology).map(|store| store.under(validations))
    })
    .expect("the SQLite provider opens");

    assert_eq!(
        commit
            .commit(validated(&world, &proposal(&world)))
            .expect_err("there is no revision for a commit to follow"),
        CommitError::NotSeeded
    );
}

/// What [`ekr_kernel::Validations`] stands behind, asked directly.
///
/// The three fields together, because any two of them leave the third free — and the fold asks with
/// all three, so a case that only committed would not tell an authority that compares them from one
/// that compares the transaction id alone.
#[test]
fn the_kernels_authority_stands_behind_exactly_what_it_validated() {
    let directory = TempDir::new().expect("a temporary directory");
    let world = world();
    let commit = seeded(&directory, &world);
    let proposal = proposal(&world);
    let validated = validated(&world, &proposal);
    let recorded = RecordedValidation {
        transaction_id: proposal.id,
        against: validated.validated_against(),
        validation_hash: validated.validation_hash(),
    };

    let validations = ekr_kernel::Validations::new();
    assert!(
        !validations.attests(&recorded),
        "a fresh authority has validated nothing"
    );

    commit.commit(validated).expect("it commits");

    // The authority the *store* was opened under is the one that now stands behind it, and it is
    // reached through the only path that adds to it.
    let elsewhere = RecordedValidation {
        against: RevisionNumber::new(9),
        ..recorded
    };
    assert!(
        !validations.attests(&elsewhere),
        "and a different authority never stands behind another's work"
    );
    assert_eq!(
        commit
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::new(1),
        "the one that was injected does"
    );
}
