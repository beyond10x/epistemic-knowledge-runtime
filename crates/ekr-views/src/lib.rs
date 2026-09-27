//! Rendered views of committed revisions: the graph projection ekr.graph-projection/1.
//!
//! Implements the `ekr.views` domain, `systems/ekr/domains/views.yaml`: one JSON document per
//! committed revision, scoped to canonical state as of that revision (design § 47), every fact
//! carrying the id of the assertion it came from (§ 82, § 62), the schema lineage (§ 95) and
//! evidence identities without their bytes (§ 86).
//!
//! Two halves, so that determinism is a property of a pure function:
//!
//! * [`load`] reads one revision through the kernel's verified read surface — [`Runtime::head`],
//!   [`Runtime::read`], [`Runtime::replay`] for each earlier schema version and
//!   [`Runtime::content`] for evidence retention — into a [`LoadedRevision`].
//! * [`render`] turns a [`LoadedRevision`] into the document's bytes and nothing else: no clock,
//!   no host, no path, no provider.
//!
//! [`project`] is the two in sequence. **Reads only**: nothing here proposes, validates, commits,
//! seeds or opens a provider, and `tests/reads_only.rs` reads that off this crate's source. The
//! caller opens the [`Runtime`], which is the host's business.
//!
//! The format names no domain concept: every name in it comes from the projected ontology or the
//! graph state (AGENTS.md invariant 8).

mod document;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{ContentHash, RevisionNumber, SchemaVersionId, Timestamp, TransactionId};
use ekr_graph::CanonicalGraph;
use ekr_kernel::{CommitError, PersistenceError, Runtime};
use ekr_ontology::Ontology;

/// The format literal every projection carries in `meta.format`.
pub const FORMAT: &str = "ekr.graph-projection/1";

/// What one render returned, counted over the document: `ekr.views.GraphProjected`.
///
/// Every field is a function of the document's bytes, so this is as deterministic as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphProjected {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.head`.
    pub head: u64,
    /// `meta.node_count`.
    pub nodes: u64,
    /// `meta.edge_count`.
    pub edges: u64,
    /// `meta.assertion_count`.
    pub assertions: u64,
    /// `meta.evidence_count`.
    pub evidence: u64,
    /// Entries of `schema.versions`.
    pub schema_versions: u64,
    /// Entries of `schema.revisions`.
    pub revisions: u64,
    /// Entries of `schema.revisions` that carry a `transaction_id`.
    pub transactions: u64,
    /// Entries of `ontology.node_types`.
    pub node_types: u64,
    /// Entries of `ontology.edge_types`.
    pub edge_types: u64,
    /// Entries of `ontology.properties`.
    pub properties: u64,
    /// Assertion entries under `edges[]`.
    pub edge_assertions: u64,
    /// Assertion entries, anywhere in the document, whose lifecycle is `Retracted`.
    pub retracted_assertions: u64,
    /// Evidence entries marked `retained`.
    pub retained_evidence: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub projection_hash: String,
}

/// One rendered projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    /// The exact document bytes: UTF-8 JSON with no insignificant whitespace.
    pub bytes: Vec<u8>,
    /// The counts of the document.
    pub summary: GraphProjected,
}

/// Why nothing was rendered.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// `ekr.views.NotSeeded`: the store has no revision and no head.
    #[error("the store was never seeded")]
    NotSeeded {
        /// The requested revision, echoed, if one was named.
        requested: Option<RevisionNumber>,
    },
    /// `ekr.views.RevisionNotFound`: no committed revision has the requested number.
    #[error("the store holds no revision {requested}; its head is {head}")]
    RevisionNotFound {
        /// The requested revision.
        requested: RevisionNumber,
        /// The store's newest committed revision.
        head: RevisionNumber,
    },
    /// The kernel's verified read refused the store's history.
    #[error("the verified read refused: {0}")]
    Read(String),
    /// The revision holds state `ekr.graph-projection/1` cannot represent without losing part of
    /// it. Two causes. One is an assertion whose subject the revision does not hold, which a
    /// revision the kernel admitted never has. The other is one property id that two types
    /// declare with a different name or a different value kind: the ontology admits it, and the
    /// format's single `ontology.properties` entry per id carries one name and one value kind
    /// (task:projection-carries-per-type-property-definitions). Declarations that differ only in
    /// what that entry does not carry, such as `required`, render.
    #[error("the projected revision is inconsistent: {0}")]
    Inconsistent(String),
}

impl From<PersistenceError> for ProjectError {
    fn from(error: PersistenceError) -> Self {
        Self::Read(error.to_string())
    }
}

impl From<CommitError> for ProjectError {
    fn from(error: CommitError) -> Self {
        Self::Read(error.to_string())
    }
}

