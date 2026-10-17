//! The four bounded reads of `ekr.views` and their formats: `ekr.graph-overview/2`
//! ([`Index::overview_sections`], with `ekr.graph-overview/1` its every-section predecessor,
//! [`Index::overview`]), `ekr.graph-slice/1` ([`Index::expand`]), `ekr.node-detail/1`
//! ([`Index::describe`]) and `ekr.node-matches/1` ([`Index::search`]); and the named successors
//! `ekr.graph-overview/3` ([`Index::overview_as`]) and `ekr.graph-slice/2`
//! ([`Index::expand_as`], [`Index::page_as`]), which name each type they count or carry
//! (`names.rs`).
//!
//! A request is bounded when it is built — [`OverviewRequest::new`], [`ExpandRequest::new`],
//! [`SearchRequest::new`] — so a broken bound is refused before any store is read, naming the
//! first broken input in the command's declared order (`views.yaml`, "Bounds and the order of
//! refusals"). A store that is not seeded, or holds no such revision, is refused by
//! [`Index::load`]; a node the revision does not hold, by the read itself.
//!
//! Every document type here is one of `views.yaml`'s, its fields in the order the specification
//! declares them, as `document.rs` holds `ekr.graph-projection/1` to rules (2) to (5): serde
//! writes a struct's fields in declaration order, an absent `Option` is skipped and a present one
//! written, and every list and map is always written but a lineage version's `widened` and
//! `modified`, which `views.yaml` omits when empty. Each array is built in the order the
//! format's own paragraph of rule (1) names.

use std::collections::BTreeSet;

use ekr_core::{AssertionId, EdgeId, NodeId, PropertyId, RevisionNumber, TypeId};
use ekr_graph::{Assertion, Object, Subject};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::document::{self, ProjectedAssertion, ProjectedEdge, ProjectedOntology, ProjectedValue};
use crate::index::Index;
use crate::names::{OverviewFormat, SliceFormat};
use crate::ProjectError;

/// The format literal of [`Index::overview`]'s documents.
pub const OVERVIEW_FORMAT: &str = "ekr.graph-overview/1";
/// The format literal of [`Index::overview_sections`]'s documents.
pub const OVERVIEW_V2_FORMAT: &str = "ekr.graph-overview/2";
/// The format literal of [`Index::expand`]'s documents.
pub const SLICE_FORMAT: &str = "ekr.graph-slice/1";
/// The format literal of [`Index::describe`]'s documents.
pub const DETAIL_FORMAT: &str = "ekr.node-detail/1";
/// The format literal of [`Index::search`]'s documents.
pub const MATCHES_FORMAT: &str = "ekr.node-matches/1";

// ---- requests, bounds and refusals ----------------------------------------------------------------

/// `ekr.views.LimitExceeded`: an input outside its bound. The store was not read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{parameter} is {requested}, and it must be at least {minimum}{}", maximum.map(|m| format!(" and at most {m}")).unwrap_or_default())]
pub struct LimitExceeded {
    /// The input's name, as the command declares it.
    pub parameter: &'static str,
    /// The value the request gave it.
    pub requested: i64,
    /// Its least admitted value.
    pub minimum: i64,
    /// Its greatest admitted value; `None` for an input with no upper bound (`after`).
    pub maximum: Option<i64>,
}

/// Why a bounded read answered nothing.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// `ekr.views.LimitExceeded`.
    #[error(transparent)]
    LimitExceeded(#[from] LimitExceeded),
    /// `ekr.views.NodeNotFound`: the revision holds no node with the requested id — for an
    /// expansion, the first seed in the order the request gave them that it does not hold.
    #[error("revision {revision} holds no node {node}")]
    NodeNotFound {
        /// The node asked for.
        node: NodeId,
        /// The revision that was read.
        revision: RevisionNumber,
    },
    /// `ekr.views.NodeIsAType` (ekr-mcp-fixes-20261015 #10): an `ekr.node-detail/2` request whose
    /// id no node of the revision holds and a node type or edge type of its ontology does.
    #[error(
        "{node} is the type {type_name}, not a node: overview lists the types; search (with type) \
         lists its nodes"
    )]
    NodeIsAType {
        /// The id asked for.
        node: NodeId,
        /// Which kind of type it names.
        kind: crate::detail::TypeKind,
        /// The name the revision's ontology gives that type.
        type_name: String,
        /// The revision that was read.
        revision: RevisionNumber,
    },
    /// `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`, or a store or revision that could
    /// not be read.
    #[error(transparent)]
    Project(#[from] ProjectError),
}

pub(crate) fn bounded(
    parameter: &'static str,
    requested: i64,
    minimum: i64,
    maximum: Option<i64>,
) -> Result<u64, LimitExceeded> {
    if requested < minimum || maximum.is_some_and(|maximum| requested > maximum) {
        return Err(LimitExceeded {
            parameter,
            requested,
            minimum,
            maximum,
        });
    }
    Ok(requested.unsigned_abs())
}

/// `ekr.views.OverviewSection`: an optional section of `ekr.graph-overview/2`, named by the key
/// it is answered under. Variants are in the order the specification declares them, which is
/// the order [`OverviewRequest::sections`] and [`GraphOverviewed::sections`] list them in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OverviewSection {
    /// `ontology`: the projected ontology.
    Ontology,
    /// `schema`: the schema lineage and the revisions.
    Schema,
    /// `roles`: each node type's roles in valid time.
    Roles,
    /// `timeline`: dated assertions per bucket.
    Timeline,
}

impl OverviewSection {
    /// Every section, in declared order.
    pub const ALL: [Self; 4] = [Self::Ontology, Self::Schema, Self::Roles, Self::Timeline];

    /// The section's name: its key in the document and its wire value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ontology => "ontology",
            Self::Schema => "schema",
            Self::Roles => "roles",
            Self::Timeline => "timeline",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

impl std::fmt::Display for OverviewSection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A text that names no [`OverviewSection`]: only the four names, exactly, are sections.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is no overview section; the sections are ontology, schema, roles and timeline")]
pub struct UnknownOverviewSection(pub String);

impl std::str::FromStr for OverviewSection {
    type Err = UnknownOverviewSection;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|section| section.name() == text)
            .ok_or_else(|| UnknownOverviewSection(text.to_owned()))
    }
}

