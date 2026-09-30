//! `ChangesSince` and its format `ekr.graph-changes/1` ([`Index::changes`]): what the committed
//! revisions up to one revision changed after a revision or a time (`views.yaml`,
//! `ekr.views.GraphChangesV1`).
//!
//! A request is bounded when it is built — [`ChangesRequest::new`] — so a malformed `since` and a
//! broken bound are refused before any store is read, in that order. The read then answers from
//! the [`Index`] of revision `at`, which the host already holds, and from the kernel's retained
//! records:
//!
//! * the revisions a `since` chooses are found in the index's revision list: a revision since
//!   chooses those after it, a transaction time those committed after it (a suffix, since commit
//!   times never decrease), and a valid time every one from the seed;
//! * each chosen revision's changes are read off the transaction it committed —
//!   [`Runtime::transactions`] is one verified capture of the retained records, and only the
//!   chosen revisions' documents are parsed — and the seed's off its canonical state,
//!   [`Runtime::replay`] of revision 0, read only when the seed is chosen;
//! * a superseded or retracted assertion's subject, valid time and evidence are the index's
//!   graph's, which holds every assertion ever added, whatever its lifecycle.
//!
//! So the parsing and the records built cost the chosen revisions and their changes, not the
//! whole history; what does not narrow is the kernel's capture of the retained transaction
//! records, which [`Runtime`] offers only whole, and a valid-time since, which chooses by the
//! valid time of each change and so reads every revision up to `at`.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, RevisionNumber, TypeId};
use ekr_graph::{CanonicalGraph, Subject};
use ekr_kernel::{GraphOperation, Runtime, TransactionDocument};
use serde::Serialize;

use crate::index::Index;
use crate::query::{bounded, encode, hash, Answer, LimitExceeded};
use crate::{LoadedRevisionEntry, ProjectError};

/// The format literal of [`Index::changes`]'s documents.
pub const CHANGES_FORMAT: &str = "ekr.graph-changes/1";

/// `ekr.views.SinceKind`: which of the three a `since` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SinceKind {
    /// A revision number: the changes of the revisions after it.
    Revision,
    /// A valid time, in milliseconds since the epoch: the assertion changes valid after it.
    ValidTime,
    /// A transaction time, in milliseconds since the epoch: the changes of the revisions
    /// committed after it.
    TransactionTime,
}

impl SinceKind {
    /// The variant's name as `views.yaml` declares it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Revision => "Revision",
            Self::ValidTime => "ValidTime",
            Self::TransactionTime => "TransactionTime",
        }
    }

    /// The `since` a host that takes it as three inputs — `since_revision`, `since_valid` and
    /// `since_recorded` — was given: exactly one of them, with its value. Given none, or more
    /// than one, it answers the names of those it was given, in that order, for the host to
    /// refuse as its own malformed input.
    ///
    /// # Errors
    ///
    /// The names given, when they are not exactly one.
    pub fn one_of(
        since_revision: Option<i64>,
        since_valid: Option<i64>,
        since_recorded: Option<i64>,
    ) -> Result<(Self, i64), Vec<&'static str>> {
        let given: Vec<(Self, &'static str, i64)> = [
            (Self::Revision, "since_revision", since_revision),
            (Self::ValidTime, "since_valid", since_valid),
            (Self::TransactionTime, "since_recorded", since_recorded),
        ]
        .into_iter()
        .filter_map(|(kind, name, value)| value.map(|value| (kind, name, value)))
        .collect();
        match given.as_slice() {
            [(kind, _, value)] => Ok((*kind, *value)),
            _ => Err(given.iter().map(|(_, name, _)| *name).collect()),
        }
    }
}

/// `ekr.views.SinceMalformed`: a `since` that is no revision, valid time or transaction time — a
/// revision below 0. The store was not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the {} since {requested} names no revision; a revision is 0 or more", .kind.name())]
pub struct SinceMalformed {
    /// The request's since kind.
    pub kind: SinceKind,
    /// The request's since value.
    pub requested: i64,
}