/// One committed revision up to and including the projected one, as loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedRevisionEntry {
    /// Its number.
    pub number: RevisionNumber,
    /// When it was committed, as the store retains it.
    pub committed_at: Timestamp,
    /// The transaction that produced it; `None` for the seed.
    pub transaction_id: Option<TransactionId>,
    /// The schema version its canonical state is valid against.
    pub schema_version: SchemaVersionId,
}

/// Everything [`render`] reads, already loaded: the pure function's whole input.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedRevision {
    /// Canonical state as of the projected revision.
    pub graph: CanonicalGraph,
    /// The store's newest committed revision when this was loaded.
    pub head: RevisionNumber,
    /// Every revision from the seed through the projected one, ascending.
    pub revisions: Vec<LoadedRevisionEntry>,
    /// The ontology of every schema version some listed revision is valid against, with the
    /// first listed revision valid against it.
    pub schemas: BTreeMap<SchemaVersionId, (RevisionNumber, Ontology)>,
    /// The content hashes, among the projected evidence's, whose bytes the store holds.
    pub retained: BTreeSet<ContentHash>,
}

/// Loads revision `at` of `runtime`'s store, or its head when `at` is `None`.
///
/// Reads the head first, so an unseeded store answers [`ProjectError::NotSeeded`] whatever `at`
/// names, and a seeded one [`ProjectError::RevisionNotFound`] for any `at` beyond its head.
/// The schema history replays only the revisions whose ontology root differs from the one before
/// it — one per schema version — rather than every revision.
///
/// # Errors
///
/// [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`], or the kernel's refusal of
/// the store's history as [`ProjectError::Read`].
pub fn load(runtime: &Runtime, at: Option<RevisionNumber>) -> Result<LoadedRevision, ProjectError> {
    let head = runtime
        .head()?
        .ok_or(ProjectError::NotSeeded { requested: at })?
        .revision;
    let revision = at.unwrap_or(head);
    if revision > head {
        return Err(ProjectError::RevisionNotFound {
            requested: revision,
            head,
        });
    }
    let read = runtime.read(Some(revision)).map_err(|error| match error {
        CommitError::RevisionNotFound { against } => ProjectError::RevisionNotFound {
            requested: against,
            head,
        },
        CommitError::NotSeeded => ProjectError::NotSeeded { requested: at },
        other => other.into(),
    })?;
    if read.graph.revision != revision {
        return Err(ProjectError::Inconsistent(format!(
            "asked for revision {revision}, the verified read holds {}",
            read.graph.revision
        )));
    }
    let transactions: BTreeMap<RevisionNumber, TransactionId> = read
        .transactions
        .iter()
        .filter_map(|(id, record)| {
            record
                .committed
                .as_ref()
                .map(|receipt| (receipt.result.revision, *id))
        })
        .collect();

    let mut revisions = Vec::new();
    let mut schemas: BTreeMap<SchemaVersionId, (RevisionNumber, Ontology)> = BTreeMap::new();
    let mut previous: Option<(ContentHash, SchemaVersionId)> = None;
    for number in (0..=revision.get()).map(RevisionNumber::new) {
        let coordinates = read.revisions.get(&number).ok_or_else(|| {
            ProjectError::Inconsistent(format!("the verified read lists no revision {number}"))
        })?;
        let ontology_root = coordinates.root.ontology_root;
        let version = match previous {
            Some((root, version)) if root == ontology_root => version,
            _ => {
                let ontology = if number == revision {
                    read.graph.ontology.clone()
                } else {
                    runtime.replay(number)?.ontology
                };
                let version = ontology.version().id;
                schemas.entry(version).or_insert((number, ontology));
                version
            }
        };
        previous = Some((ontology_root, version));
        revisions.push(LoadedRevisionEntry {
            number,
            committed_at: coordinates.committed_at,
            transaction_id: transactions.get(&number).copied(),
            schema_version: version,
        });
    }

    let mut retained = BTreeSet::new();
    for evidence in read.graph.evidence.values() {
        if runtime.content(&evidence.content_hash)?.is_some() {
            retained.insert(evidence.content_hash);
        }
    }
    Ok(LoadedRevision {
        graph: read.graph,
        head,
        revisions,
        schemas,
        retained,
    })
}

/// Renders `loaded` to the bytes of `ekr.graph-projection/1` and counts them.
///
/// Pure: the same [`LoadedRevision`] renders the same bytes on every call, in every process.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] for state the format cannot place.
pub fn render(loaded: &LoadedRevision) -> Result<Rendered, ProjectError> {
    document::render(loaded)
}

/// Loads revision `at` (the head when `None`) and renders it: [`load`] then [`render`].
///
/// # Errors
///
/// Whatever [`load`] or [`render`] refuses.
pub fn project(runtime: &Runtime, at: Option<RevisionNumber>) -> Result<Rendered, ProjectError> {
    render(&load(runtime, at)?)
}
