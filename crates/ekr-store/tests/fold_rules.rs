//! What the store refuses on its own, and what it leaves to the authority it was given.
//!
//! The fold used to hold a lineage to its rules itself: seed first and once, proposal before
//! validation, validation before commit, one revision per commit, a published knowledge root the
//! fold reproduces. Design § 91.5 moved every one of those into the kernel's fallible replay
//! authority — "Only the kernel validates and constructs admitted canonical state and roots" — and
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels` removed the raw `append`
//! every case here wrote its lineage through. Those rules are now held by the kernel's own suite
//! against real retained records; the cases that remain here are the ones the store still owns:
//!
//! * an empty lineage, and a store opened without an authority;
//! * `Initialize` accepts only a Seeded occurrence into an empty stream, once;
//! * every occurrence names retained bytes, and a lineage that does not refuses on read;
//! * a publication the authority does not admit is not written, and a retained occurrence it does
//!   not admit refuses reopen rather than folding away;
//! * the roots a head carries are the authority's, and `knowledge_root` / `evidence_root` — the two
//!   sub-roots this crate still computes — are functions of the state they address.
//!
//! Every case drives the SQLite provider; `providers.rs` holds the two providers to each other.
//! Self-contained rather than declaring `tests/fixture` or `tests/lineage`, which still name the
//! removed port.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EventId, EvidenceId, GraphRootId, NodeId,
    PropertyId, RevisionId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate, RevisionEvent,
    RevisionPayload, Root, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, GraphDocument, Initialize, Publication,
    PublicationObject, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

/// An ontology with no types.
///
/// The store type-checks nothing — `AGENTS.md` invariant 7 puts type validity in the kernel — so
/// the declarations would be decoration here.
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with two nodes, an edge between them, an assertion and the evidence it rests on.
///
/// Content in all four maps, so that `knowledge_root` and `evidence_root` are functions of
/// something rather than constants: an empty seed would make every "the address is the same"
/// assertion below pass for a store that lost the lot. Both ends of the edge are here — canonical
/// state is referentially complete (design § 21) — though the store checks none of it.
fn seed_graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let property = PropertyId::mint();
    let (subject, object) = (NodeId::mint(), NodeId::mint());
    let evidence_id = EvidenceId::mint();

    let mut observed = Node::new(subject, root_id, type_id, "revision-lineage");
    observed
        .properties
        .insert(property, vec![CanonicalValue::Decimal("1.0".to_owned())]);
    let reached = Node::new(object, root_id, type_id, "revision-lineage-target");

    let mut holds = Edge::new(
        EdgeId::mint(),
        root_id,
        type_id,
        CanonicalRef::new(subject),
        CanonicalRef::new(object),
    );
    holds
        .properties
        .insert(property, vec![CanonicalValue::Enum("canonical".to_owned())]);

    let evidence = Evidence {
        id: evidence_id,
        source: EvidenceSource::Document {
            document_id: "docs/roadmap.md".to_owned(),
            section: Some("P1".to_owned()),
        },
        content_hash: ContentHash::of_bytes(b"the roadmap's P1 exit criterion"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };

    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id,
        subject: Subject::Node(CanonicalRef::new(subject)),
        predicate: Predicate::Relation(type_id),
        object: Object::Node(CanonicalRef::new(object)),
        evidence: BTreeSet::from([evidence_id]),
        proposed_by: AgentId::mint(),
        assessment: Assessment::Accepted {
            validators: BTreeSet::new(),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::EPOCH),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

    CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::from([(subject, observed), (object, reached)]),
        edges: BTreeMap::from([(holds.id, holds)]),
        assertions: BTreeMap::from([(assertion.id, assertion)]),
        evidence: BTreeMap::from([(evidence_id, evidence)]),
    }
}

/// The record bytes of the only validation [`Attesting`] stands behind unless told otherwise.
const VALIDATION_RESULT: &[u8] = b"seven validators, no issues";

/// The agent root the stand-in reports: an address over its own name, not a zero placeholder.
fn agent_root() -> ContentHash {
    ContentHash::of_bytes(b"ekr-store fold_rules stand-in authority")
}

/// The stand-in for `ekr-kernel` in this binary.
///
/// `ekr-store` sits below `ekr-kernel` and cannot depend on it, so a case that needs a lineage to
/// advance needs something here to stand behind it. This does, and only this far: it admits the
/// seed document it was built from, read back out of the bytes the store retained; it stands behind
/// a commit only when that transaction's validation record is the one it was told to trust; and it
/// derives the head's roots the way design § 91.2 shapes them. Everything else — proposals, rejected
/// or stale decisions, revision numbers and published roots — it takes as bookkeeping, because
/// those rules are the kernel's and are held by the kernel's suite.
///
/// It is not a second writer to canonical state, and no `src/` implements this trait but the
/// kernel's, which `crates/ekr/tests/story_contract.rs` holds.
struct Attesting {
    graph: CanonicalGraph,
    stands_behind: &'static [u8],
    reads_seed: bool,
}

impl Attesting {
    fn of(graph: &CanonicalGraph) -> Self {
        Self {
            graph: graph.clone(),
            stands_behind: VALIDATION_RESULT,
            reads_seed: true,
        }
    }

    /// The same authority trusting a different validation record.
    fn standing_behind(self, stands_behind: &'static [u8]) -> Self {
        Self {
            stands_behind,
            ..self
        }
    }

    /// The same authority admitting the seed occurrence without reading the bytes it names.
    fn without_reading_the_seed(self) -> Self {
        Self {
            reads_seed: false,
            ..self
        }
    }
}

impl CommitAuthority for Attesting {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        let Some((first, later)) = history.occurrences.split_first() else {
            return Ok(None);
        };
        let RevisionPayload::Seeded {
            revision_id,
            seed_hash,
        } = first.event.payload
        else {
            return Err(StoreError::NotSeeded);
        };
        if self.reads_seed
            && history.content(seed_hash, StorageClass::Canonical)?
                != GraphDocument::of(&self.graph).to_bytes()?.as_slice()
        {
            return Err(StoreError::InvalidSeed(
                "not the seed this authority stands behind".to_owned(),
            ));
        }
        let mut head = AdmittedRevision {
            graph: self.graph.clone(),
            root: Root {
                revision: RevisionNumber::SEED,
                parent: None,
                ontology_root: ContentHash::of(&self.graph.ontology),
                knowledge_root: ekr_store::knowledge_root(&self.graph),
                evidence_root: ekr_store::evidence_root(&self.graph),
                agent_root: agent_root(),
                transaction: seed_hash,
            },
            revision_id,
            event_id: first.event.event_id,
            record_hash: first.event.record_hash,
            committed_at: Timestamp::EPOCH,
        };
        let mut attested = BTreeMap::new();
        for occurrence in later {
            if revision == Some(head.root.revision) {
                break;
            }
            match occurrence.event.payload {
                RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
                RevisionPayload::TransactionValidated { transaction_id, .. } => {
                    let record =
                        history.content(occurrence.event.record_hash, StorageClass::Canonical)?;
                    attested.insert(transaction_id, record == self.stands_behind);
                }
                RevisionPayload::RevisionCommitted {
                    transaction_id,
                    revision_id,
                    ..
                } => {
                    if attested.get(&transaction_id) != Some(&true) {
                        return Err(StoreError::ValidationMissing { transaction_id });
                    }
                    let number = head.root.revision.next().expect("a small test lineage");
                    let mut graph = head.graph.clone();
                    graph.revision = number;
                    head = AdmittedRevision {
                        graph,
                        root: Root {
                            revision: number,
                            parent: Some(ContentHash::of(&head.root)),
                            transaction: occurrence.event.record_hash,
                            ..head.root
                        },
                        revision_id,
                        event_id: occurrence.event.event_id,
                        record_hash: occurrence.event.record_hash,
                        committed_at: Timestamp::EPOCH,
                    };
                }
                RevisionPayload::TransactionProposed { .. }
                | RevisionPayload::TransactionRejected { .. }
                | RevisionPayload::TransactionStale { .. } => {}
            }
        }
        if let Some(requested) = revision.filter(|number| *number != head.root.revision) {
            return Err(StoreError::NoMaterialisedState { requested });
        }
        Ok(Some(head))
    }
}

/// A store over a fresh SQLite database in `directory`, typed by `ontology`, under `authority`.
fn open(
    directory: &TempDir,
    ontology: &Ontology,
    authority: impl CommitAuthority + 'static,
) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(authority)
}

/// One occurrence publishing `objects`, the first of which is its retained record.
fn occurrence(payload: RevisionPayload, objects: &[&[u8]], expected_version: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.to_owned(),
            event_id: EventId::mint(),
            record_hash: ContentHash::of_bytes(objects[0]),
            payload,
        },
        objects: objects
            .iter()
            .map(|bytes| {
                (
                    ContentHash::of_bytes(bytes),
                    PublicationObject {
                        storage_class: StorageClass::Canonical,
                        stored_at: Timestamp::EPOCH,
                        bytes: bytes.to_vec(),
                    },
                )
            })
            .collect(),
        expected_version,
    }
}

/// The Seeded occurrence for `graph`, staging the document it names beside its record.
fn seed(graph: &CanonicalGraph) -> Publication {
    let document = GraphDocument::of(graph)
        .to_bytes()
        .expect("the seed serialises");
    occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        &[b"a seed result", &document],
        0,
    )
}

/// A store in `directory` under [`Attesting`] for `graph`, holding `graph`'s seed and nothing else.
fn seeded(directory: &TempDir, ontology: &Ontology, graph: &CanonicalGraph) -> SqliteStore {
    let store = open(directory, ontology, Attesting::of(graph));
    assert_eq!(
        store.initialize(&seed(graph)),
        Ok(Appended::Written),
        "seeded"
    );
    store
}

/// Publishes `payload` with `record` at the current end of `store`'s lineage.
fn append(
    store: &SqliteStore,
    payload: RevisionPayload,
    record: &[u8],
) -> Result<Appended, StoreError> {
    let expected = store
        .history()
        .expect("the retained lineage verifies")
        .occurrences
        .len() as u64;
    store.publish(&occurrence(payload, &[record], expected))
}

/// Proposes and validates a fresh transaction, the validation carrying `validation` as its record,
/// and then publishes its commit as revision 1, answering the transaction and what the commit's
/// publication returned.
fn propose_validate_commit(
    store: &SqliteStore,
    graph: &CanonicalGraph,
    validation: &[u8],
) -> (TransactionId, Result<Appended, StoreError>) {
    let transaction = TransactionId::mint();
    let proposal = format!("proposal of {transaction}");
    assert_eq!(
        append(
            store,
            RevisionPayload::TransactionProposed {
                transaction_id: transaction,
                proposer: AgentId::mint(),
                operations_hash: None,
            },
            proposal.as_bytes(),
        ),
        Ok(Appended::Written),
        "proposed"
    );
    assert_eq!(
        append(
            store,
            RevisionPayload::TransactionValidated {
                transaction_id: transaction,
                against: RevisionNumber::SEED,
                validation_hash: ContentHash::of_bytes(validation),
            },
            validation,
        ),
        Ok(Appended::Written),
        "a validation is bookkeeping until something commits on it"
    );
    let receipt = format!("commit receipt of {transaction}");
    let committed = append(
        store,
        RevisionPayload::RevisionCommitted {
            transaction_id: transaction,
            revision_id: RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: ekr_store::knowledge_root(graph),
        },
        receipt.as_bytes(),
    );
    (transaction, committed)
}

#[test]
fn an_empty_log_has_no_head_and_does_not_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = open(&directory, &ontology, Attesting::of(&graph));

    assert_eq!(store.head().expect("an empty log folds"), None);
    assert!(
        matches!(store.fold(), Err(StoreError::NotSeeded)),
        "a fold with no seed has no state to fold onto"
    );
}

#[test]
fn a_lineage_that_does_not_begin_at_a_seed_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = open(&directory, &ontology, Attesting::of(&graph));

    let proposal = occurrence(
        RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        &[b"a proposal with no seed before it"],
        0,
    );
    assert_eq!(
        store.initialize(&proposal),
        Err(StoreError::InvalidSeed("invalid-seed-publication".into())),
        "the first occurrence of a lineage is its seed"
    );
    assert_eq!(store.head(), Ok(None), "and nothing was written");
    assert!(matches!(store.fold(), Err(StoreError::NotSeeded)));
}

#[test]
fn a_second_seed_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);
    let before = store.head().expect("the seed folds");

    // The same document, a new occurrence: a second seed, not a retry of the first.
    assert_eq!(
        store.initialize(&seed(&graph)),
        Err(StoreError::Conflict),
        "a lineage is seeded once; a second seed would silently restart it"
    );
    assert_eq!(store.head().expect("the seed still folds"), before);
    assert_eq!(
        store
            .history()
            .expect("the lineage verifies")
            .occurrences
            .len(),
        1,
        "the second seed was not written"
    );
}

#[test]
fn a_seed_naming_bytes_the_store_does_not_hold_is_refused() {
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let absent = ContentHash::of_bytes(b"a seed nobody stored");
    let unbacked = || {
        occurrence(
            RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: absent,
            },
            &[b"a seed result naming nothing"],
            0,
        )
    };
    let missing = StoreError::Document("required-object-missing".into());

    // At publication: the candidate replay reads the seed through the store's retained history.
    let directory = TempDir::new().expect("a temporary directory");
    let refused = open(&directory, &ontology, Attesting::of(&graph));
    assert_eq!(refused.initialize(&unbacked()), Err(missing.clone()));
    assert_eq!(refused.head(), Ok(None), "and nothing was written");

    // On read: the store requires every seed's bytes before it asks any authority, so an
    // authority that would admit the occurrence without reading it is never reached.
    let directory = TempDir::new().expect("a temporary directory");
    let lenient = open(
        &directory,
        &ontology,
        Attesting::of(&graph).without_reading_the_seed(),
    );
    assert_eq!(lenient.initialize(&unbacked()), Ok(Appended::Written));
    assert_eq!(
        lenient.head(),
        Err(missing.clone()),
        "a seed address that resolves to nothing is a lineage with no beginning"
    );
    assert_eq!(lenient.fold(), Err(missing));
}

/// Design § 91.2 decided what the two sub-roots that used to be a placeholder are.
///
/// `task:two-of-the-five-revision-sub-roots-are-placeholders`. `ontology_root` is the value-domain
/// hash of the complete loaded ontology and `agent_root` the hash of the full authority state, and
/// "a populated ontology or authority registry cannot be represented by a zero placeholder". Both
/// are derived by the kernel's authority; `crates/ekr-kernel/tests/durable_seed.rs`'s
/// `seed_roots_bind_unused_ontology_declarations_on_both_providers` holds the ontology root to its
/// content and to being non-zero.
///
/// What the store is held to is that it has no placeholder left to write: the head it reports is
/// exactly the root its authority admitted, sub-root for sub-root.
#[test]
fn the_store_reports_the_sub_roots_its_authority_derived_and_writes_no_placeholder() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);
    let (_, committed) = propose_validate_commit(&store, &graph, VALIDATION_RESULT);
    assert_eq!(committed, Ok(Appended::Written));

    let head = store
        .head()
        .expect("the log folds")
        .expect("a seeded log has a head");
    let admitted = Attesting::of(&graph)
        .replay(&store.history().expect("the lineage verifies"), None, None)
        .expect("the authority admits the lineage it stands behind")
        .expect("and it has a head")
        .root;
    assert_eq!(
        head, admitted,
        "the store reports the root its authority admitted"
    );

    let placeholder = ContentHash::from_bytes([0u8; 32]);
    assert_eq!(head.ontology_root, ContentHash::of(&ontology));
    assert_ne!(head.ontology_root, placeholder);
    assert_eq!(head.agent_root, agent_root());
    assert_ne!(head.agent_root, placeholder);
    assert_eq!(
        head.knowledge_root,
        ekr_store::knowledge_root(&graph),
        "knowledge from the graph state"
    );
    assert_eq!(
        head.evidence_root,
        ekr_store::evidence_root(&graph),
        "evidence from the retained evidence"
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
/// to hash an empty graph turned **no case red**: the fixture computed the root a commit published
/// with the same function the fold checked it against, so a stub agreed with itself.
#[test]
fn the_knowledge_root_is_a_function_of_the_graph_state() {
    let ontology = ontology();
    let graph = seed_graph(&ontology);

    let mut emptied = graph.clone();
    emptied.nodes = BTreeMap::new();
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
    without_edges.edges = BTreeMap::new();
    assert_ne!(
        ekr_store::knowledge_root(&graph),
        ekr_store::knowledge_root(&without_edges),
        "edges are in it"
    );

    let mut without_assertions = graph.clone();
    without_assertions.assertions = BTreeMap::new();
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
    let ontology = ontology();
    let graph = seed_graph(&ontology);

    let mut swept = graph.clone();
    swept.evidence = BTreeMap::new();

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

/// AGENTS.md invariant 1, at the store: a validation nothing stands behind does not commit — and,
/// since design § 91.5, says so.
///
/// The old fold ignored such a commit and reported no error, so a lineage holding one folded to
/// the seed as if nothing had happened. § 91.5 replaced that boolean attestation with a fallible
/// authority: "Corrupt commits refuse reopen; they do not silently disappear from the fold or
/// return the seed as if it were the latest revision." Two halves, then. A commit the authority
/// does not stand behind is refused at publication and is not written. And a retained commit an
/// authority does not stand behind — here, written while a different validation was trusted —
/// refuses the reopen rather than folding back to the seed.
#[test]
fn a_commit_whose_validation_the_authority_does_not_stand_behind_is_refused_not_folded_away() {
    let ontology = ontology();
    let graph = seed_graph(&ontology);

    let directory = TempDir::new().expect("a temporary directory");
    let store = seeded(&directory, &ontology, &graph);
    let (transaction, committed) = propose_validate_commit(
        &store,
        &graph,
        b"a validation this suite's authority never did",
    );
    assert_eq!(
        committed,
        Err(StoreError::ValidationMissing {
            transaction_id: transaction
        }),
        "the store returns the authority's refusal"
    );
    assert_eq!(
        store
            .head()
            .expect("the lineage still folds")
            .expect("a seeded lineage has a head")
            .revision,
        RevisionNumber::SEED,
        "and the lineage stays where it was"
    );
    assert_eq!(
        store
            .history()
            .expect("the lineage verifies")
            .occurrences
            .len(),
        3,
        "the seed, the proposal and the validation are retained; the refused commit is not"
    );

    let directory = TempDir::new().expect("a temporary directory");
    let written = seeded(&directory, &ontology, &graph);
    let (transaction, committed) = propose_validate_commit(&written, &graph, VALIDATION_RESULT);
    assert_eq!(committed, Ok(Appended::Written));
    drop(written);
    let reopened = open(
        &directory,
        &ontology,
        Attesting::of(&graph).standing_behind(b"a validation other than the one retained"),
    );
    let refusal = StoreError::ValidationMissing {
        transaction_id: transaction,
    };
    assert_eq!(
        reopened.head(),
        Err(refusal.clone()),
        "a retained commit nothing stands behind refuses the reopen, not returns the seed"
    );
    assert_eq!(reopened.fold(), Err(refusal));
}

/// The same lineage with the validation the authority *does* stand behind, which advances.
///
/// The case above is one record away from this one, and without this one it would pass for a
/// store that refused every commit — including the one it is supposed to accept.
#[test]
fn a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);

    let (_, committed) = propose_validate_commit(&store, &graph, VALIDATION_RESULT);
    assert_eq!(committed, Ok(Appended::Written), "the commit is written");

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

/// A store nobody gave an authority to says so, rather than folding every commit away in silence.
///
/// The refusals in this file's other cases are about a *lineage*. This one is about the **caller**,
/// and the difference is who can act on it. `SqliteStore::sqlite` is public and `under` is opt-in,
/// so a store constructed without an authority must not report a head that did not move as every
/// signal a consumer gets.
///
/// `AGENTS.md` invariant 1 names this case: a store without seed authority refuses. It refuses
/// `head` too, now — every read of a non-empty lineage goes through the authority, and a store
/// without one cannot say how far the lineage verifiably got either.
#[test]
fn a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let written = seeded(&directory, &ontology, &graph);
    let (_, committed) = propose_validate_commit(&written, &graph, VALIDATION_RESULT);
    assert_eq!(committed, Ok(Appended::Written));
    drop(written);

    // Deliberately not this file's `open`, which opens under an authority.
    let store = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens");

    assert_eq!(store.fold(), Err(StoreError::NoSeedAuthority));
    assert_eq!(
        store.replay(RevisionNumber::SEED),
        Err(StoreError::NoSeedAuthority)
    );
    assert_eq!(store.head(), Err(StoreError::NoSeedAuthority));

    // The same lineage under an authority that stands behind it folds.
    let authorised = open(&directory, &ontology, Attesting::of(&graph));
    assert_eq!(
        authorised
            .fold()
            .expect("the same bytes, with somebody to ask")
            .revision,
        RevisionNumber::new(1),
        "the refusal is about the store's construction and not about the lineage"
    );
}