/// A bounded `ProjectOverview` request: how many of the highest-degree nodes to list, and which
/// optional sections `ekr.graph-overview/2` carries — none unless [`Self::with_sections`] names
/// them. [`Index::overview`]'s `ekr.graph-overview/1` carries every section whatever is named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverviewRequest {
    limit: u64,
    sections: u8,
}

impl OverviewRequest {
    /// The limit when the request gives none.
    pub const DEFAULT_LIMIT: i64 = 300;
    /// The greatest admitted limit.
    pub const MAX_LIMIT: i64 = 500;

    /// `limit` is 1 to 500; `None` is 300.
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming `limit`.
    pub fn new(limit: Option<i64>) -> Result<Self, LimitExceeded> {
        let limit = bounded(
            "limit",
            limit.unwrap_or(Self::DEFAULT_LIMIT),
            1,
            Some(Self::MAX_LIMIT),
        )?;
        Ok(Self { limit, sections: 0 })
    }

    /// This request with `sections` named as well. A section named twice is named once, and the
    /// order they are named in does not change the answer; none named is the request's default.
    #[must_use]
    pub fn with_sections(mut self, sections: impl IntoIterator<Item = OverviewSection>) -> Self {
        for section in sections {
            self.sections |= section.bit();
        }
        self
    }

    /// The limit, after the default is applied.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    /// The sections named, in declared order; empty when none is.
    #[must_use]
    pub fn sections(&self) -> Vec<OverviewSection> {
        OverviewSection::ALL
            .into_iter()
            .filter(|section| self.names(*section))
            .collect()
    }

    const fn names(&self, section: OverviewSection) -> bool {
        self.sections & section.bit() != 0
    }
}

/// A bounded `ExpandNeighbourhood` request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandRequest {
    seeds: Vec<NodeId>,
    depth: u64,
    limit: u64,
    edge_limit: u64,
    after: u64,
}

impl ExpandRequest {
    /// The greatest admitted depth.
    pub const MAX_DEPTH: i64 = 2;
    /// The greatest admitted node limit per page.
    pub const MAX_LIMIT: i64 = 2_000;
    /// The greatest admitted edge limit per page, and the edge limit when the request gives none.
    pub const MAX_EDGE_LIMIT: i64 = 5_000;

    /// `seeds` is a set whose order names only which unknown seed is refused; `depth` is 0 to
    /// 2, `limit` 1 to 2,000 nodes a page, `edge_limit` 1 to 5,000 edges a page (5,000 when
    /// `None`), and `after` a cursor of 0 or more (0 when `None`).
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming the first broken input of `depth`, `limit`, `edge_limit` and
    /// `after`.
    pub fn new(
        seeds: Vec<NodeId>,
        depth: i64,
        limit: i64,
        edge_limit: Option<i64>,
        after: Option<i64>,
    ) -> Result<Self, LimitExceeded> {
        let depth = bounded("depth", depth, 0, Some(Self::MAX_DEPTH))?;
        let limit = bounded("limit", limit, 1, Some(Self::MAX_LIMIT))?;
        let edge_limit = bounded(
            "edge_limit",
            edge_limit.unwrap_or(Self::MAX_EDGE_LIMIT),
            1,
            Some(Self::MAX_EDGE_LIMIT),
        )?;
        let after = bounded("after", after.unwrap_or(0), 0, None)?;
        Ok(Self {
            seeds,
            depth,
            limit,
            edge_limit,
            after,
        })
    }

    /// The seeds, as the request gave them.
    #[must_use]
    pub fn seeds(&self) -> &[NodeId] {
        &self.seeds
    }

    /// The hop count.
    #[must_use]
    pub const fn depth(&self) -> u64 {
        self.depth
    }

    /// The most nodes one page holds.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    /// The most edges one page holds, after the default is applied.
    #[must_use]
    pub const fn edge_limit(&self) -> u64 {
        self.edge_limit
    }

    /// The cursor: the position in the record sequence the page starts at.
    #[must_use]
    pub const fn after(&self) -> u64 {
        self.after
    }
}

/// A bounded `SearchNodes` request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequest {
    text: String,
    limit: u64,
}

impl SearchRequest {
    /// The greatest admitted limit.
    pub const MAX_LIMIT: i64 = 100;

    /// `text` has no length bound; `limit` is 1 to 100.
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming `limit`.
    pub fn new(text: String, limit: i64) -> Result<Self, LimitExceeded> {
        let limit = bounded("limit", limit, 1, Some(Self::MAX_LIMIT))?;
        Ok(Self { text, limit })
    }

    /// The text searched for.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The most matches answered.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }
}

// ---- answers and their events -------------------------------------------------------------------

/// One answer: the exact document bytes and the event counted over them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer<S> {
    /// UTF-8 JSON with no insignificant whitespace.
    pub bytes: Vec<u8>,
    /// The event, every field a function of `bytes`.
    pub summary: S,
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub(crate) fn encode(document: &impl Serialize) -> Result<Vec<u8>, ProjectError> {
    serde_json::to_vec(document)
        .map_err(|error| ProjectError::Inconsistent(format!("encoding the document: {error}")))
}

