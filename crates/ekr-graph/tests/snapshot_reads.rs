//! Design § 65, built as canonical state, read through a snapshot.
//!
//! > Canonical state: `Alice CEO_OF Acme`. New evidence shows: `Bob became CEO on 2026-03-12`.
//! > The system does not delete Alice's historical relationship. Instead:
//! > `Alice CEO_OF Acme, valid_to = 2026-03-12`; `Bob CEO_OF Acme, valid_from = 2026-03-12`.
//! > **The current-world query returns Bob. Historical query remains possible.**
//!
//! The acceptance of `story:graph-model-and-assertions`, restated after adversary pass 1:
//!
//! > A `GraphSnapshot` over the § 65 fixture answers `valid_at(t)` with Bob for every `t` at or
//! > after the handover, with Alice for every `t` before it, and the handover instant itself
//! > belongs to exactly one of them.
//!
//! # One read, both roles
//!
//! There is no clock. `architecture-decision-record:0004-timestamp-in-ekr-core` is explicit —
//! "Nothing in `ekr-core` reads the system time; a `Timestamp` arrives from its caller" — and P1
//! declares no time crate to read one with, so the runtime cannot supply "now" and cannot answer
//! the current world by itself. The caller can: `valid_at(now)` is design § 65's current-world
//! query and `valid_at(anything_else)` is its historical one, out of one function.
//!
//! The first version of this file asked `active()` instead, which filtered valid time with no
//! *known* end. Adversary pass 1 measured that against `valid_at(i64::MAX)` over 56 records of
//! every shape and got the identical answer, so it was the world at the end of representable time
//! rather than the present — and it put a fixed-term fact that is true today on the wrong side of
//! the line. `active()` is gone, and these cases pin the acceptance in **both** directions rather
//! than asserting only what the surviving read happens to return.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, EvidenceId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId,
    Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionStatus, CanonicalGraph, CanonicalRef, GraphRoot, GraphSnapshot, Node,
    Object, Predicate, RetractionReason, Space, Subject, TemporalRange, TransactionTime,
    ValidationState,
};
use ekr_ontology::{EdgeType, NodeType, Ontology, OntologyDocument, SchemaVersion};

/// 2024-01-01T00:00:00Z — when Alice's tenure began, from the § 14.2 example.
const TENURE_BEGAN: Timestamp = Timestamp::from_millis(1_704_067_200_000);
/// 2025-06-01T00:00:00Z — a date inside Alice's tenure, which the historical read asks about.
const DURING_ALICE: Timestamp = Timestamp::from_millis(1_748_736_000_000);
/// 2026-03-12T00:00:00Z — the day § 65 hands the chair over.
const HANDOVER: Timestamp = Timestamp::from_millis(1_773_273_600_000);
/// 2027-01-01T00:00:00Z — an instant well inside Bob's tenure, standing in for a caller's "now".
const AFTER_HANDOVER: Timestamp = Timestamp::from_millis(1_798_761_600_000);

/// The § 65 world, as canonical state.
///
/// Built here rather than behind a constructor on `CanonicalGraph`: a fixture is a statement about
/// one example, and a type that knows about Alice is a type that has a domain concept in it.
struct Fixture {
    graph: CanonicalGraph,
    alice: AssertionId,
    bob: AssertionId,
    retracted: AssertionId,
    superseded: AssertionId,
}

