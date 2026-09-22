//! What the fold refuses, and what it writes where it has nothing to compute.
//!
//! `providers.rs` proves a fold survives a reopen. That property holds just as well for a fold
//! that accepts any sequence of events at all, so the rules that make a lineage a chain a reader
//! can verify — rather than a sequence a writer asserts — are here, one case each.
//!
//! Every case drives the SQLite provider. The rules live in the fold, above the provider, and
//! `providers.rs` is where the two providers are held to each other.

mod fixture;
mod lineage;

use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{CanonicalGraph, RevisionEvent};
use ekr_store::{Appended, ObjectStore, RevisionLog, SqliteStore, StorageClass, StoreError};
use tempfile::TempDir;

/// A store over a fresh SQLite database, typed by `ontology`.
///
/// The ontology is the caller's rather than minted here. It was minted here first, and that made
/// `two_stores_seeded_from_different_state_have_different_heads` seed documents written against one
/// schema version into a store opened with another — which the crossing now refuses, correctly, and
/// which the case was never about. A helper that mints a schema version is a helper that decides a
/// thing its callers care about.
fn store(directory: &TempDir, ontology: &ekr_ontology::Ontology) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(lineage::Attesting)
}

#[test]
fn an_empty_log_has_no_head_and_does_not_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);

    assert_eq!(store.head().expect("an empty log folds"), None);
    assert!(
        matches!(store.fold(), Err(StoreError::NotSeeded)),
        "a fold with no seed has no state to fold onto"
    );
}

#[test]
fn a_lineage_that_does_not_begin_at_a_seed_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    assert_eq!(
        store
            .append(&lineage::proposed(TransactionId::mint()))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(store.fold(), Err(StoreError::NotSeeded)),
        "the first event of a lineage is its seed"
    );
}

#[test]
fn a_second_seed_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    lineage::seed_and_commit(&store, &graph).expect("the lineage is appendable");

    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let again = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the same bytes are already stored");
    assert_eq!(
        store
            .append(&lineage::seed_event(again.content_hash))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(store.fold(), Err(StoreError::SeedIsNotFirst)),
        "a lineage is seeded once; a second seed would silently restart it"
    );
}

#[test]
fn a_seed_naming_bytes_the_store_does_not_hold_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let absent = ContentHash::of_bytes(b"a seed nobody stored");
    assert_eq!(
        store
            .append(&lineage::seed_event(absent))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::SeedNotStored { seed_hash }) if seed_hash == absent
        ),
        "a seed address that resolves to nothing is a lineage with no beginning"
    );
}

#[test]
fn validating_a_transaction_that_was_never_proposed_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::validated(transaction, RevisionNumber::SEED))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::ProposalMissing { transaction_id }) if transaction_id == transaction
        ),
        "validation of a transaction nothing proposed has nothing to validate"
    );
}

#[test]
fn a_commit_of_a_transaction_that_was_never_validated_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::proposed(transaction))
            .expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::committed(
                transaction,
                RevisionNumber::new(1),
                ekr_store::knowledge_root(&graph),
            ))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::ValidationMissing { transaction_id }) if transaction_id == transaction
        ),
        "AGENTS.md invariant 1: only a validated transaction commits, and the fold says so"
    );
}

#[test]
fn a_transaction_that_went_stale_cannot_then_commit() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::proposed(transaction))
            .expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::validated(transaction, RevisionNumber::SEED))
            .expect("validated"),
        Appended::Written,
        "a new fact, not a retry: validated"
    );
    assert_eq!(
        store
            .append(&RevisionEvent::TransactionStale {
                transaction_id: transaction,
                validated_against: RevisionNumber::SEED,
                current: RevisionNumber::new(1),
            })
            .expect("stale"),
        Appended::Written,
        "a new fact, not a retry: stale"
    );
    assert_eq!(
        store
            .append(&lineage::committed(
                transaction,
                RevisionNumber::new(1),
                ekr_store::knowledge_root(&graph),
            ))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::ValidationMissing { transaction_id }) if transaction_id == transaction
        ),
        "design § 72: a transaction canonical state moved under is revalidated, not committed"
    );
}