/// Why `ChangesSince` answered nothing.
#[derive(Debug, thiserror::Error)]
pub enum ChangesError {
    /// `ekr.views.SinceMalformed`, decided before any bound.
    #[error(transparent)]
    SinceMalformed(#[from] SinceMalformed),
    /// `ekr.views.LimitExceeded`.
    #[error(transparent)]
    LimitExceeded(#[from] LimitExceeded),
    /// `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound` — for `at`, or for a revision since
    /// beyond the head — or a store or revision that could not be read.
    #[error(transparent)]
    Project(#[from] ProjectError),
}

/// A bounded `ChangesSince` request: the `since`, and the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangesRequest {
    kind: SinceKind,
    since: i64,
    limit: u64,
    after: u64,
}

impl ChangesRequest {
    /// The limit when the request gives none.
    pub const DEFAULT_LIMIT: i64 = 500;
    /// The greatest admitted limit.
    pub const MAX_LIMIT: i64 = 2_000;

    /// `kind` and `since` are the `since`; `limit` is 1 to 2,000 changes a page (500 when
    /// `None`), and `after` a cursor of 0 or more (0 when `None`).
    ///
    /// # Errors
    ///
    /// [`ChangesError::SinceMalformed`] for a revision since below 0, before any bound; then
    /// [`ChangesError::LimitExceeded`] naming the first broken input of `limit` and `after`.
    pub fn new(
        kind: SinceKind,
        since: i64,
        limit: Option<i64>,
        after: Option<i64>,
    ) -> Result<Self, ChangesError> {
        if kind == SinceKind::Revision && since < 0 {
            return Err(SinceMalformed {
                kind,
                requested: since,
            }
            .into());
        }
        let limit = bounded(
            "limit",
            limit.unwrap_or(Self::DEFAULT_LIMIT),
            1,
            Some(Self::MAX_LIMIT),
        )?;
        let after = bounded("after", after.unwrap_or(0), 0, None)?;
        Ok(Self {
            kind,
            since,
            limit,
            after,
        })
    }

    /// Which of the three the `since` is.
    #[must_use]
    pub const fn kind(&self) -> SinceKind {
        self.kind
    }

    /// The `since`'s value: a revision number, or milliseconds since the epoch.
    #[must_use]
    pub const fn since(&self) -> i64 {
        self.since
    }

    /// The most changes one page holds, after the default is applied.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    /// The cursor: the position in the change sequence the page starts at.
    #[must_use]
    pub const fn after(&self) -> u64 {
        self.after
    }
}

/// `ekr.views.ChangesListed`: what one page of changes returned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangesListed {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.after`.
    pub after: u64,
    /// Entries of `changes`.
    pub changes: u64,
    /// `meta.total`.
    pub total: u64,
    /// `remaining`: the changes after this page; 0 exactly when the page has no `next`.
    pub remaining: u64,
    /// This page's NodeCreated changes.
    pub nodes_created: u64,
    /// This page's EdgeCreated changes.
    pub edges_created: u64,
    /// This page's AssertionAdded changes.
    pub assertions_added: u64,
    /// This page's AssertionSuperseded changes.
    pub assertions_superseded: u64,
    /// This page's AssertionRetracted changes.
    pub assertions_retracted: u64,
    /// This page's EvidenceAdded changes.
    pub evidence_added: u64,
    /// The revision of the page's first change; `None` for an empty page.
    pub first_revision: Option<u64>,
    /// The revision of the page's last change; `None` for an empty page.
    pub last_revision: Option<u64>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub changes_hash: String,
}

/// `ekr.views.ChangeKind`, in its declared order, which is the format's order within a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum ChangeKind {
    NodeCreated,
    EdgeCreated,
    AssertionAdded,
    AssertionSuperseded,
    AssertionRetracted,
    EvidenceAdded,
}

impl ChangeKind {
    const fn is_assertion(self) -> bool {
        matches!(
            self,
            Self::AssertionAdded | Self::AssertionSuperseded | Self::AssertionRetracted
        )
    }
}

/// `ekr.views.GraphChange`, its fields in declared order.
#[derive(Clone, Debug, Serialize)]
struct GraphChange {
    revision: u64,
    recorded_at: i64,
    change: ChangeKind,
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_id: Option<TypeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<AssertionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_hash: Option<ContentHash>,
    evidence: Vec<String>,
}