fn fixture() -> Fixture {
    let root_id = GraphRootId::mint();
    let (person, organisation, ceo_of) = (TypeId::mint(), TypeId::mint(), TypeId::mint());
    let schema_version = SchemaVersionId::mint();

    let mut ceo_of_type = EdgeType::new(ceo_of, "CEO_OF");
    ceo_of_type.source_types = [person].into_iter().collect();
    ceo_of_type.target_types = [organisation].into_iter().collect();

    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema_version, TENURE_BEGAN),
        node_types: vec![
            NodeType::new(person, "Person"),
            NodeType::new(organisation, "Organisation"),
        ],
        edge_types: vec![ceo_of_type],
    })
    .expect("the fixture ontology coheres");

    let (alice_node, bob_node, acme) = (NodeId::mint(), NodeId::mint(), NodeId::mint());
    let nodes: BTreeMap<NodeId, Node> = [
        (alice_node, Node::new(alice_node, root_id, person, "Alice")),
        (bob_node, Node::new(bob_node, root_id, person, "Bob")),
        (acme, Node::new(acme, root_id, organisation, "Acme")),
    ]
    .into_iter()
    .collect();

    let proposer = AgentId::mint();
    let accepted = || ValidationState::Accepted {
        validators: [proposer].into_iter().collect(),
    };
    let ceo_of_acme = |subject: NodeId, valid: TemporalRange, validation: ValidationState| {
        let id = AssertionId::mint();
        (
            id,
            Assertion {
                id,
                root_id,
                subject: Subject::Node(CanonicalRef::new(subject)),
                predicate: Predicate::Relation(ceo_of),
                object: Object::Node(CanonicalRef::new(acme)),
                evidence: BTreeSet::from([EvidenceId::mint()]),
                proposed_by: proposer,
                validation,
                valid_time: valid,
                transaction_time: TransactionTime::since(TENURE_BEGAN),
            },
        )
    };

    // Alice held the chair from 2024-01-01 until the handover: a closed valid time.
    let (alice, alice_assertion) = ceo_of_acme(
        alice_node,
        TemporalRange::new(Some(TENURE_BEGAN), Some(HANDOVER))
            .expect("Alice's tenure is not inverted"),
        accepted(),
    );
    // Bob holds it from the handover, with no end: an open valid time.
    let (bob, bob_assertion) = ceo_of_acme(
        bob_node,
        TemporalRange::new(Some(HANDOVER), None).expect("Bob's tenure is not inverted"),
        accepted(),
    );
    // Two records that would answer both reads if their status did not say otherwise. Design § 36:
    // "Current queries can expose only active assertions by default."
    let (retracted, retracted_assertion) = ceo_of_acme(
        bob_node,
        TemporalRange::UNBOUNDED,
        ValidationState::Retracted {
            at_revision: RevisionNumber::new(4),
            reason: RetractionReason::new("the evidence was another Acme"),
        },
    );
    let (superseded, superseded_assertion) = ceo_of_acme(
        alice_node,
        TemporalRange::UNBOUNDED,
        ValidationState::Superseded { by: bob },
    );

    let assertions: BTreeMap<AssertionId, Assertion> = [
        (alice, alice_assertion),
        (bob, bob_assertion),
        (retracted, retracted_assertion),
        (superseded, superseded_assertion),
    ]
    .into_iter()
    .collect();

    Fixture {
        graph: CanonicalGraph {
            root: GraphRoot {
                id: root_id,
                space: Space::Canonical,
                schema_version_id: schema_version,
                parent: None,
                created_at: TENURE_BEGAN,
            },
            revision: RevisionNumber::new(7),
            ontology,
            nodes,
            edges: BTreeMap::new(),
            assertions,
            evidence: BTreeMap::new(),
        },
        alice,
        bob,
        retracted,
        superseded,
    }
}

/// The name of the node an assertion's subject points at, which is what § 65 reads out loud.
fn subject_name<'a>(graph: &'a CanonicalGraph, assertion: &Assertion) -> &'a str {
    let Subject::Node(node) = assertion.subject else {
        panic!("the fixture's subjects are nodes");
    };
    graph.nodes[&node.node()].canonical_name.as_str()
}

