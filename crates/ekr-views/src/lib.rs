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
//!   [`Runtime::schema_history`] for the revision's graph and every schema version of its lineage
//!   from one replay, and [`Runtime::content`] for evidence retention — into a
//!   [`LoadedRevision`].
//! * [`render`] turns a [`LoadedRevision`] into the document's bytes and nothing else: no clock,
//!   no host, no path, no provider.
//!
//! [`project`] is the two in sequence. **Reads only**: nothing here proposes, validates, commits,
//! seeds or opens a provider, and `tests/reads_only.rs` reads that off this crate's source. The
//! caller opens the [`Runtime`], which is the host's business.
//!
//! The format names no domain concept: every name in it comes from the projected ontology or the
//! graph state (AGENTS.md invariant 8).
//!
//! Five bounded reads draw from the same loaded revision without exporting it. [`Index::build`]
//! indexes a [`LoadedRevision`] once; [`Index::overview`] (`ekr.graph-overview/1`),
//! [`Index::expand`] (`ekr.graph-slice/1`, with [`Index::page`] for a host that streams the
//! records), [`Index::describe`] (`ekr.node-detail/1`), [`Index::search`]
//! (`ekr.node-matches/1`) and [`Index::timeline`] (`ekr.graph-timeline/1`) then answer from it,
//! each costing its answer. [`OverviewRequest`], [`ExpandRequest`], [`SearchRequest`] and
//! [`TimelineRequest`] hold each command's bounds, so a broken one is refused as
//! [`LimitExceeded`] before a store is read. [`IndexCache`] keeps the most recently used indexes
//! for a host.
//!
//! A sixth, [`Index::changes`] (`ekr.graph-changes/1`), answers what the revisions up to the
//! indexed one changed after a revision or a time, reading each chosen revision's committed
//! transaction; [`ChangesRequest`] holds its `since` and bounds.
//!
//! [`find_code_names`] (`ekr.code-names/1`) answers which of a revision's names a consumer's source
//! files carry as literals, given their text as [`SourceText`]s; [`code_names`] is its pure half
//! over a [`LoadedRevision`], and [`literals`] what it counts as a literal.
//!
//! [`report_quality`] (`ekr.store-quality/1`) reports how well one revision's assertions are
//! evidenced, its properties constrained and its nodes of one type named apart; [`quality`] is
//! its pure half.
//!
//! [`export_ocel`] (`ekr.ocel/1`) exports one revision as an OCEL 2.0 object-centric event log,
//! its event types the overview's or the ones a request names; [`ocel`] is its pure half.
//!
//! [`draw_sample`] (`ekr.fact-sample/1`) draws a reproducible sample of one revision's facts, each
//! with the bytes of its evidence, for a judge; [`sample`] is its pure half.
//! [`report_fact_quality`] (`ekr.fact-quality/1`) reports the pass rate of the judged sample,
//! [`FactJudgements`], with its Wilson score interval; it reads no store. The runtime judges
//! nothing.
//!
//! [`Index::event_types`] is EKR's one rule for which node types are events: the timeline,
//! [`ocel`] and [`Index::view_roles_document`] (`ekr.view-roles/1`, the host's `GET /roles`) all
//! read it.

mod changes;
mod code_names;
mod document;
mod index;
mod ocel;
mod quality;
mod query;
mod roles;
mod sample;
mod timeline;

pub use changes::{
    ChangesError, ChangesListed, ChangesRequest, SinceKind, SinceMalformed, CHANGES_FORMAT,
};
pub use code_names::{
    code_names, find_code_names, literals, runtime_vocabulary, CodeNamesFound, Literal, SourceText,
    CODE_NAMES_FORMAT, EMBEDDED_DOMAINS,
};
pub use index::{Index, IndexCache};
pub use ocel::{export_ocel, ocel, OcelError, OcelExported, OCEL_FORMAT};
pub use quality::{quality, report_quality, StoreQualityReported, QUALITY_FORMAT};
pub use query::{
    Answer, ExpandRequest, GraphOverviewed, LimitExceeded, NeighbourhoodExpanded, NodeDescribed,
    NodeSummary, NodesSearched, OverviewRequest, QueryError, SearchRequest, SliceEdge, SliceMeta,
    SliceNode, SlicePage, SliceRecord, DETAIL_FORMAT, MATCHES_FORMAT, OVERVIEW_FORMAT,
    SLICE_FORMAT,
};
pub use roles::{Role, ROLES_FORMAT};
pub use sample::{
    draw_key, draw_sample, report_fact_quality, sample, wilson_interval, wilson_z, FactJudgements,
    FactQualityError, FactQualityReported, FactSampleDrawn, Judgement, JudgementsMalformed,
    SampleOrigin, SampleRequest, Verdict, DEFAULT_CONFIDENCE, FACT_QUALITY_FORMAT,
    JUDGEMENTS_FORMAT, SAMPLE_FORMAT,
};
pub use timeline::{BucketWidth, SubjectsTimelined, TimelineRequest, TIMELINE_FORMAT};

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
    /// Entries of `ontology.properties`: one per definition of each property id, so more than the
    /// ids when two types define one differently.
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
    /// The store refused the history the runtime observed as diverged from the store at its path
    /// (`PersistenceError::Diverged`): a long-running reader opens the store again on it. It reads
    /// as [`ProjectError::Read`] does.
    #[error("the verified read refused: {0}")]
    Diverged(String),
    /// The store refused to answer because the SQLite database at its path is no longer the one
    /// the runtime opened (`PersistenceError::Replaced`, `store-replaced`): a long-running reader
    /// opens the store again on it, as on [`ProjectError::Diverged`]. It reads as
    /// [`ProjectError::Read`] does.
    #[error("the verified read refused: {0}")]
    Replaced(String),
    /// The revision holds state `ekr.graph-projection/1` cannot represent without losing part of
    /// it, such as an assertion whose subject the revision does not hold, which a revision the
    /// kernel admitted never has. One property id that two types declare with a different name or
    /// value kind is not such state: `ontology.properties` gives each definition its own entry,
    /// naming its `owners` (task:projection-carries-per-type-property-definitions).
    #[error("the projected revision is inconsistent: {0}")]
    Inconsistent(String),
}