/// `ekr.views.GraphOverviewed`. A field drawn from a section the document does not carry is `None`:
/// the schema counts without `schema`, `event_types` and `observation_type` without `roles`, the
/// timeline counts without `timeline`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphOverviewed {
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
    /// Entries of `node_types`.
    pub node_types: u64,
    /// Entries of `edge_types`.
    pub edge_types: u64,
    /// Entries of `schema.versions`.
    pub schema_versions: Option<u64>,
    /// Entries of `schema.revisions`.
    pub revisions: Option<u64>,
    /// Members of every listed version's `added`.
    pub added: Option<u64>,
    /// Members of every listed version's `removed`.
    pub removed: Option<u64>,
    /// Entries of every listed version's `widened`.
    pub widened: Option<u64>,
    /// Entries of every listed version's `modified`.
    pub modified: Option<u64>,
    /// `schema.unrecorded_nodes`.
    pub unrecorded_nodes: Option<u64>,
    /// `schema.unrecorded_edges`.
    pub unrecorded_edges: Option<u64>,
    /// Revision 0's `nodes`.
    pub revision_zero_nodes: Option<u64>,
    /// Revision 0's `edges`.
    pub revision_zero_edges: Option<u64>,
    /// Revision 0's `assertions`.
    pub revision_zero_assertions: Option<u64>,
    /// Entries of `roles.types` whose `event` is true.
    pub event_types: Option<u64>,
    /// `roles.observation_type`.
    pub observation_type: Option<TypeId>,
    /// `timeline.bucket_ms`.
    pub bucket_ms: Option<u64>,
    /// Entries of `timeline.buckets`.
    pub timeline_buckets: Option<u64>,
    /// The sum of the buckets' `assertions`.
    pub dated_assertions: Option<u64>,
    /// `timeline.undated`.
    pub undated_assertions: Option<u64>,
    /// Entries of `top`.
    pub top: u64,
    /// The optional sections the document carries, in declared order.
    pub sections: Vec<OverviewSection>,
    /// `meta.format`: [`OVERVIEW_V2_FORMAT`], [`OVERVIEW_FORMAT`] or
    /// [`crate::OVERVIEW_V3_FORMAT`].
    pub format: &'static str,
    /// `ekr.graph-overview/3`'s `meta.committed_at`; `None` for /1 and /2.
    pub committed_at: Option<i64>,
    /// `ekr.graph-overview/3`'s `name` of each `node_types` entry, in order; `None` for /1 and
    /// /2.
    pub node_type_names: Option<Vec<String>>,
    /// `ekr.graph-overview/3`'s `name` of each `edge_types` entry, in order; `None` for /1 and
    /// /2.
    pub edge_type_names: Option<Vec<String>>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub overview_hash: String,
}

/// `ekr.views.NeighbourhoodExpanded`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NeighbourhoodExpanded {
    /// `meta.revision`.
    pub revision: u64,
    /// Entries of `meta.seeds`.
    pub seeds: u64,
    /// `meta.depth`.
    pub depth: u64,
    /// `meta.after`.
    pub after: u64,
    /// Node records on this page.
    pub nodes: u64,
    /// Edge records on this page.
    pub edges: u64,
    /// `meta.node_total`.
    pub node_total: u64,
    /// `meta.edge_total`.
    pub edge_total: u64,
    /// Records of the sequence after this page; 0 exactly when the page has no `next`.
    pub remaining: u64,
    /// `meta.format`: [`SLICE_FORMAT`] or [`crate::SLICE_V2_FORMAT`].
    pub format: &'static str,
    /// Under `ekr.graph-slice/2`, every distinct `type_name` of the page's records, ascending by
    /// its UTF-8 bytes; `None` under /1.
    pub type_names: Option<Vec<String>>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub slice_hash: String,
}

/// `ekr.views.NodeDescribed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDescribed {
    /// `meta.revision`.
    pub revision: u64,
    /// `node.id`.
    pub node: NodeId,
    /// Entries of `assertions`.
    pub assertions: u64,
    /// Entries of `referencing`.
    pub referencing: u64,
    /// Entries of `edges`.
    pub edges: u64,
    /// Entries of `neighbours`.
    pub neighbours: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub detail_hash: String,
}

/// `ekr.views.NodesSearched`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodesSearched {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.text`.
    pub text: String,
    /// Entries of `matches`.
    pub matches: u64,
    /// `meta.total`.
    pub total: u64,
    /// `meta.exact_total`.
    pub exact_total: u64,
    /// `meta.format`: [`MATCHES_FORMAT`], [`crate::MATCHES_V2_FORMAT`] or
    /// [`crate::MATCHES_V3_FORMAT`].
    pub format: &'static str,
    /// The first match's id; `None` when there is none.
    pub first_match: Option<NodeId>,
    /// The first match's tier (`Exact`, `Folded` or `Words`); `None` when there is none, and
    /// under `ekr.node-matches/3`, which has no tier.
    pub first_tier: Option<&'static str>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub matches_hash: String,
    /// `meta.whole_total` under `ekr.node-matches/3`; `None` under /1 and /2.
    pub whole_total: Option<u64>,
    /// The first match's kind (`WholeName`, `WholeAlias`, `Prefix`, `Substring` or `Words`)
    /// under `ekr.node-matches/3`; `None` when there is none, and under /1 and /2.
    pub first_kind: Option<&'static str>,
    /// Whether the first match is case-folded under `ekr.node-matches/3`; `None` when there is
    /// none, and under /1 and /2.
    pub first_folded: Option<bool>,
}

// ---- light records ------------------------------------------------------------------------------

/// `ekr.views.NodeSummary`: what a viewer needs to draw a node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NodeSummary {
    /// The node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its canonical name.
    pub name: String,
    /// The revision's edges with it as source or target, self-loops not counted.
    pub degree: u64,
}

/// `ekr.views.SliceNode`: a node of a neighbourhood, with its hop distance from the nearest seed;
/// with `type_name`, `ekr.views.SliceNodeV2`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SliceNode {
    /// The node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its type's name, in an `ekr.graph-slice/2` page; `None`, and not written, in /1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Its canonical name.
    pub name: String,
    /// Its degree over the whole revision.
    pub degree: u64,
    /// Hops from the nearest seed; seeds are at 0.
    pub distance: u64,
}

/// `ekr.views.SliceEdge`: an edge of a neighbourhood, without its props; with `type_name`,
/// `ekr.views.SliceEdgeV2`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SliceEdge {
    /// The edge.
    pub id: EdgeId,
    /// Its source.
    pub source: NodeId,
    /// Its target.
    pub target: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its type's name, in an `ekr.graph-slice/2` page; `None`, and not written, in /1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// The assertions whose subject is this edge, by id.
    pub assertions: Vec<AssertionId>,
}

/// One record of a neighbourhood's record sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SliceRecord {
    /// A node.
    Node(SliceNode),
    /// An edge, after both its endpoints.
    Edge(SliceEdge),
}

/// `ekr.views.SliceMeta`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SliceMeta {
    /// [`SLICE_FORMAT`], or [`crate::SLICE_V2_FORMAT`] for a page whose records carry their
    /// `type_name`.
    pub format: &'static str,
    /// The revision read.
    pub revision: u64,
    /// The request's seeds as a set, by id.
    pub seeds: Vec<NodeId>,
    /// The request's depth.
    pub depth: u64,
    /// The request's cursor, 0 when it gave none.
    pub after: u64,
    /// Nodes of the whole neighbourhood.
    pub node_total: u64,
    /// Edges of the whole neighbourhood.
    pub edge_total: u64,
}

