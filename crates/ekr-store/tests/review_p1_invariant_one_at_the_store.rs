//! Independent review of the P1 core: AGENTS.md invariant 1, read at the store boundary.
//!
//! The invariant has two halves. "Only `ekr-kernel` constructs a `ValidatedTransaction`" is held by
//! the two compile-fail cases in `crates/ekr-kernel/tests/compile_fail/`, and this review could not
//! fault it. "Only a `ValidatedTransaction` commits" is the half these cases attack, and they attack
//! the gap rather than the sentence: `ekr-store` sits *below* `ekr-kernel` in the workspace order
//! (`docs/roadmap.md` § 3, `crates/ekr-store/Cargo.toml`), so this test binary cannot name
//! `ValidatedTransaction` at all — nothing in this process has ever constructed one.
//!
//! When the review ran, `RevisionLog::append` was public and took a bare `RevisionEvent`, and the
//! lineage advanced on three hand-written events. `append` is gone
//! (`architecture-decision-record:0007-the-commit-path-is-the-kernels`, design § 91.5): every path
//! that writes a revision occurrence now replays the candidate through the injected
//! [`CommitAuthority`] first. The case below drives every one of those paths with the same
//! hand-written commit, and holds the list of paths to the port's own declarations.
//!
//! The second case this file used to carry — a commit validated against a revision that is no
//! longer the head — is the kernel's now, held by
//! `crates/ekr-kernel/tests/commit_path.rs::a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused`.
//!
//! Self-contained rather than declaring `tests/fixture` or `tests/lineage`, which still name the
//! removed port.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ekr_core::{
    AgentId, ContentHash, EventId, GraphRootId, RevisionId, RevisionNumber, SchemaVersionId,
    Timestamp, TransactionId,
};
use ekr_graph::{CanonicalGraph, GraphRoot, RevisionEvent, RevisionPayload, Root, Space};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, GraphDocument, Initialize, NativeCommandMeta,
    NativePublicationRequest, ObjectStore, Publication, PublicationCommandKey,
    PublicationCommandKind, PublicationObject, PublicationPreparationV1, RetainedHistory,
    RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