#[test]
fn a_rejected_transaction_cannot_then_commit() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::proposed(transaction))
            .expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::validated(transaction, RevisionNumber::SEED))
            .expect("validated"),
        Appended::Written,
        "a new fact, not a retry: validated"
    );
    assert_eq!(
        store
            .append(&RevisionEvent::TransactionRejected {
                transaction_id: transaction,
                issues: 3,
            })
            .expect("rejected"),
        Appended::Written,
        "a new fact, not a retry: rejected"
    );
    assert_eq!(
        store
            .append(&lineage::committed(
                transaction,
                RevisionNumber::new(1),
                ekr_store::knowledge_root(&graph),
            ))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::ValidationMissing { transaction_id }) if transaction_id == transaction
        ),
        "a refused transaction is not a validated one, whatever validated it earlier"
    );
}

#[test]
fn a_revision_that_does_not_follow_its_parent_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::proposed(transaction))
            .expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::validated(transaction, RevisionNumber::SEED))
            .expect("validated"),
        Appended::Written,
        "a new fact, not a retry: validated"
    );
    assert_eq!(
        store
            .append(&lineage::committed(
                transaction,
                RevisionNumber::new(7),
                ekr_store::knowledge_root(&graph),
            ))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::RevisionOutOfOrder { expected, found })
                if expected == RevisionNumber::new(1) && found == RevisionNumber::new(7)
        ),
        "design § 34: the lineage counts one per commit, and a gap is a missing revision"
    );
}

#[test]
fn a_commit_publishing_a_knowledge_root_the_fold_does_not_reach_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    let transaction = TransactionId::mint();
    assert_eq!(
        store
            .append(&lineage::proposed(transaction))
            .expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::validated(transaction, RevisionNumber::SEED))
            .expect("validated"),
        Appended::Written,
        "a new fact, not a retry: validated"
    );
    let claimed = ContentHash::of_bytes(b"a knowledge root nothing in the log reaches");
    assert_eq!(
        store
            .append(&lineage::committed(
                transaction,
                RevisionNumber::new(1),
                claimed,
            ))
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    assert!(
        matches!(
            store.fold(),
            Err(StoreError::KnowledgeRootDisagrees { revision, published, .. })
                if revision == RevisionNumber::new(1) && published == claimed
        ),
        "roadmap § 4's P1 exit is that replay *reproduces* the root hash, not that it copies it"
    );
}

/// `task:two-of-the-five-revision-sub-roots-are-placeholders`.
///
/// Two of `Root`'s five sub-roots have no type to hash in P1 — `ekr_ontology::Ontology` has no
/// `Canonical` implementation and no `Agent` type exists — so the fold writes a placeholder. This
/// case asserts they are *the placeholder* rather than something derived, so the day either
/// becomes real is a day this case changes.
#[test]
fn two_of_the_five_sub_roots_are_the_placeholder_and_not_derived() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let graph = fixture::seed_graph(&ontology);
    lineage::seed_and_commit(&store, &graph).expect("the lineage is appendable");

    let head = store
        .head()
        .expect("the log folds")
        .expect("a seeded log has a head");

    assert_eq!(
        head.ontology_root,
        ekr_store::PLACEHOLDER_SUB_ROOT,
        "ontology_root is a placeholder until `Canonical for Ontology` exists"
    );
    assert_eq!(
        head.agent_root,
        ekr_store::PLACEHOLDER_SUB_ROOT,
        "agent_root is a placeholder until an Agent type exists"
    );
    assert_eq!(
        ekr_store::PLACEHOLDER_SUB_ROOT,
        ContentHash::from_bytes([0u8; 32]),
        "and the placeholder is all zeroes, which no canonical encoding reaches"
    );

    assert_eq!(
        head.knowledge_root,
        ekr_store::knowledge_root(&graph),
        "the other three are derived: knowledge from the folded graph state"
    );
    assert_eq!(
        head.evidence_root,
        ekr_store::evidence_root(&graph),
        "evidence from the folded evidence"
    );
    assert_ne!(
        head.knowledge_root, head.evidence_root,
        "and they are different addresses over different state"
    );
    assert_eq!(
        head.revision,
        RevisionNumber::new(1),
        "the lineage reached its first commit"
    );
    assert!(
        head.parent.is_some(),
        "and it chains to the seed root by that root's own address"
    );
}