/// One page of a neighbourhood, before it is encoded: what a host streams record by record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlicePage {
    meta: SliceMeta,
    records: Vec<SliceRecord>,
    next: Option<u64>,
    remaining: u64,
}

impl SlicePage {
    /// The page's meta.
    #[must_use]
    pub const fn meta(&self) -> &SliceMeta {
        &self.meta
    }

    /// The page's records, in record-sequence order.
    #[must_use]
    pub fn records(&self) -> &[SliceRecord] {
        &self.records
    }

    /// The cursor of the next page; `None` exactly when no record remains after this one.
    #[must_use]
    pub const fn next(&self) -> Option<u64> {
        self.next
    }

    /// Records of the sequence after this page.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// The page as the format its meta names: `ekr.graph-slice/1`, or `ekr.graph-slice/2`, whose
    /// records carry their `type_name`, for a page [`Index::page_as`] built in /2.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] if the document cannot be encoded.
    pub fn render(&self) -> Result<Answer<NeighbourhoodExpanded>, ProjectError> {
        #[derive(Serialize)]
        struct GraphSliceV1<'a> {
            meta: &'a SliceMeta,
            nodes: Vec<&'a SliceNode>,
            edges: Vec<&'a SliceEdge>,
            #[serde(skip_serializing_if = "Option::is_none")]
            next: Option<u64>,
        }
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for record in &self.records {
            match record {
                SliceRecord::Node(node) => nodes.push(node),
                SliceRecord::Edge(edge) => edges.push(edge),
            }
        }
        let (node_count, edge_count) = (nodes.len() as u64, edges.len() as u64);
        let type_names = (self.meta.format != SLICE_FORMAT).then(|| {
            let named: BTreeSet<&str> = nodes
                .iter()
                .filter_map(|node| node.type_name.as_deref())
                .chain(edges.iter().filter_map(|edge| edge.type_name.as_deref()))
                .collect();
            named.into_iter().map(str::to_owned).collect()
        });
        let bytes = encode(&GraphSliceV1 {
            meta: &self.meta,
            nodes,
            edges,
            next: self.next,
        })?;
        let summary = NeighbourhoodExpanded {
            revision: self.meta.revision,
            seeds: self.meta.seeds.len() as u64,
            depth: self.meta.depth,
            after: self.meta.after,
            nodes: node_count,
            edges: edge_count,
            node_total: self.meta.node_total,
            edge_total: self.meta.edge_total,
            remaining: self.remaining,
            format: self.meta.format,
            type_names,
            slice_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }
}

// ---- ekr.graph-overview/1's sections, built once per revision by the index ----------------------

#[derive(Serialize)]
struct OverviewMeta {
    format: &'static str,
    revision: u64,
    /// `ekr.graph-overview/3`'s only (`ekr.views.OverviewMetaV3`).
    #[serde(skip_serializing_if = "Option::is_none")]
    committed_at: Option<i64>,
    node_count: u64,
    edge_count: u64,
    assertion_count: u64,
    evidence_count: u64,
    limit: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SchemaMember {
    pub(crate) id: String,
    pub(crate) kind: &'static str,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OverviewSchemaVersion {
    pub(crate) number: u64,
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent: Option<String>,
    pub(crate) revision: u64,
    pub(crate) added: Vec<SchemaMember>,
    pub(crate) removed: Vec<SchemaMember>,
    /// Present only when not empty (`views.yaml`, `ekr.views.OverviewSchemaVersion`), so a
    /// version with none answers the bytes it answered before the field existed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) widened: Vec<WidenedEnd>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) modified: Vec<ModifiedProperty>,
}

/// `ekr.views.WidenedEnd`.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct WidenedEnd {
    pub(crate) edge_type: TypeId,
    pub(crate) side: &'static str,
    pub(crate) node_types: Vec<TypeId>,
}

/// `ekr.views.ModifiedProperty`; `changed` holds `ekr.views.PropertyAspect` variants in their
/// declared order.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct ModifiedProperty {
    pub(crate) owner: TypeId,
    pub(crate) property: PropertyId,
    pub(crate) name: String,
    pub(crate) changed: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OverviewRevision {
    pub(crate) number: u64,
    pub(crate) committed_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) transaction_id: Option<String>,
    pub(crate) schema_version: String,
    pub(crate) nodes: u64,
    pub(crate) edges: u64,
    pub(crate) assertions: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OverviewSchema {
    pub(crate) versions: Vec<OverviewSchemaVersion>,
    pub(crate) revisions: Vec<OverviewRevision>,
    pub(crate) unrecorded_nodes: u64,
    pub(crate) unrecorded_edges: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TypeCount {
    #[serde(rename = "type")]
    pub(crate) type_id: TypeId,
    pub(crate) count: u64,
}

/// `ekr.views.NamedTypeCount`: a type count of `ekr.graph-overview/3`, named.
#[derive(Clone, Debug, Serialize)]
struct NamedTypeCount<'a> {
    #[serde(rename = "type")]
    type_id: TypeId,
    name: &'a str,
    count: u64,
}