impl From<PersistenceError> for ProjectError {
    fn from(error: PersistenceError) -> Self {
        match error {
            error @ PersistenceError::Diverged(_) => Self::Diverged(error.to_string()),
            error @ PersistenceError::Replaced(_) => Self::Replaced(error.to_string()),
            error => Self::Read(error.to_string()),
        }
    }
}

impl From<CommitError> for ProjectError {
    fn from(error: CommitError) -> Self {
        match error {
            error @ CommitError::Store(PersistenceError::Diverged(_)) => {
                Self::Diverged(error.to_string())
            }
            error @ CommitError::Store(PersistenceError::Replaced(_)) => {
                Self::Replaced(error.to_string())
            }
            error => Self::Read(error.to_string()),
        }
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

/// Everything [`render`] reads, already loaded: the pure function's whole input. It holds
/// nothing about the store beyond the projected revision — not the head it was loaded under — so
/// a later commit does not change what a revision loads to.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedRevision {
    /// Canonical state as of the projected revision: the kernel's verified graph, shared with it
    /// rather than copied from it.
    pub graph: Arc<CanonicalGraph>,
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
/// Reads the head first only when `at` is `None`; a named revision is read directly, and the head
/// only when that read finds no such revision. An unseeded store answers
/// [`ProjectError::NotSeeded`] whatever `at` names, and a seeded one
/// [`ProjectError::RevisionNotFound`], carrying its head, for any `at` beyond its head.
/// The graph and the schema history come from one kernel read, [`Runtime::schema_history`], which
/// replays the store's history at most once whatever the number of schema versions.
///
/// # Errors
///
/// [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`], or the kernel's refusal of
/// the store's history as [`ProjectError::Read`].
pub fn load(runtime: &Runtime, at: Option<RevisionNumber>) -> Result<LoadedRevision, ProjectError> {
    let head = || -> Result<RevisionNumber, ProjectError> {
        Ok(runtime
            .head()?
            .ok_or(ProjectError::NotSeeded { requested: at })?
            .revision)
    };
    // A requested revision is read before the head is: the kernel keeps only a few revisions'
    // graphs, and the replay that verifies the history keeps the requested one's only while that
    // read runs, so reading the head first could cost a second replay.
    let revision = match at {
        Some(revision) => revision,
        None => head()?,
    };
    let mut read = runtime
        .schema_history(revision)
        .map_err(|error| match error {
            CommitError::RevisionNotFound { against } => match head() {
                Ok(head) => ProjectError::RevisionNotFound {
                    requested: against,
                    head,
                },
                Err(error) => error,
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
                let ontology = read.schemas.remove(&number).ok_or_else(|| {
                    ProjectError::Inconsistent(format!(
                        "the verified read holds no ontology for revision {number}"
                    ))
                })?;
                let version = ontology.version().id;
                schemas.entry(version).or_insert((number, ontology));
                version
            }
        };
        previous = Some((ontology_root, version));
        revisions.push(LoadedRevisionEntry {
            number,
            committed_at: coordinates.committed_at,
            transaction_id: read.transactions.get(&number).copied(),
            schema_version: version,
        });
    }

    // Retention comes from the same verified read, in one pass: asking the runtime per entry
    // reloaded the whole history each time, which made a render quadratic in evidence entries.
    Ok(LoadedRevision {
        graph: read.graph,
        revisions,
        schemas,
        retained: read.retained_evidence,
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