impl GraphChange {
    fn new(entry: &LoadedRevisionEntry, change: ChangeKind, id: String) -> Self {
        Self {
            revision: entry.number.get(),
            recorded_at: entry.committed_at.millis(),
            change,
            id,
            type_id: None,
            name: None,
            source: None,
            target: None,
            subject_kind: None,
            subject: None,
            by: None,
            valid_time: None,
            locator: None,
            content_hash: None,
            evidence: Vec::new(),
        }
    }

    fn about(mut self, (kind, subject): (&'static str, String)) -> Self {
        self.subject_kind = Some(kind);
        self.subject = Some(subject);
        self
    }
}

#[derive(Serialize)]
struct ChangesMeta {
    format: &'static str,
    revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    since_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since_valid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since_recorded: Option<i64>,
    limit: u64,
    after: u64,
    total: u64,
}

#[derive(Serialize)]
struct GraphChangesV1<'a> {
    meta: ChangesMeta,
    changes: &'a [GraphChange],
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<u64>,
    remaining: u64,
}

fn inconsistent(detail: String) -> ProjectError {
    ProjectError::Inconsistent(detail)
}

/// A proposed assertion's subject kind and id.
fn proposed(subject: &Subject<NodeId, EdgeId>) -> (&'static str, String) {
    match subject {
        Subject::Node(node) => ("Node", node.to_string()),
        Subject::Edge(edge) => ("Edge", edge.to_string()),
        Subject::Type(type_id) => ("Type", type_id.to_string()),
    }
}

/// A canonical assertion's subject kind and id.
fn held(subject: &Subject) -> (&'static str, String) {
    match subject {
        Subject::Node(node) => ("Node", node.id().to_string()),
        Subject::Edge(edge) => ("Edge", edge.id().to_string()),
        Subject::Type(type_id) => ("Type", type_id.to_string()),
    }
}