/// An overview's node_types or edge_types: /1's and /2's type counts, or /3's named ones.
#[derive(Serialize)]
#[serde(untagged)]
enum TypeCounts<'a> {
    Counted(&'a [TypeCount]),
    Named(Vec<NamedTypeCount<'a>>),
}

impl TypeCounts<'_> {
    fn names(&self) -> Option<Vec<String>> {
        match self {
            Self::Counted(_) => None,
            Self::Named(named) => Some(named.iter().map(|count| count.name.to_owned()).collect()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TypeTiming {
    #[serde(rename = "type")]
    pub(crate) type_id: TypeId,
    pub(crate) nodes: u64,
    pub(crate) timestamped: u64,
    pub(crate) judged: u64,
    pub(crate) within_hour: u64,
    pub(crate) instant: u64,
    pub(crate) neighbour_types: u64,
    pub(crate) event: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OverviewRoles {
    pub(crate) types: Vec<TypeTiming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) observation_type: Option<TypeId>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TimelineBucket {
    pub(crate) start: i64,
    #[serde(rename = "type")]
    pub(crate) type_id: TypeId,
    pub(crate) assertions: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OverviewTimeline {
    pub(crate) bucket_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i64>,
    pub(crate) undated: u64,
    pub(crate) buckets: Vec<TimelineBucket>,
}

/// Every section of the overview that does not depend on the request: built once per revision.
/// A section the revision cannot be overviewed with keeps its refusal, so the other three reads
/// still answer.
pub(crate) struct OverviewParts {
    pub(crate) ontology: Result<ProjectedOntology, String>,
    pub(crate) schema: Result<OverviewSchema, String>,
    pub(crate) node_types: Vec<TypeCount>,
    pub(crate) edge_types: Vec<TypeCount>,
    pub(crate) roles: OverviewRoles,
    pub(crate) timeline: OverviewTimeline,
}

// ---- ekr.node-detail/1 and ekr.node-matches/1 ---------------------------------------------------

#[derive(Serialize)]
struct DetailNode<'a> {
    id: NodeId,
    name: &'a str,
    #[serde(rename = "type")]
    type_id: TypeId,
    aliases: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'a str>,
    props: std::collections::BTreeMap<String, Vec<ProjectedValue>>,
    degree: u64,
}

#[derive(Serialize)]
struct ReferencingAssertion {
    subject_kind: &'static str,
    subject: String,
    assertion: ProjectedAssertion,
}

#[derive(Serialize)]
struct DetailMeta {
    format: &'static str,
    revision: u64,
}

#[derive(Serialize)]
struct NodeDetailV1<'a> {
    meta: DetailMeta,
    node: DetailNode<'a>,
    assertions: Vec<ProjectedAssertion>,
    referencing: Vec<ReferencingAssertion>,
    edges: Vec<ProjectedEdge>,
    neighbours: Vec<NodeSummary>,
}

/// Which comparison a match passed, in the order the tiers rank (`views.yaml`,
/// `ekr.views.MatchTier`). `ekr.node-matches/1` answers `Exact` and `Folded` only; `Words` is
/// `ekr.node-matches/2`'s ([`Index::search_v2`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) enum MatchTier {
    Exact,
    Folded,
    Words,
}

impl MatchTier {
    /// The tier's name, as the documents write it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Exact => "Exact",
            Self::Folded => "Folded",
            Self::Words => "Words",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum MatchField {
    Name,
    Alias,
}

#[derive(Serialize)]
struct NodeMatch<'a> {
    id: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    name: &'a str,
    degree: u64,
    tier: MatchTier,
    field: MatchField,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<&'a str>,
}

#[derive(Serialize)]
struct MatchesMeta<'a> {
    format: &'static str,
    revision: u64,
    text: &'a str,
    total: u64,
    exact_total: u64,
}

#[derive(Serialize)]
struct NodeMatchesV1<'a> {
    meta: MatchesMeta<'a>,
    matches: Vec<NodeMatch<'a>>,
}

const fn subject_kind(subject: &Subject) -> &'static str {
    match subject {
        Subject::Node(_) => "Node",
        Subject::Edge(_) => "Edge",
        Subject::Type(_) => "Type",
    }
}

fn subject_id(subject: &Subject) -> String {
    match subject {
        Subject::Node(node) => node.id().to_string(),
        Subject::Edge(edge) => edge.id().to_string(),
        Subject::Type(type_id) => type_id.to_string(),
    }
}

// ---- the four reads -----------------------------------------------------------------------------

impl Index {
    /// `ekr.graph-overview/1`: this revision's every section, whatever `request` names but its
    /// limit. It is [`Self::overview_sections`] with all four sections named, under its own
    /// format name: every section but `top` was built with the index; `top` is the first `limit`
    /// of the nodes the index keeps in degree order, so the answer costs the document's size.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] for a schema lineage the format cannot carry — the refusal
    /// `ekr.graph-projection/1` makes of the same revision. Every ontology the kernel admits is
    /// carried, including one in which two types define one property id differently.
    pub fn overview(
        &self,
        request: &OverviewRequest,
    ) -> Result<Answer<GraphOverviewed>, ProjectError> {
        self.render_overview(
            OVERVIEW_FORMAT,
            request.limit(),
            &request.with_sections(OverviewSection::ALL),
        )
    }

    /// `ProjectOverview`: this revision's `ekr.graph-overview/2`, carrying meta, node_types,
    /// edge_types and top, and of ontology, schema, roles and timeline only the ones `request`
    /// names. A section not named is absent, never empty, and is not encoded: the answer named
    /// none costs meta, one entry per declared type and the `top` nodes, whatever the ontology's
    /// properties, the lineage's versions or the revisions committed.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] for a named `ontology` or `schema` the format cannot carry,
    /// as [`Self::overview`] refuses it; a request that names neither is answered.
    pub fn overview_sections(
        &self,
        request: &OverviewRequest,
    ) -> Result<Answer<GraphOverviewed>, ProjectError> {
        self.render_overview(OVERVIEW_V2_FORMAT, request.limit(), request)
    }

    /// `ProjectOverview` in the format `format` names: [`Self::overview_sections`]'s
    /// `ekr.graph-overview/2`, or `ekr.graph-overview/3` — the same sections, each node_types and
    /// edge_types entry with its type's `name`, and meta.committed_at, the time the revision read
    /// was committed (`views.yaml`, `ekr.views.GraphOverviewV3`; ekr-mcp-fixes-20261015 #3,
    /// #13). /3 costs /2's answer and one name per declared type.
    ///
    /// # Errors
    ///
    /// What [`Self::overview_sections`] refuses.
    pub fn overview_as(
        &self,
        format: OverviewFormat,
        request: &OverviewRequest,
    ) -> Result<Answer<GraphOverviewed>, ProjectError> {
        self.render_overview(format.name(), request.limit(), request)
    }