/// `knowledge_root` is a function of the graph's state, not a constant the fold agrees with itself
/// about.
///
/// Written because the first version of this suite did not have it, and a `knowledge_root` stubbed
/// to hash an empty graph turned **no case red**: the fixture computes the root a commit publishes
/// with the same function the fold checks it against, so a stub agreed with itself, and the
/// acceptance — a reopened store folding to the same head — holds just as well for a store that
/// lost every node it was given.
#[test]
fn the_knowledge_root_is_a_function_of_the_graph_state() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut emptied = graph.clone();
    emptied.nodes = std::collections::BTreeMap::new();
    assert_ne!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&emptied),
        "a graph and the same graph with its nodes removed are not one address"
    );

    let mut renamed = graph.clone();
    let node_id = *graph.nodes.keys().next().expect("the seed has a node");
    renamed
        .nodes
        .get_mut(&node_id)
        .expect("that node")
        .canonical_name = "a-different-name".to_owned();
    assert_ne!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&renamed),
        "one changed property of one node moves the address"
    );

    let mut without_edges = graph.clone();
    without_edges.edges = std::collections::BTreeMap::new();
    assert_ne!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&without_edges),
        "edges are in it"
    );

    let mut without_assertions = graph.clone();
    without_assertions.assertions = std::collections::BTreeMap::new();
    assert_ne!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&without_assertions),
        "and assertions are in it"
    );
}

/// The four sub-roots are four, not one: design § 34 gives knowledge, evidence, ontology and agent
/// state different addresses precisely so that a retention sweep of evidence does not read as a
/// change to knowledge.
#[test]
fn evidence_is_addressed_apart_from_knowledge() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut swept = graph.clone();
    swept.evidence = std::collections::BTreeMap::new();

    assert_ne!(
        ekr_store::evidence_root(&graph),
        ekr_store::evidence_root(&swept),
        "removing the evidence moves the evidence root"
    );
    assert_eq!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&swept),
        "and leaves the knowledge root exactly where it was, which is what § 34 splits them for"
    );
}

/// Two lineages seeded from different state fold to different heads.
///
/// The acceptance is that a reopened store folds to the *same* head. That is satisfied by a store
/// that folds every lineage to one constant, so this is the other half: the head is a function of
/// what was seeded.
#[test]
fn two_stores_seeded_from_different_state_have_different_heads() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let mut other = graph.clone();
    let node_id = *graph.nodes.keys().next().expect("the seed has a node");
    other
        .nodes
        .get_mut(&node_id)
        .expect("that node")
        .canonical_name = "a-second-lineage".to_owned();

    let first_directory = TempDir::new().expect("a temporary directory");
    let first = store(&first_directory, &ontology);
    lineage::seed_and_commit(&first, &graph).expect("the first lineage is appendable");

    let second_directory = TempDir::new().expect("a temporary directory");
    let second = store(&second_directory, &ontology);
    lineage::seed_and_commit(&second, &other).expect("the second lineage is appendable");

    assert_ne!(
        first.head().expect("folds").expect("has a head"),
        second.head().expect("folds").expect("has a head"),
        "a head root is a function of the state it was seeded from"
    );
}

/// Seeds a fresh lineage from `graph` and answers the store holding it.
///
/// The three cases below all need a seeded lineage and then a *hand-written* sequence after it,
/// which `lineage::seed_and_commit` cannot give them: it writes the commit too.
fn seeded(
    directory: &TempDir,
    ontology: &ekr_ontology::Ontology,
    graph: &CanonicalGraph,
) -> SqliteStore {
    let store = store(directory, ontology);
    let document = ekr_store::GraphDocument::of(graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );
    store
}