/// **The acceptance.** `valid_at(t)` answers Alice before the handover and Bob at or after it.
///
/// Both directions, at every instant that matters. A filter that answered "Bob always", or
/// "whichever record has no end date", fails here rather than passing on the half of the example
/// that happens to hold.
#[test]
fn valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it() {
    let fixture = fixture();
    let snapshot = GraphSnapshot::of(&fixture.graph);

    for at in [
        TENURE_BEGAN,
        DURING_ALICE,
        Timestamp::from_millis(HANDOVER.millis() - 1),
    ] {
        let answered = snapshot.valid_at(at);
        assert_eq!(answered.len(), 1, "one person held the chair at {at}");
        assert_eq!(answered[0].id, fixture.alice, "at {at}");
        assert_eq!(subject_name(&fixture.graph, answered[0]), "Alice");
    }

    for at in [
        HANDOVER,
        Timestamp::from_millis(HANDOVER.millis() + 1),
        AFTER_HANDOVER,
        Timestamp::from_millis(i64::MAX),
    ] {
        let answered = snapshot.valid_at(at);
        assert_eq!(answered.len(), 1, "one person held the chair at {at}");
        assert_eq!(answered[0].id, fixture.bob, "at {at}");
        assert_eq!(subject_name(&fixture.graph, answered[0]), "Bob");
    }

    // Before either tenure began, nobody held it — `None`, not a guess and not whichever record
    // has no end date.
    assert!(snapshot
        .valid_at(Timestamp::from_millis(TENURE_BEGAN.millis() - 1))
        .is_empty());
}

/// The historical query remains possible: on 2025-06-01 the chair was Alice's.
///
/// § 65's second sentence, on its own. The acceptance case above covers this instant among
/// others; this one keeps the design's own claim answerable by name, so that a future change that
/// narrowed the acceptance case could not take the historical read down with it unnoticed.
#[test]
fn the_historical_query_returns_alice() {
    let fixture = fixture();
    let snapshot = GraphSnapshot::of(&fixture.graph);

    let held = snapshot.valid_at(DURING_ALICE);
    assert_eq!(held.len(), 1, "one person held the chair on 2025-06-01");
    assert_eq!(held[0].id, fixture.alice);
    assert_eq!(subject_name(&fixture.graph, held[0]), "Alice");
}

/// The handover instant belongs to Bob, not to both of them.
///
/// Valid time is half-open — `[from, to)` — so the two tenures partition the timeline instead of
/// overlapping on the one millisecond § 65 names. Without that, the example's own date returns two
/// chief executives.
#[test]
fn the_handover_instant_belongs_to_exactly_one_of_them() {
    let fixture = fixture();
    let snapshot = GraphSnapshot::of(&fixture.graph);

    let at_handover = snapshot.valid_at(HANDOVER);
    assert_eq!(
        at_handover.len(),
        1,
        "the two tenures overlap at the handover"
    );
    assert_eq!(at_handover[0].id, fixture.bob);

    // One millisecond earlier it is still Alice's.
    let just_before = snapshot.valid_at(Timestamp::from_millis(HANDOVER.millis() - 1));
    assert_eq!(just_before.len(), 1);
    assert_eq!(just_before[0].id, fixture.alice);
    assert_ne!(
        at_handover[0].id, just_before[0].id,
        "the two tenures meet at the handover and must not both contain it"
    );
}

/// `valid_at` never returns a retracted or a superseded assertion, at any `t`.
///
/// Both excluded records are built to pass every *other* filter this read applies: their valid
/// time is unbounded, so it contains every instant, and their transaction time is open. Their
/// validation state is the only thing keeping them out — which is what this case is for, and is as
/// much as it proves. It does not exercise the valid-time bounds or the transaction-time bound;
/// `valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it` and
/// `a_record_whose_transaction_time_is_closed_is_not_current` do that.
#[test]
fn valid_at_never_returns_a_retracted_or_superseded_assertion() {
    let fixture = fixture();
    let snapshot = GraphSnapshot::of(&fixture.graph);

    assert!(matches!(
        fixture.graph.assertions[&fixture.retracted].status(),
        AssertionStatus::Retracted { .. }
    ));
    assert!(matches!(
        fixture.graph.assertions[&fixture.superseded].status(),
        AssertionStatus::Superseded { .. }
    ));
    assert!(
        fixture.graph.assertions[&fixture.retracted]
            .valid_time
            .contains(DURING_ALICE),
        "the case is only worth anything if the excluded records would otherwise answer"
    );
    assert!(
        fixture.graph.assertions[&fixture.superseded]
            .transaction_time
            .is_open(),
        "and if the runtime still holds the record, so the other two filters pass"
    );

    let excluded = [fixture.retracted, fixture.superseded];
    for at in [
        DURING_ALICE,
        HANDOVER,
        TENURE_BEGAN,
        Timestamp::EPOCH,
        Timestamp::from_millis(i64::MAX),
    ] {
        for answered in snapshot.valid_at(at) {
            assert!(
                !excluded.contains(&answered.id),
                "valid_at({at}) returned a retracted or superseded assertion"
            );
        }
    }
}