    fn render_overview(
        &self,
        format: &'static str,
        limit: u64,
        request: &OverviewRequest,
    ) -> Result<Answer<GraphOverviewed>, ProjectError> {
        #[derive(Serialize)]
        struct GraphOverview<'a> {
            meta: OverviewMeta,
            #[serde(skip_serializing_if = "Option::is_none")]
            ontology: Option<&'a ProjectedOntology>,
            #[serde(skip_serializing_if = "Option::is_none")]
            schema: Option<&'a OverviewSchema>,
            node_types: TypeCounts<'a>,
            edge_types: TypeCounts<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            roles: Option<&'a OverviewRoles>,
            #[serde(skip_serializing_if = "Option::is_none")]
            timeline: Option<&'a OverviewTimeline>,
            top: Vec<NodeSummary>,
        }
        let parts = &self.overview;
        let named = format == OverviewFormat::V3.name();
        let (node_types, edge_types, committed_at) = if named {
            let committed_at = self
                .loaded
                .revisions
                .iter()
                .rev()
                .find(|entry| entry.number == self.loaded.graph.revision)
                .map(|entry| entry.committed_at.millis())
                .ok_or_else(|| {
                    ProjectError::Inconsistent(format!(
                        "revision {} is not in its own revision list",
                        self.loaded.graph.revision
                    ))
                })?;
            (
                TypeCounts::Named(self.named_counts(&parts.node_types)?),
                TypeCounts::Named(self.named_counts(&parts.edge_types)?),
                Some(committed_at),
            )
        } else {
            (
                TypeCounts::Counted(&parts.node_types),
                TypeCounts::Counted(&parts.edge_types),
                None,
            )
        };
        let ontology = if request.names(OverviewSection::Ontology) {
            Some(
                parts
                    .ontology
                    .as_ref()
                    .map_err(|error| ProjectError::Inconsistent(error.clone()))?,
            )
        } else {
            None
        };
        let schema = if request.names(OverviewSection::Schema) {
            Some(
                parts
                    .schema
                    .as_ref()
                    .map_err(|error| ProjectError::Inconsistent(error.clone()))?,
            )
        } else {
            None
        };
        let roles = request
            .names(OverviewSection::Roles)
            .then_some(&parts.roles);
        let timeline = request
            .names(OverviewSection::Timeline)
            .then_some(&parts.timeline);
        let graph = &self.loaded.graph;
        let top: Vec<NodeSummary> = self
            .by_degree
            .iter()
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|node| self.summary(*node))
            .collect();
        let document = GraphOverview {
            meta: OverviewMeta {
                format,
                revision: graph.revision.get(),
                committed_at,
                node_count: graph.nodes.len() as u64,
                edge_count: graph.edges.len() as u64,
                assertion_count: graph.assertions.len() as u64,
                evidence_count: graph.evidence.len() as u64,
                limit,
            },
            ontology,
            schema,
            node_types,
            edge_types,
            roles,
            timeline,
            top,
        };
        let bytes = encode(&document)?;
        let versions = |count: fn(&OverviewSchemaVersion) -> usize| {
            schema.map(|schema| schema.versions.iter().map(|v| count(v) as u64).sum())
        };
        let revision_zero = |count: fn(&OverviewRevision) -> u64| {
            schema.map(|schema| schema.revisions.first().map_or(0, count))
        };
        let summary = GraphOverviewed {
            revision: document.meta.revision,
            nodes: document.meta.node_count,
            edges: document.meta.edge_count,
            assertions: document.meta.assertion_count,
            evidence: document.meta.evidence_count,
            node_types: parts.node_types.len() as u64,
            edge_types: parts.edge_types.len() as u64,
            schema_versions: schema.map(|schema| schema.versions.len() as u64),
            revisions: schema.map(|schema| schema.revisions.len() as u64),
            added: versions(|v| v.added.len()),
            removed: versions(|v| v.removed.len()),
            widened: versions(|v| v.widened.len()),
            modified: versions(|v| v.modified.len()),
            unrecorded_nodes: schema.map(|schema| schema.unrecorded_nodes),
            unrecorded_edges: schema.map(|schema| schema.unrecorded_edges),
            revision_zero_nodes: revision_zero(|entry| entry.nodes),
            revision_zero_edges: revision_zero(|entry| entry.edges),
            revision_zero_assertions: revision_zero(|entry| entry.assertions),
            event_types: roles.map(|roles| roles.types.iter().filter(|t| t.event).count() as u64),
            observation_type: roles.and_then(|roles| roles.observation_type),
            bucket_ms: timeline.map(|timeline| timeline.bucket_ms),
            timeline_buckets: timeline.map(|timeline| timeline.buckets.len() as u64),
            dated_assertions: timeline
                .map(|timeline| timeline.buckets.iter().map(|b| b.assertions).sum()),
            undated_assertions: timeline.map(|timeline| timeline.undated),
            top: document.top.len() as u64,
            sections: request.sections(),
            format,
            committed_at,
            node_type_names: document.node_types.names(),
            edge_type_names: document.edge_types.names(),
            overview_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }

    /// `ExpandNeighbourhood`: one page of the neighbourhood's record sequence, unencoded.
    ///
    /// The neighbourhood is every node within `depth` hops of a seed across the revision's edges
    /// in either direction, and every edge with both ends in it. Its records are its nodes by
    /// distance ascending, then degree descending, then id, each followed by the edges it closes
    /// by id. The cost is the neighbourhood's size; only the page's records are built.
    ///
    /// # Errors
    ///
    /// [`QueryError::NodeNotFound`] for the first seed, in the request's order, the revision
    /// does not hold.
    pub fn page(&self, request: &ExpandRequest) -> Result<SlicePage, QueryError> {
        self.page_as(SliceFormat::V1, request)
    }

    /// [`Self::page`] in the format `format` names: in `ekr.graph-slice/2` each record carries
    /// its type's name and the meta names /2 (`views.yaml`, `ekr.views.GraphSliceV2`;
    /// ekr-mcp-fixes-20261015 #3). The same records, in the same order, as /1.
    ///
    /// # Errors
    ///
    /// What [`Self::page`] refuses.
    pub fn page_as(
        &self,
        format: SliceFormat,
        request: &ExpandRequest,
    ) -> Result<SlicePage, QueryError> {
        let named = format.is_named();
        let graph = &self.loaded.graph;
        let mut seeds = Vec::with_capacity(request.seeds().len());
        for seed in request.seeds() {
            let Some(node) = self.node_index.get(seed) else {
                return Err(QueryError::NodeNotFound {
                    node: *seed,
                    revision: graph.revision,
                });
            };
            seeds.push(*node);
        }
        seeds.sort_unstable();
        seeds.dedup();

        let mut distance: std::collections::HashMap<u32, u64> =
            seeds.iter().map(|seed| (*seed, 0)).collect();
        let mut frontier = seeds.clone();
        for hop in 1..=request.depth() {
            let mut reached = Vec::new();
            for node in &frontier {
                for (other, _) in self.adjacent(*node) {
                    if let std::collections::hash_map::Entry::Vacant(entry) = distance.entry(*other)
                    {
                        entry.insert(hop);
                        reached.push(*other);
                    }
                }
            }
            frontier = reached;
        }
        let mut order: Vec<u32> = distance.keys().copied().collect();
        order.sort_unstable_by(|a, b| {
            distance[a]
                .cmp(&distance[b])
                .then(self.degree[*b as usize].cmp(&self.degree[*a as usize]))
                .then(a.cmp(b))
        });
        let position: std::collections::HashMap<u32, usize> = order
            .iter()
            .enumerate()
            .map(|(at, node)| (*node, at))
            .collect();

        let (after, limit, edge_limit) = (request.after(), request.limit(), request.edge_limit());
        let mut at = 0_u64;
        let mut records = Vec::new();
        let (mut nodes, mut edges, mut full) = (0_u64, 0_u64, false);
        for (place, node) in order.iter().enumerate() {
            if at >= after && !full {
                if nodes < limit {
                    records.push(SliceRecord::Node(self.slice_node(
                        *node,
                        distance[node],
                        named,
                    )?));
                    nodes += 1;
                } else {
                    full = true;
                }
            }
            at += 1;
            for (other, edge) in self.adjacent(*node) {
                // The edges this node closes: its other end is this node, or comes earlier.
                if !position
                    .get(other)
                    .is_some_and(|there| *there < place || other == node)
                {
                    continue;
                }
                if at >= after && !full {
                    if edges < edge_limit {
                        records.push(SliceRecord::Edge(self.slice_edge(*edge, named)?));
                        edges += 1;
                    } else {
                        full = true;
                    }
                }
                at += 1;
            }
        }
        let total = at;
        let node_total = order.len() as u64;
        let end = after.saturating_add(records.len() as u64);
        Ok(SlicePage {
            meta: SliceMeta {
                format: format.name(),
                revision: graph.revision.get(),
                seeds: seeds
                    .iter()
                    .map(|seed| self.node_ids[*seed as usize])
                    .collect(),
                depth: request.depth(),
                after,
                node_total,
                edge_total: total - node_total,
            },
            records,
            next: (end < total).then_some(end),
            remaining: total.saturating_sub(end),
        })
    }

    /// `ExpandNeighbourhood`: [`Index::page`] encoded as `ekr.graph-slice/1`.
    ///
    /// # Errors
    ///
    /// Whatever [`Index::page`] refuses.
    pub fn expand(
        &self,
        request: &ExpandRequest,
    ) -> Result<Answer<NeighbourhoodExpanded>, QueryError> {
        Ok(self.page(request)?.render()?)
    }

    /// `ExpandNeighbourhood` in the format `format` names: [`Self::page_as`] encoded as
    /// `ekr.graph-slice/1` or `ekr.graph-slice/2`.
    ///
    /// # Errors
    ///
    /// Whatever [`Index::page`] refuses.
    pub fn expand_as(
        &self,
        format: SliceFormat,
        request: &ExpandRequest,
    ) -> Result<Answer<NeighbourhoodExpanded>, QueryError> {
        Ok(self.page_as(format, request)?.render()?)
    }

    /// `DescribeNode`: one node's `ekr.node-detail/1` — its record, every assertion about it,
    /// every assertion whose object is it, every edge at it, and every node those name.
    ///
    /// # Errors
    ///
    /// [`QueryError::NodeNotFound`] for a node the revision does not hold.
    pub fn describe(&self, node: NodeId) -> Result<Answer<NodeDescribed>, QueryError> {
        let graph = &self.loaded.graph;
        let (Some(index), Some(held)) = (self.node_index.get(&node), graph.nodes.get(&node)) else {
            return Err(QueryError::NodeNotFound {
                node,
                revision: graph.revision,
            });
        };
        let index = *index;
        let claims = |ids: Option<&Vec<AssertionId>>| -> Vec<&Assertion> {
            ids.into_iter()
                .flatten()
                .filter_map(|id| graph.assertions.get(id))
                .collect()
        };
        let own = claims(self.about_node.get(&index));
        let incoming = claims(self.referencing.get(&index));

        let mut neighbours: BTreeSet<u32> = BTreeSet::new();
        let mut incident: Vec<u32> = Vec::new();
        for (other, edge) in self.adjacent(index) {
            incident.push(*edge);
            neighbours.insert(*other);
        }
        incident.sort_unstable();
        incident.dedup();
        for claim in &own {
            if let Object::Node(target) = &claim.object {
                neighbours.extend(self.node_index.get(&target.id()));
            }
        }
        for claim in &incoming {
            if let Subject::Node(source) = &claim.subject {
                neighbours.extend(self.node_index.get(&source.id()));
            }
        }
        neighbours.remove(&index);

        let mut edges = Vec::with_capacity(incident.len());
        for edge in incident {
            let id = self.edge_ids[edge as usize];
            let held_edge = graph.edges.get(&id).ok_or_else(|| {
                ProjectError::Inconsistent(format!("the index lists edge {id} the graph lacks"))
            })?;
            let about = claims(self.about_edge.get(&edge));
            edges.push(ProjectedEdge::of(held_edge, Some(&about)));
        }
        let document = NodeDetailV1 {
            meta: DetailMeta {
                format: DETAIL_FORMAT,
                revision: graph.revision.get(),
            },
            node: DetailNode {
                id: held.id,
                name: &held.canonical_name,
                type_id: held.type_id,
                aliases: &held.aliases,
                state: held.type_state.as_deref(),
                props: document::props(&held.properties),
                degree: self.degree[index as usize],
            },
            assertions: document::listed_under(Some(&own)),
            referencing: incoming
                .iter()
                .map(|claim| ReferencingAssertion {
                    subject_kind: subject_kind(&claim.subject),
                    subject: subject_id(&claim.subject),
                    assertion: document::assertion(claim),
                })
                .collect(),
            edges,
            neighbours: neighbours
                .iter()
                .map(|other| self.summary(*other))
                .collect(),
        };
        let bytes = encode(&document)?;
        let summary = NodeDescribed {
            revision: document.meta.revision,
            node,
            assertions: document.assertions.len() as u64,
            referencing: document.referencing.len() as u64,
            edges: document.edges.len() as u64,
            neighbours: document.neighbours.len() as u64,
            detail_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }

    /// `SearchNodes`: the first `limit` nodes whose canonical name or an alias contains the
    /// text — byte-exactly, else after lowercasing both sides — Exact before Folded, then degree
    /// descending, then id; never by a name. Every name is compared, against the lowercased
    /// forms the index keeps, so the cost is the revision's name bytes.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] if the document cannot be encoded.
    pub fn search(&self, request: &SearchRequest) -> Result<Answer<NodesSearched>, ProjectError> {
        let graph = &self.loaded.graph;
        let text = request.text();
        let folded = text.to_lowercase();
        let mut found: Vec<(MatchTier, u32, MatchField, Option<&str>)> = Vec::new();
        for (index, (node, (name, aliases))) in graph.nodes.values().zip(&self.folded).enumerate() {
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            let hit = if node.canonical_name.contains(text) {
                Some((MatchTier::Exact, MatchField::Name, None))
            } else if let Some(alias) = node.aliases.iter().find(|alias| alias.contains(text)) {
                Some((MatchTier::Exact, MatchField::Alias, Some(alias.as_str())))
            } else if name.contains(&folded) {
                Some((MatchTier::Folded, MatchField::Name, None))
            } else {
                aliases
                    .iter()
                    .position(|alias| alias.contains(&folded))
                    .map(|at| {
                        (
                            MatchTier::Folded,
                            MatchField::Alias,
                            Some(node.aliases[at].as_str()),
                        )
                    })
            };
            if let Some((tier, field, alias)) = hit {
                found.push((tier, index, field, alias));
            }
        }
        found.sort_unstable_by(|a, b| {
            a.0.cmp(&b.0)
                .then(self.degree[b.1 as usize].cmp(&self.degree[a.1 as usize]))
                .then(a.1.cmp(&b.1))
        });
        let total = found.len() as u64;
        let exact_total = found.iter().filter(|hit| hit.0 == MatchTier::Exact).count() as u64;
        let matches: Vec<NodeMatch<'_>> = found
            .iter()
            .take(usize::try_from(request.limit()).unwrap_or(usize::MAX))
            .map(|(tier, index, field, alias)| {
                let id = self.node_ids[*index as usize];
                let node = &graph.nodes[&id];
                NodeMatch {
                    id,
                    type_id: node.type_id,
                    name: &node.canonical_name,
                    degree: self.degree[*index as usize],
                    tier: *tier,
                    field: *field,
                    alias: *alias,
                }
            })
            .collect();
        let first_match = matches.first().map(|hit| hit.id);
        let first_tier = matches.first().map(|hit| hit.tier.name());
        let document = NodeMatchesV1 {
            meta: MatchesMeta {
                format: MATCHES_FORMAT,
                revision: graph.revision.get(),
                text,
                total,
                exact_total,
            },
            matches,
        };
        let bytes = encode(&document)?;
        let summary = NodesSearched {
            revision: document.meta.revision,
            text: text.to_owned(),
            matches: document.matches.len() as u64,
            total,
            exact_total,
            format: MATCHES_FORMAT,
            first_match,
            first_tier,
            matches_hash: hash(&bytes),
            whole_total: None,
            first_kind: None,
            first_folded: None,
        };
        Ok(Answer { bytes, summary })
    }

    fn summary(&self, node: u32) -> NodeSummary {
        let id = self.node_ids[node as usize];
        let held = &self.loaded.graph.nodes[&id];
        NodeSummary {
            id,
            type_id: held.type_id,
            name: held.canonical_name.clone(),
            degree: self.degree[node as usize],
        }
    }

    fn slice_node(&self, node: u32, distance: u64, named: bool) -> Result<SliceNode, ProjectError> {
        let summary = self.summary(node);
        let type_name = named
            .then(|| self.declared_type_name(summary.type_id).map(str::to_owned))
            .transpose()?;
        Ok(SliceNode {
            id: summary.id,
            type_id: summary.type_id,
            type_name,
            name: summary.name,
            degree: summary.degree,
            distance,
        })
    }

    fn slice_edge(&self, edge: u32, named: bool) -> Result<SliceEdge, ProjectError> {
        let id = self.edge_ids[edge as usize];
        let held = &self.loaded.graph.edges[&id];
        let type_name = named
            .then(|| self.declared_type_name(held.type_id).map(str::to_owned))
            .transpose()?;
        Ok(SliceEdge {
            id,
            source: held.source.id(),
            target: held.target.id(),
            type_id: held.type_id,
            type_name,
            assertions: self.about_edge.get(&edge).cloned().unwrap_or_default(),
        })
    }

    /// The name the revision's ontology declares for the node or edge type `type_id`. Every
    /// node's and edge's type is declared in the ontology of the revision that holds it.
    fn declared_type_name(&self, type_id: TypeId) -> Result<&str, ProjectError> {
        let ontology = &self.loaded.graph.ontology;
        ontology
            .node_type(type_id)
            .map(|declared| declared.name.as_str())
            .or_else(|| {
                ontology
                    .edge_type(type_id)
                    .map(|declared| declared.name.as_str())
            })
            .ok_or_else(|| {
                ProjectError::Inconsistent(format!(
                    "revision {} holds the type {type_id}, which its ontology does not declare",
                    self.loaded.graph.revision
                ))
            })
    }

    /// `counts`, each named by its type (`ekr.views.NamedTypeCount`).
    fn named_counts<'a>(
        &'a self,
        counts: &'a [TypeCount],
    ) -> Result<Vec<NamedTypeCount<'a>>, ProjectError> {
        counts
            .iter()
            .map(|count| {
                Ok(NamedTypeCount {
                    type_id: count.type_id,
                    name: self.declared_type_name(count.type_id)?,
                    count: count.count,
                })
            })
            .collect()
    }
}