/// AGENTS.md invariant 1, at the fold: a validation nothing stands behind does not commit.
///
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels`. Every event here is
/// well-formed and every one is written — the append is not the check — and the transaction is
/// proposed before it is validated and validated before it is committed, so every earlier rule in
/// this file is satisfied. The one thing wrong with it is the thing that used not to be checked at
/// all: the validation names an address [`lineage::Attesting`] does not stand behind, so the
/// authority declines and the lineage does not move.
///
/// **And the fold reports no error.** That is the rule and not an omission: a log is append-only
/// and its writer is not this crate, so a commit nobody stands behind is a claim the lineage holds
/// and did not act on — where an error would let one append brick every read of it forever.
/// `a_commit_of_a_transaction_that_was_never_validated_is_refused` above is the other half: a
/// lineage that is *malformed* is still a `StoreError`.
#[test]
fn a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);

    let transaction = TransactionId::mint();
    for event in [
        lineage::proposed(transaction),
        RevisionEvent::TransactionValidated {
            transaction_id: transaction,
            against: RevisionNumber::SEED,
            validation_hash: ContentHash::of_bytes(
                b"a validation this suite's authority never did",
            ),
        },
        lineage::committed(
            transaction,
            RevisionNumber::new(1),
            ekr_store::knowledge_root(&graph),
        ),
    ] {
        assert_eq!(
            store
                .append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    assert_eq!(
        store
            .head()
            .expect("the log folds rather than failing")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::SEED,
        "the commit named a validation the authority does not stand behind, so the lineage stayed \
         where it was"
    );
}

/// The same lineage with the validation the authority *does* stand behind, which advances.
///
/// The case above is one field away from this one, and without this one it would pass for a fold
/// that refused every commit — including the one it is supposed to accept.
#[test]
fn a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);

    let transaction = TransactionId::mint();
    for event in [
        lineage::proposed(transaction),
        lineage::validated(transaction, RevisionNumber::SEED),
        lineage::committed(
            transaction,
            RevisionNumber::new(1),
            ekr_store::knowledge_root(&graph),
        ),
    ] {
        assert_eq!(
            store
                .append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    assert_eq!(
        store
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::new(1),
        "the authority stands behind this validation, so the commit moved the lineage"
    );
}

/// Design § 72: a transaction is committed only against the revision it was validated against.
///
/// The fold was handed that revision in `TransactionValidated.against` and discarded it, so a
/// commit the design calls **stale** replayed as valid — which is the worse half, because a replay
/// is what `docs/roadmap.md` § 4's exit criterion rests on: a lineage that reproduces is a lineage
/// a reader has checked, and one that reproduces a stale commit has checked nothing about it.
///
/// Both transactions are validated against the seed and both validations are ones the authority
/// stands behind, so the only thing separating them is the revision the lineage had reached when
/// each commit arrived.
#[test]
fn a_commit_validated_against_a_revision_the_lineage_has_moved_past_does_not_advance_it() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);

    let (first, second) = (TransactionId::mint(), TransactionId::mint());
    let root = ekr_store::knowledge_root(&graph);
    for event in [
        lineage::proposed(first),
        lineage::proposed(second),
        lineage::validated(first, RevisionNumber::SEED),
        lineage::validated(second, RevisionNumber::SEED),
        // The second commits first, so revision 1 lands under it.
        lineage::committed(second, RevisionNumber::new(1), root),
        // And the first was validated against revision 0, which the lineage has moved past.
        lineage::committed(first, RevisionNumber::new(2), root),
    ] {
        assert_eq!(
            store
                .append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    assert_eq!(
        store
            .head()
            .expect("the log folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::new(1),
        "transaction {first} was validated against revision 0 and revision 1 landed under it; the \
         fold reads the `against` it is handed and leaves the lineage where the commit it did \
         honour put it"
    );
}

/// A store nobody gave an authority to says so, rather than folding every commit away in silence.
///
/// The two refusals in this file's other cases are about a *lineage*: a commit with no validation
/// behind it, a revision out of order, a published root the fold does not reach. This one is about
/// the **caller**, and the difference is who can act on it. `SqliteStore::sqlite` is public and
/// `under` is opt-in, so before this the default construction of a public store folded every commit
/// away and reported nothing at all — a head that did not move being every signal a consumer got,
/// while the hand-written bad lineage beside it returned an error.
///
/// `head` deliberately still answers: how far the lineage verifiably got has a total answer, and
/// `review_p1_invariant_one_at_the_store.rs` asks exactly that question from a process that has no
/// `ekr-kernel` in it.
#[test]
fn a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    // Deliberately not this file's `store`, which is opened under `lineage::Attesting`.
    let store = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens");
    lineage::seed_and_commit(&store, &graph).expect("the lineage is appendable");

    assert_eq!(store.fold(), Err(StoreError::NoSeedAuthority));
    assert_eq!(
        store.replay(RevisionNumber::SEED),
        Err(StoreError::NoSeedAuthority)
    );
    assert_eq!(store.head(), Err(StoreError::NoSeedAuthority));

    // The same lineage under an authority that stands behind it folds.
    let authorised = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider reopens")
    .under(lineage::Attesting);
    assert_eq!(
        authorised
            .fold()
            .expect("the same bytes, with somebody to ask")
            .revision,
        RevisionNumber::new(1),
        "the refusal is about the store's construction and not about the lineage"
    );
}