/// A snapshot is taken against one revision and says which — design § 71, and the reason § 72's
/// commit attempt can notice a mismatch at all.
#[test]
fn a_snapshot_names_the_revision_it_reads() {
    let fixture = fixture();
    let snapshot = GraphSnapshot::of(&fixture.graph);

    assert_eq!(snapshot.revision(), RevisionNumber::new(7));
    assert_eq!(snapshot.graph().assertions.len(), 4);
    assert_eq!(snapshot.graph().root.space, Space::Canonical);
    assert_eq!(
        snapshot.graph().ontology.version().id,
        fixture.graph.root.schema_version_id
    );
}

/// An assertion that never reached `Accepted` is not canonical knowledge, whatever its valid time.
///
/// Design § 21: the canonical core is what "has crossed the system's highest integrity boundary".
/// A `Proposed` record is in the graph and is not that, so the read never answers with it.
#[test]
fn a_proposed_assertion_is_never_answered() {
    let mut fixture = fixture();
    let proposed = AssertionId::mint();
    let mut candidate = fixture.graph.assertions[&fixture.bob].clone();
    candidate.id = proposed;
    candidate.validation = ValidationState::Proposed;
    candidate.valid_time = TemporalRange::UNBOUNDED;
    fixture.graph.assertions.insert(proposed, candidate);

    let snapshot = GraphSnapshot::of(&fixture.graph);
    for at in [Timestamp::EPOCH, DURING_ALICE, HANDOVER, AFTER_HANDOVER] {
        assert!(
            snapshot.valid_at(at).iter().all(|a| a.id != proposed),
            "valid_at({at}) answered with a proposal"
        );
    }
    assert_eq!(
        ValidationState::Proposed.name(),
        "Proposed",
        "the state names are the domain's, and the snapshot filters on them"
    );
    assert!(
        !ValidationState::Proposed.is_accepted(),
        "a proposal has not crossed the integrity boundary"
    );
    assert!(ValidationState::Accepted {
        validators: std::collections::BTreeSet::new()
    }
    .is_accepted());
}

/// A record whose transaction time has been closed is a record the graph no longer believes.
///
/// Design § 14.2: transaction time is "when the system believed or stored the assertion". Closing
/// `recorded_to` is how a correction removes a belief without deleting the history of having held
/// it, and the read must not answer with one at any `t` — the valid time says nothing about this.
#[test]
fn a_record_whose_transaction_time_is_closed_is_not_current() {
    let mut fixture = fixture();
    let mut bob = fixture.graph.assertions[&fixture.bob].clone();
    bob.transaction_time =
        TransactionTime::new(TENURE_BEGAN, Some(HANDOVER)).expect("withdrawn after it was formed");
    assert!(!bob.is_current());
    let id = bob.id;
    fixture.graph.assertions.insert(fixture.bob, bob);

    let snapshot = GraphSnapshot::of(&fixture.graph);
    for at in [HANDOVER, AFTER_HANDOVER, Timestamp::from_millis(i64::MAX)] {
        assert!(
            snapshot.valid_at(at).iter().all(|a| a.id != id),
            "valid_at({at}) answered with a record the runtime stopped believing"
        );
    }
}