/// Ids as the format lists them: their text, ascending.
fn listed(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    ids.into_iter()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// The changes the seed made: everything its canonical state holds, but its evidence, which came
/// with the seed and not through an `AddEvidence`, so is no EvidenceAdded. A node's or an edge's
/// evidence is that of the seed's assertions about it.
fn seed_changes(graph: &CanonicalGraph, entry: &LoadedRevisionEntry, into: &mut Vec<GraphChange>) {
    let mut cited: BTreeMap<(&'static str, String), BTreeSet<String>> = BTreeMap::new();
    for claim in graph.assertions.values() {
        cited
            .entry(held(&claim.subject))
            .or_default()
            .extend(claim.evidence.iter().map(|held| held.id().to_string()));
    }
    let evidence =
        |key: (&'static str, String)| listed(cited.get(&key).into_iter().flatten().cloned());
    for node in graph.nodes.values() {
        let mut change = GraphChange::new(entry, ChangeKind::NodeCreated, node.id.to_string());
        change.type_id = Some(node.type_id);
        change.name = Some(node.canonical_name.clone());
        change.evidence = evidence(("Node", node.id.to_string()));
        into.push(change);
    }
    for edge in graph.edges.values() {
        let mut change = GraphChange::new(entry, ChangeKind::EdgeCreated, edge.id.to_string());
        change.type_id = Some(edge.type_id);
        change.source = Some(edge.source.id());
        change.target = Some(edge.target.id());
        change.evidence = evidence(("Edge", edge.id.to_string()));
        into.push(change);
    }
    for claim in graph.assertions.values() {
        let mut change = GraphChange::new(entry, ChangeKind::AssertionAdded, claim.id.to_string())
            .about(held(&claim.subject));
        change.valid_time = claim.valid_time.from.map(|from| from.millis());
        change.evidence = listed(claim.evidence.iter().map(|held| held.id().to_string()));
        into.push(change);
    }
}

/// The changes one committed transaction made, read off its operations. An assertion it
/// superseded or retracted is read from `graph`, the state at the read revision, which holds
/// every assertion ever added. An `AddEvidence` is an EvidenceAdded carrying the entry's source
/// identity (its locator) and the address of the payload it brought.
fn transaction_changes(
    graph: &CanonicalGraph,
    entry: &LoadedRevisionEntry,
    operations: &[GraphOperation],
    into: &mut Vec<GraphChange>,
) -> Result<(), ProjectError> {
    let mut cited: BTreeMap<(&'static str, String), BTreeSet<String>> = BTreeMap::new();
    for operation in operations {
        if let GraphOperation::AddAssertion(added) = operation {
            cited
                .entry(proposed(&added.subject))
                .or_default()
                .extend(added.evidence.iter().map(EvidenceId::to_string));
        }
    }
    let evidence =
        |key: (&'static str, String)| listed(cited.get(&key).into_iter().flatten().cloned());
    let claim_of = |id: AssertionId| {
        graph.assertions.get(&id).ok_or_else(|| {
            inconsistent(format!(
                "revision {} changes assertion {id}, which revision {} does not hold",
                entry.number, graph.revision
            ))
        })
    };
    for operation in operations {
        let change = match operation {
            GraphOperation::CreateNode(draft) => {
                let mut change =
                    GraphChange::new(entry, ChangeKind::NodeCreated, draft.id.to_string());
                change.type_id = Some(draft.type_id);
                change.name = Some(draft.canonical_name.clone());
                change.evidence = evidence(("Node", draft.id.to_string()));
                change
            }
            GraphOperation::CreateEdge(draft) => {
                let mut change =
                    GraphChange::new(entry, ChangeKind::EdgeCreated, draft.id.to_string());
                change.type_id = Some(draft.type_id);
                change.source = Some(draft.source);
                change.target = Some(draft.target);
                change.evidence = evidence(("Edge", draft.id.to_string()));
                change
            }
            GraphOperation::AddAssertion(added) => {
                let mut change =
                    GraphChange::new(entry, ChangeKind::AssertionAdded, added.id.to_string())
                        .about(proposed(&added.subject));
                change.valid_time = added.valid_time.from.map(|from| from.millis());
                change.evidence = listed(added.evidence.iter().map(EvidenceId::to_string));
                change
            }
            GraphOperation::SupersedeAssertion(supersession) => {
                let claim = claim_of(supersession.assertion)?;
                let mut change = GraphChange::new(
                    entry,
                    ChangeKind::AssertionSuperseded,
                    supersession.assertion.to_string(),
                )
                .about(held(&claim.subject));
                change.by = Some(supersession.by);
                change.valid_time = Some(supersession.effective_from.millis());
                change.evidence = listed(claim.evidence.iter().map(|e| e.id().to_string()));
                change
            }
            GraphOperation::RetractAssertion(retraction) => {
                let claim = claim_of(retraction.assertion)?;
                let mut change = GraphChange::new(
                    entry,
                    ChangeKind::AssertionRetracted,
                    retraction.assertion.to_string(),
                )
                .about(held(&claim.subject));
                change.valid_time = claim.valid_time.from.map(|from| from.millis());
                change.evidence = listed(claim.evidence.iter().map(|e| e.id().to_string()));
                change
            }
            GraphOperation::AddEvidence(addition) => {
                let added = &addition.evidence;
                let mut change =
                    GraphChange::new(entry, ChangeKind::EvidenceAdded, added.id.to_string());
                change.locator = Some(added.source.locator());
                change.content_hash = Some(added.content_hash);
                change
            }
            _ => continue,
        };
        into.push(change);
    }
    Ok(())
}

impl Index {
    /// `ChangesSince`: one page of `ekr.graph-changes/1`, the changes `request`'s `since`
    /// chooses among the revisions from the seed up to this index's revision, which is `at`.
    ///
    /// Each chosen revision's changes are read off its committed transaction — the seed's off
    /// its state — and ordered by revision, then change kind in declared order, then id; the
    /// page is the `limit` of them from position `after`. The answer names no head, so the same
    /// request answers the same bytes before and after any later commit.
    ///
    /// # Errors
    ///
    /// [`ProjectError::RevisionNotFound`] for a revision since beyond the store's head (one at or
    /// after this revision that the store holds chooses nothing); a read of the store's retained
    /// records the kernel refuses; [`ProjectError::Inconsistent`] for records that disagree with
    /// the revision list.
    pub fn changes(
        &self,
        runtime: &Runtime,
        request: &ChangesRequest,
    ) -> Result<Answer<ChangesListed>, ChangesError> {
        let loaded = &self.loaded;
        let at = self.revision();
        let since = request.since();
        let first = match request.kind() {
            SinceKind::Revision => {
                let after = since.unsigned_abs();
                if after > at.get() {
                    let head = runtime
                        .head()
                        .map_err(ProjectError::from)?
                        .ok_or(ProjectError::NotSeeded {
                            requested: Some(at),
                        })?
                        .revision;
                    if after > head.get() {
                        return Err(ProjectError::RevisionNotFound {
                            requested: RevisionNumber::new(after),
                            head,
                        }
                        .into());
                    }
                }
                after.saturating_add(1)
            }
            SinceKind::TransactionTime => loaded
                .revisions
                .iter()
                .find(|entry| entry.committed_at.millis() > since)
                .map_or(u64::MAX, |entry| entry.number.get()),
            SinceKind::ValidTime => 0,
        };
        let chosen: Vec<&LoadedRevisionEntry> = loaded
            .revisions
            .iter()
            .filter(|entry| entry.number.get() >= first && entry.number <= at)
            .collect();

        let mut changes = Vec::new();
        let records = if chosen
            .iter()
            .any(|entry| entry.number != RevisionNumber::SEED)
        {
            runtime.transactions().map_err(ProjectError::from)?
        } else {
            std::sync::Arc::default()
        };
        for entry in chosen {
            if entry.number == RevisionNumber::SEED {
                let seed = runtime
                    .replay(RevisionNumber::SEED)
                    .map_err(ProjectError::from)?;
                seed_changes(&seed, entry, &mut changes);
                continue;
            }
            let receipt = entry
                .transaction_id
                .and_then(|id| records.get(&id))
                .and_then(|record| record.committed.as_ref())
                .filter(|receipt| receipt.result.revision == entry.number)
                .ok_or_else(|| {
                    inconsistent(format!(
                        "no retained transaction committed revision {}",
                        entry.number
                    ))
                })?;
            let document =
                TransactionDocument::parse(&receipt.proposal.document_bytes).map_err(|error| {
                    inconsistent(format!(
                        "revision {}'s transaction document: {error}",
                        entry.number
                    ))
                })?;
            transaction_changes(
                &loaded.graph,
                entry,
                &document.transaction().operations,
                &mut changes,
            )?;
        }
        if request.kind() == SinceKind::ValidTime {
            changes.retain(|change| {
                change.change.is_assertion() && change.valid_time.is_some_and(|valid| valid > since)
            });
        }
        changes.sort_by(|a, b| (a.revision, a.change, &a.id).cmp(&(b.revision, b.change, &b.id)));
        changes.dedup_by(|a, b| (a.revision, a.change, &a.id) == (b.revision, b.change, &b.id));

        let total = changes.len() as u64;
        let start = usize::try_from(request.after())
            .unwrap_or(usize::MAX)
            .min(changes.len());
        let end = start
            .saturating_add(usize::try_from(request.limit()).unwrap_or(usize::MAX))
            .min(changes.len());
        let page = &changes[start..end];
        let remaining = (changes.len() - end) as u64;
        let (since_revision, since_valid, since_recorded) = match request.kind() {
            SinceKind::Revision => (Some(since), None, None),
            SinceKind::ValidTime => (None, Some(since), None),
            SinceKind::TransactionTime => (None, None, Some(since)),
        };
        let document = GraphChangesV1 {
            meta: ChangesMeta {
                format: CHANGES_FORMAT,
                revision: at.get(),
                since_revision,
                since_valid,
                since_recorded,
                limit: request.limit(),
                after: request.after(),
                total,
            },
            changes: page,
            next: (remaining > 0).then(|| request.after().saturating_add(page.len() as u64)),
            remaining,
        };
        let bytes = encode(&document)?;
        let count =
            |kind: ChangeKind| page.iter().filter(|change| change.change == kind).count() as u64;
        let summary = ChangesListed {
            revision: at.get(),
            after: request.after(),
            changes: page.len() as u64,
            total,
            remaining,
            nodes_created: count(ChangeKind::NodeCreated),
            edges_created: count(ChangeKind::EdgeCreated),
            assertions_added: count(ChangeKind::AssertionAdded),
            assertions_superseded: count(ChangeKind::AssertionSuperseded),
            assertions_retracted: count(ChangeKind::AssertionRetracted),
            evidence_added: count(ChangeKind::EvidenceAdded),
            first_revision: page.first().map(|change| change.revision),
            last_revision: page.last().map(|change| change.revision),
            changes_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }
}