/// An ontology with no declarations: the store type-checks nothing (invariant 7).
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with no content. The case is about what advances the lineage, not about what is in it.
fn graph(ontology: &Ontology) -> CanonicalGraph {
    CanonicalGraph {
        root: GraphRoot {
            id: GraphRootId::mint(),
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

/// One occurrence publishing `objects`, the first of which is its retained record.
fn publication(payload: RevisionPayload, objects: &[&[u8]], expected_version: u64) -> Publication {
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

/// The stand-in for `ekr-kernel` this binary may have: it admits a seed and stands behind no
/// validation at all, so it refuses every commit.
///
/// It is not a second writer to canonical state, and no `src/` implements this trait but the
/// kernel's, which `crates/ekr/tests/story_contract.rs` holds.
struct SeedOnly(CanonicalGraph);

impl CommitAuthority for SeedOnly {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
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
        for occurrence in later {
            if let RevisionPayload::RevisionCommitted { transaction_id, .. } =
                occurrence.event.payload
            {
                return Err(StoreError::ValidationMissing { transaction_id });
            }
        }
        Ok(Some(AdmittedRevision {
            graph: self.0.clone(),
            root: Root {
                revision: RevisionNumber::SEED,
                parent: None,
                ontology_root: ContentHash::of(&self.0.ontology),
                knowledge_root: ekr_store::knowledge_root(&self.0),
                evidence_root: ekr_store::evidence_root(&self.0),
                agent_root: ContentHash::of_bytes(b"a seed-only stand-in"),
                transaction: seed_hash,
            },
            revision_id,
            event_id: first.event.event_id,
            record_hash: first.event.record_hash,
            committed_at: Timestamp::EPOCH,
        }))
    }
}

/// A well-formed preparation argument that no store ever elected.
fn never_elected(key: &PublicationCommandKey, decision: &Publication) -> PublicationPreparationV1 {
    PublicationPreparationV1 {
        format: PublicationPreparationV1::FORMAT.to_owned(),
        command_key: key.clone(),
        input_hash: ContentHash::of_bytes(b"a commit command nobody validated"),
        decision: decision.clone(),
        attempt_number: 0,
        previous_attempt_hash: None,
        native_request: NativePublicationRequest {
            tenant: "ekr".to_owned(),
            appends: Vec::new(),
            meta: NativeCommandMeta {
                idempotency_key: String::new(),
                request_hash: String::new(),
                subject: String::new(),
                actor: String::new(),
                request_id: String::new(),
                trace_id: String::new(),
                causation_id: None,
                causation_depth: 0,
                occurred_at_unix_nanos: "0".to_owned(),
                occurred_at_offset_seconds: 0,
                claim: None,
            },
            blobs: Vec::new(),
        },
        native_fingerprint: String::new(),
    }
}

/// Every method of the three persistence ports and every inherent `pub fn` of the store, read off
/// `src/`, so that a writer added to the port is a writer this case must drive.
fn declared_entry_points() -> BTreeSet<String> {
    let source = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let read = |file: &str| {
        std::fs::read_to_string(source.join(file))
            .unwrap_or_else(|error| panic!("reading src/{file}: {error}"))
    };
    let names = |text: &str, head: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.trim_start().strip_prefix(head))
            .map(|rest| {
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect()
    };
    let mut declared = BTreeSet::new();
    for (file, port) in [
        ("log.rs", "RevisionLog"),
        ("log.rs", "Initialize"),
        ("objects.rs", "ObjectStore"),
    ] {
        let text = read(file);
        let start = text
            .find(&format!("pub trait {port} {{"))
            .unwrap_or_else(|| panic!("src/{file} no longer declares {port}"));
        let body = &text[start..];
        let body = &body[..body.find("\n}").expect("the trait closes")];
        declared.extend(names(body, "fn "));
    }
    declared.extend(names(&read("eventlog.rs"), "pub fn "));
    declared
}

/// The store's read-only entry points and its constructors: none of them can write an occurrence.
///
/// `published_events` (wave p1-14) reads the provider's own feed through the open handle and
/// nothing else. `tests/published_events.rs` and ekr-kernel's `tests/runtime_published_events.rs`
/// hold that a reread and a reopen return the identical log, so the read itself adds nothing.
const NOT_WRITERS: [&str; 12] = [
    "preparation",
    "published_events",
    "history",
    "history_at",
    "seed_bytes",
    "fold",
    "head",
    "replay",
    "get",
    "sqlite",
    "file",
    "under",
];

/// The gap, not the sentence: a commit written with no `ValidatedTransaction` anywhere in the
/// process, through every writer the store's public surface has.
///
/// This binary links `ekr-core`, `ekr-ontology`, `ekr-graph` and `ekr-store`. It does not and
/// cannot link `ekr-kernel`, which is the only crate that constructs a `ValidatedTransaction`. The
/// name is the review's finding; the assertion is the invariant, and it is what
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels` names as its exit criterion.
#[test]
fn a_commit_lands_with_no_validated_transaction_anywhere_in_the_process() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = graph(&ontology);
    let store = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(SeedOnly(graph.clone()));
    let document = GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = publication(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        &[b"a seed result", &document],
        0,
    );
    assert_eq!(store.initialize(&seed), Ok(Appended::Written), "seeded");

    let transaction = TransactionId::mint();
    let proposal = publication(
        RevisionPayload::TransactionProposed {
            transaction_id: transaction,
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        &[b"a proposal this test wrote"],
        1,
    );
    assert_eq!(store.publish(&proposal), Ok(Appended::Written), "proposed");
    // A validation nothing validated: no pipeline ran, and the record is a string this test made
    // up. The store writes it, because a validation is not a commit.
    let invented = b"no validator ran; this test wrote the event";
    let validation = publication(
        RevisionPayload::TransactionValidated {
            transaction_id: transaction,
            against: RevisionNumber::SEED,
            validation_hash: ContentHash::of_bytes(invented),
        },
        &[invented],
        2,
    );
    assert_eq!(
        store.publish(&validation),
        Ok(Appended::Written),
        "validated"
    );

    let commit = publication(
        RevisionPayload::RevisionCommitted {
            transaction_id: transaction,
            revision_id: RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: ekr_store::knowledge_root(&graph),
        },
        &[b"a commit receipt this test wrote"],
        3,
    );
    let refused = StoreError::ValidationMissing {
        transaction_id: transaction,
    };
    let key = PublicationCommandKey {
        kind: PublicationCommandKind::Commit,
        transaction_id: Some(transaction),
        predecessor_event_id: Some(validation.event.event_id),
        predecessor_record_hash: Some(validation.event.record_hash),
    };
    let mut driven = BTreeSet::new();
    assert_eq!(store.publish(&commit), Err(refused.clone()), "publish");
    driven.insert("publish");
    assert_eq!(
        store
            .prepare(&key, ContentHash::of_bytes(b"commit input"), &commit, None)
            .map(|_| ()),
        Err(refused),
        "prepare: the recoverable path the kernel itself publishes through"
    );
    driven.insert("prepare");
    assert_eq!(
        store.preparation(&key),
        Ok(None),
        "no preparation was elected"
    );
    assert_eq!(
        store.resume(&never_elected(&key, &commit)),
        Err(StoreError::Document("preparation-not-elected".into())),
        "resume"
    );
    driven.insert("resume");
    assert_eq!(
        store.initialize(&commit),
        Err(StoreError::InvalidSeed("invalid-seed-publication".into())),
        "initialize"
    );
    driven.insert("initialize");
    // The object writers hold bytes, never a revision occurrence.
    store
        .put(
            StorageClass::Canonical,
            b"a commit receipt this test wrote",
            Timestamp::EPOCH,
        )
        .expect("an object write is not a commit");
    driven.insert("put");
    store
        .store_graph(&graph, Timestamp::EPOCH)
        .expect("an archived document is not a commit");
    driven.insert("store_graph");

    let mut placed: BTreeSet<String> = driven.into_iter().map(str::to_owned).collect();
    placed.extend(NOT_WRITERS.map(str::to_owned));
    assert_eq!(
        placed,
        declared_entry_points(),
        "a store entry point is neither driven with the unvalidated commit nor listed as one that \
         cannot write an occurrence"
    );

    let head = store
        .head()
        .expect("the log folds")
        .expect("a seeded log has a head");
    assert_eq!(
        head.revision,
        RevisionNumber::SEED,
        "AGENTS.md invariant 1 says only a ValidatedTransaction commits. This process links \
         ekr-core, ekr-ontology, ekr-graph and ekr-store and not ekr-kernel, so no \
         ValidatedTransaction exists anywhere in it — and the canonical lineage advanced to \
         revision {} on a hand-written commit",
        head.revision
    );
    assert_eq!(
        store
            .history()
            .expect("the retained lineage verifies")
            .occurrences
            .len(),
        3,
        "the seed, the proposal and the validation are retained, and the commit is not"
    );
}
