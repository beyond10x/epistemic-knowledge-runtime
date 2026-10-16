//! `ekr.node-detail/2` ([`Index::detail_v2`]): one node bounded for a reader with a budget, and
//! the ontology's type names ([`Index::type_name`]) it labels ids with
//! (ekr-mcp-fixes-20261015 #1, #10, #11; `views.yaml`, ekr.views.NodeDetailV2).
//!
//! `ekr.node-detail/1` ([`Index::describe`]) answers every assertion about a node or naming it and
//! every edge at it, so its size grows with the node's degree. `/2` answers the same node as
//!
//! - its record, with its type's name;
//! - `same_name`: up to `limit` other nodes whose canonical name or an alias equals one of its
//!   own, lowercased as the search's Folded tier lowercases;
//! - its properties, grouped by property and named: the record's values, then the Active value
//!   assertions newest first, at most `limit` per group, a String over [`VALUE_BYTES`] cut at a
//!   character boundary and marked `truncated`;
//! - its edges grouped by edge type and direction, each group named and counted, listing at most
//!   `limit` neighbours from `after`, by degree, each with the newest Active Relation assertion
//!   between the two (the id `explain` takes) and how many there are.
//!
//! With `edge_type`, only that edge type's relation groups are answered, and no property group
//! or same-name list. The cost is the node's degree and assertions, never the revision's size,
//! once the same-name lookup (built on first use) exists.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use ekr_core::{AssertionId, NodeId, PropertyId, TypeId};
use ekr_graph::{Assertion, AssertionLifecycle, CanonicalDependency, Object, Predicate, Subject};
use ekr_ontology::Ontology;
use serde::Serialize;

use crate::document::{sorted, ProjectedValue};
use crate::index::Index;
use crate::query::{bounded, encode, hash, Answer, LimitExceeded, QueryError};

/// The format literal of [`Index::detail_v2`]'s documents.
pub const DETAIL_V2_FORMAT: &str = "ekr.node-detail/2";

/// The most bytes of a String value `ekr.node-detail/2` answers whole.
pub const VALUE_BYTES: usize = 2_000;

/// `ekr.views.NodeDetailFormat`: which node-detail format a `DescribeNode` request asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DetailFormat {
    /// `ekr.node-detail/1`, [`Index::describe`]: every assertion and edge, unbounded.
    #[default]
    V1,
    /// `ekr.node-detail/2`, [`Index::detail_v2`]: bounded and grouped.
    V2,
}

impl DetailFormat {
    /// The format literal.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::V1 => crate::query::DETAIL_FORMAT,
            Self::V2 => DETAIL_V2_FORMAT,
        }
    }
}

/// A text that names no node-detail format.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is no node-detail format: ekr.node-detail/1 or ekr.node-detail/2")]
pub struct UnknownDetailFormat(pub String);

impl FromStr for DetailFormat {
    type Err = UnknownDetailFormat;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            crate::query::DETAIL_FORMAT => Ok(Self::V1),
            DETAIL_V2_FORMAT => Ok(Self::V2),
            other => Err(UnknownDetailFormat(other.to_owned())),
        }
    }
}

/// `ekr.views.TypeKind`: which kind of ontology type an id names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// A node type.
    NodeType,
    /// An edge type.
    EdgeType,
}

impl TypeKind {
    /// The variant's name, as `views.yaml` declares it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NodeType => "NodeType",
            Self::EdgeType => "EdgeType",
        }
    }
}

/// `ekr.views.EdgeDirection`: `out` when the described node is the edge's source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum EdgeDirection {
    /// The node is the target.
    #[serde(rename = "in")]
    In,
    /// The node is the source; a self-loop is `out`.
    #[serde(rename = "out")]
    Out,
}

impl EdgeDirection {
    /// The variant's name, as `views.yaml` declares it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }
}

/// An `ekr.node-detail/2` request's bounds and narrowing, built before any store is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailRequest {
    limit: u64,
    edge_type: Option<String>,
    after: u64,
}

impl DetailRequest {
    /// The members per list and group when the request names no limit.
    pub const DEFAULT_LIMIT: i64 = 10;
    /// The greatest admitted limit.
    pub const MAX_LIMIT: i64 = 500;

    /// `limit` is 1 to 500 (10 when `None`), `after` 0 or more (0 when `None`); `edge_type` is an
    /// edge type's id or name, resolved against the revision read.
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming the first broken input of `limit` and `after`.
    pub fn new(
        limit: Option<i64>,
        edge_type: Option<String>,
        after: Option<i64>,
    ) -> Result<Self, LimitExceeded> {
        let limit = bounded(
            "limit",
            limit.unwrap_or(Self::DEFAULT_LIMIT),
            1,
            Some(Self::MAX_LIMIT),
        )?;
        let after = bounded("after", after.unwrap_or(0), 0, None)?;
        Ok(Self {
            limit,
            edge_type,
            after,
        })
    }

    /// The members per list and group.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    /// The edge type the request narrows to, as it gave it.
    #[must_use]
    pub fn edge_type(&self) -> Option<&str> {
        self.edge_type.as_deref()
    }

    /// The first neighbour of each relation group to answer.
    #[must_use]
    pub const fn after(&self) -> u64 {
        self.after
    }
}

/// `ekr.views.SameNameNode`: another node sharing a name with the described one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SameNameNode {
    /// Its id.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its type's name, when the revision's ontology declares the type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Its canonical name.
    pub name: String,
    /// Its degree.
    pub degree: u64,
}

/// `ekr.views.DetailGroupCount`: what [`NodeDetailed`] counts of one relation group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailGroupCount {
    /// The group's edge type.
    pub type_id: TypeId,
    /// The group's direction.
    pub direction: EdgeDirection,
    /// The group's `total`.
    pub total: u64,
    /// The neighbours the page lists.
    pub nodes: u64,
    /// The group's `next`.
    pub next: Option<u64>,
}

/// `ekr.views.NodeDetailed`, every field a function of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDetailed {
    /// `meta.revision`.
    pub revision: u64,
    /// `node.id`.
    pub node: NodeId,
    /// `meta.limit`.
    pub limit: u64,
    /// `meta.after`.
    pub after: u64,
    /// The document's `same_name`.
    pub same_name: Vec<SameNameNode>,
    /// Entries of `properties`.
    pub properties: u64,
    /// Values the property groups list.
    pub values: u64,
    /// Those of them whose `truncated` is true.
    pub cut_values: u64,
    /// The bytes of the longest String value listed; 0 when none is.
    pub longest_value: u64,
    /// One count per relation group, in the document's order.
    pub relations: Vec<DetailGroupCount>,
    /// `meta.truncated`.
    pub truncated: bool,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub detail_hash: String,
}

/// The ontology's type names by id, and its edge types by name, read once per index.
#[derive(Debug, Default)]
pub(crate) struct Labels {
    types: HashMap<TypeId, (String, TypeKind)>,
    /// Edge types by exact name; `None` where two share it.
    edges_named: HashMap<String, Option<TypeId>>,
    /// Edge types by lowercased name; `None` where two share it.
    edges_folded: HashMap<String, Option<TypeId>>,
}

impl Labels {
    pub(crate) fn of(ontology: &Ontology) -> Self {
        let document = ontology.to_document();
        let mut labels = Self::default();
        for node_type in &document.node_types {
            labels
                .types
                .insert(node_type.id, (node_type.name.clone(), TypeKind::NodeType));
        }
        let only = |held: &mut Option<TypeId>, id: TypeId| {
            if *held != Some(id) {
                *held = None;
            }
        };
        for edge_type in &document.edge_types {
            labels
                .types
                .insert(edge_type.id, (edge_type.name.clone(), TypeKind::EdgeType));
            labels
                .edges_named
                .entry(edge_type.name.clone())
                .and_modify(|held| only(held, edge_type.id))
                .or_insert(Some(edge_type.id));
            labels
                .edges_folded
                .entry(edge_type.name.to_lowercase())
                .and_modify(|held| only(held, edge_type.id))
                .or_insert(Some(edge_type.id));
        }
        labels
    }
}

impl Index {
    fn labels(&self) -> &Labels {
        self.labels
            .get_or_init(|| Labels::of(&self.loaded.graph.ontology))
    }

    /// The name the revision's ontology gives the node type or edge type `type_id`.
    #[must_use]
    pub fn type_name(&self, type_id: TypeId) -> Option<&str> {
        self.labels()
            .types
            .get(&type_id)
            .map(|(name, _)| name.as_str())
    }

    /// The kind and name of the ontology type `type_id`, when the revision declares one.
    #[must_use]
    pub fn type_kind(&self, type_id: TypeId) -> Option<(TypeKind, &str)> {
        self.labels()
            .types
            .get(&type_id)
            .map(|(name, kind)| (*kind, name.as_str()))
    }

    /// The edge type `text` names: its id, else its name byte for byte, else the one edge type
    /// whose name equals it lowercased. `None` when it names none.
    #[must_use]
    pub fn edge_type_named(&self, text: &str) -> Option<TypeId> {
        let labels = self.labels();
        if let Ok(id) = text.parse::<TypeId>() {
            return matches!(labels.types.get(&id), Some((_, TypeKind::EdgeType))).then_some(id);
        }
        labels.edges_named.get(text).copied().flatten().or_else(|| {
            labels
                .edges_folded
                .get(&text.to_lowercase())
                .copied()
                .flatten()
        })
    }

    /// Every node by each of its lowercased names and aliases, built on first use; an empty or
    /// whitespace-only name names nothing.
    fn by_folded_name(&self) -> &HashMap<String, Vec<u32>> {
        self.folded_names.get_or_init(|| {
            let mut names: HashMap<String, Vec<u32>> = HashMap::new();
            for (at, (name, aliases)) in self.folded.iter().enumerate() {
                let at = u32::try_from(at).unwrap_or(u32::MAX);
                for folded in std::iter::once(name).chain(aliases) {
                    if folded.trim().is_empty() {
                        continue;
                    }
                    let held = names.entry(folded.clone()).or_default();
                    if held.last() != Some(&at) {
                        held.push(at);
                    }
                }
            }
            names
        })
    }

    /// Every other node sharing a lowercased name or alias with node `index`, by degree
    /// descending, then id.
    fn same_name(&self, index: u32) -> Vec<u32> {
        let names = self.by_folded_name();
        let (name, aliases) = &self.folded[index as usize];
        let mut others: Vec<u32> = std::iter::once(name)
            .chain(aliases)
            .filter_map(|folded| names.get(folded))
            .flatten()
            .copied()
            .filter(|other| *other != index)
            .collect();
        others.sort_unstable_by(|a, b| self.by_degree_then_id(*a, *b));
        others.dedup();
        others
    }

    /// Degree descending, then id: node indexes follow id order.
    fn by_degree_then_id(&self, a: u32, b: u32) -> Ordering {
        self.degree[b as usize]
            .cmp(&self.degree[a as usize])
            .then(a.cmp(&b))
    }
}

/// Newest first: valid time's start descending (none last), then recorded time's start
/// descending, then id descending.
fn newest_first(a: &&Assertion, b: &&Assertion) -> Ordering {
    let valid = |x: &Assertion| x.valid_time.from.map(|at| at.millis());
    valid(b)
        .cmp(&valid(a))
        .then(
            b.transaction_time
                .recorded_from
                .millis()
                .cmp(&a.transaction_time.recorded_from.millis()),
        )
        .then(b.id.cmp(&a.id))
}

/// `value` with a String cut to [`VALUE_BYTES`] at a character boundary, and whether it was.
fn cut(value: ProjectedValue) -> (ProjectedValue, bool) {
    match value {
        ProjectedValue::String(text) if text.len() > VALUE_BYTES => {
            let mut end = VALUE_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            (ProjectedValue::String(text[..end].to_owned()), true)
        }
        other => (other, false),
    }
}

const fn active(assertion: &Assertion) -> bool {
    matches!(assertion.lifecycle, AssertionLifecycle::Active)
}

#[derive(Serialize)]
struct DetailV2Meta<'a> {
    format: &'static str,
    revision: u64,
    limit: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_type: Option<&'a str>,
    after: u64,
    property_values: u64,
    edges: u64,
    truncated: bool,
}

#[derive(Serialize)]
struct DetailV2Node<'a> {
    id: NodeId,
    name: &'a str,
    #[serde(rename = "type")]
    type_id: TypeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<&'a str>,
    aliases: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'a str>,
    degree: u64,
}

#[derive(Serialize)]
struct DetailValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    assertion: Option<AssertionId>,
    value: ProjectedValue,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_from: Option<i64>,
    evidence: Vec<String>,
}

#[derive(Serialize)]
struct DetailPropertyGroup {
    property: PropertyId,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    total: u64,
    values: Vec<DetailValue>,
}

#[derive(Serialize)]
struct DetailNeighbour<'a> {
    id: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<&'a str>,
    name: &'a str,
    degree: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    assertion: Option<AssertionId>,
    assertions: u64,
}

#[derive(Serialize)]
struct DetailRelationGroup<'a> {
    #[serde(rename = "type")]
    type_id: TypeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    direction: EdgeDirection,
    total: u64,
    nodes: Vec<DetailNeighbour<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<u64>,
}

#[derive(Serialize)]
struct NodeDetailV2<'a> {
    meta: DetailV2Meta<'a>,
    node: DetailV2Node<'a>,
    same_name: Vec<SameNameNode>,
    properties: Vec<DetailPropertyGroup>,
    relations: Vec<DetailRelationGroup<'a>>,
}

impl Index {
    /// `DescribeNode` at `ekr.node-detail/2`: node `node` bounded by `request` (module docs).
    ///
    /// # Errors
    ///
    /// [`QueryError::NodeIsAType`] for an id no node holds and a type of the revision's ontology
    /// does; [`QueryError::NodeNotFound`] for any other id the revision holds no node with.
    pub fn detail_v2(
        &self,
        node: NodeId,
        request: &DetailRequest,
    ) -> Result<Answer<NodeDetailed>, QueryError> {
        let graph = &self.loaded.graph;
        let (Some(&index), Some(held)) = (self.node_index.get(&node), graph.nodes.get(&node))
        else {
            let as_type = node.to_string().parse::<TypeId>().ok();
            if let Some((kind, type_name)) = as_type.and_then(|id| self.type_kind(id)) {
                return Err(QueryError::NodeIsAType {
                    node,
                    kind,
                    type_name: type_name.to_owned(),
                    revision: graph.revision,
                });
            }
            return Err(QueryError::NodeNotFound {
                node,
                revision: graph.revision,
            });
        };
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);
        let after = usize::try_from(request.after).unwrap_or(usize::MAX);
        let narrowed = request.edge_type.is_some();
        let wanted = request
            .edge_type
            .as_deref()
            .map(|text| self.edge_type_named(text));
        let claims = |ids: Option<&Vec<AssertionId>>| -> Vec<&Assertion> {
            ids.into_iter()
                .flatten()
                .filter_map(|id| graph.assertions.get(id))
                .filter(|claim| active(claim))
                .collect()
        };
        let own = claims(self.about_node.get(&index));
        let incoming = claims(self.referencing.get(&index));
        let mut truncated = false;

        // Properties: the record's values, then the value assertions newest first.
        let definitions = graph.ontology.properties_of(held.type_id);
        let mut by_property: BTreeMap<PropertyId, (Vec<ProjectedValue>, Vec<&Assertion>)> =
            BTreeMap::new();
        for (property, values) in &held.properties {
            by_property.entry(*property).or_default().0 =
                values.iter().map(ProjectedValue::from).collect();
        }
        for claim in &own {
            if let (Predicate::Property(property), Object::Value(_)) =
                (&claim.predicate, &claim.object)
            {
                by_property.entry(*property).or_default().1.push(claim);
            }
        }
        let property_values = by_property
            .values()
            .map(|(record, claimed)| record.len() + claimed.len())
            .sum::<usize>() as u64;
        let mut properties = Vec::new();
        if !narrowed {
            for (property, (record, mut claimed)) in by_property {
                claimed.sort_by(newest_first);
                let total = record.len() + claimed.len();
                truncated |= total > limit;
                let recorded = record.into_iter().map(|value| (None, value));
                let asserted = claimed.iter().map(|claim| {
                    let Object::Value(value) = &claim.object else {
                        unreachable!("only value assertions are grouped")
                    };
                    (Some(*claim), ProjectedValue::from(value))
                });
                let values: Vec<DetailValue> = recorded
                    .chain(asserted)
                    .take(limit)
                    .map(|(claim, value)| {
                        let (value, was_cut) = cut(value);
                        truncated |= was_cut;
                        DetailValue {
                            assertion: claim.map(|claim| claim.id),
                            value,
                            truncated: was_cut,
                            valid_from: claim
                                .and_then(|claim| claim.valid_time.from.map(|at| at.millis())),
                            evidence: claim.map_or_else(Vec::new, |claim| {
                                sorted(claim.evidence.iter().map(CanonicalDependency::id))
                            }),
                        }
                    })
                    .collect();
                properties.push(DetailPropertyGroup {
                    property,
                    name: definitions
                        .get(&property)
                        .map(|definition| definition.name.clone()),
                    total: total as u64,
                    values,
                });
            }
        }

        // Relation assertions between this node and each other one, by type and direction.
        let mut related: HashMap<(TypeId, EdgeDirection, u32), Vec<&Assertion>> = HashMap::new();
        for claim in &own {
            if let (Predicate::Relation(type_id), Object::Node(other)) =
                (&claim.predicate, &claim.object)
            {
                if let Some(other) = self.node_index.get(&other.id()) {
                    related
                        .entry((*type_id, EdgeDirection::Out, *other))
                        .or_default()
                        .push(claim);
                }
            }
        }
        for claim in &incoming {
            if let (Predicate::Relation(type_id), Subject::Node(other)) =
                (&claim.predicate, &claim.subject)
            {
                if let Some(other) = self.node_index.get(&other.id()) {
                    related
                        .entry((*type_id, EdgeDirection::In, *other))
                        .or_default()
                        .push(claim);
                }
            }
        }

        // Edges by type and direction, one entry per neighbour.
        let adjacent = self.adjacent(index);
        let mut groups: BTreeMap<(TypeId, EdgeDirection), Vec<u32>> = BTreeMap::new();
        for (other, edge) in adjacent {
            let type_id = self.edge_type[*edge as usize];
            if wanted.is_some_and(|wanted| wanted != Some(type_id)) {
                continue;
            }
            let direction = if self.edge_source[*edge as usize] == index {
                EdgeDirection::Out
            } else {
                EdgeDirection::In
            };
            groups.entry((type_id, direction)).or_default().push(*other);
        }
        let mut relations = Vec::with_capacity(groups.len());
        for ((type_id, direction), mut others) in groups {
            others.sort_unstable_by(|a, b| self.by_degree_then_id(*a, *b));
            others.dedup();
            let total = others.len();
            let start = after.min(total);
            let end = start.saturating_add(limit).min(total);
            let next = (end < total).then_some(end as u64);
            truncated |= next.is_some();
            let nodes = others[start..end]
                .iter()
                .map(|other| {
                    let id = self.node_ids[*other as usize];
                    let neighbour = &graph.nodes[&id];
                    let mut between = related
                        .get(&(type_id, direction, *other))
                        .cloned()
                        .unwrap_or_default();
                    between.sort_by(newest_first);
                    DetailNeighbour {
                        id,
                        type_id: neighbour.type_id,
                        type_name: self.type_name(neighbour.type_id),
                        name: &neighbour.canonical_name,
                        degree: self.degree[*other as usize],
                        assertion: between.first().map(|claim| claim.id),
                        assertions: between.len() as u64,
                    }
                })
                .collect();
            relations.push(DetailRelationGroup {
                type_id,
                name: self.type_name(type_id),
                direction,
                total: total as u64,
                nodes,
                next,
            });
        }
        relations.sort_by(|a, b| {
            b.total
                .cmp(&a.total)
                .then(a.type_id.cmp(&b.type_id))
                .then(a.direction.cmp(&b.direction))
        });

        let same_name = if narrowed {
            Vec::new()
        } else {
            let others = self.same_name(index);
            truncated |= others.len() > limit;
            others
                .into_iter()
                .take(limit)
                .map(|other| {
                    let id = self.node_ids[other as usize];
                    let node = &graph.nodes[&id];
                    SameNameNode {
                        id,
                        type_id: node.type_id,
                        type_name: self.type_name(node.type_id).map(str::to_owned),
                        name: node.canonical_name.clone(),
                        degree: self.degree[other as usize],
                    }
                })
                .collect()
        };

        let document = NodeDetailV2 {
            meta: DetailV2Meta {
                format: DETAIL_V2_FORMAT,
                revision: graph.revision.get(),
                limit: request.limit,
                edge_type: request.edge_type.as_deref(),
                after: request.after,
                property_values,
                edges: adjacent.len() as u64,
                truncated,
            },
            node: DetailV2Node {
                id: held.id,
                name: &held.canonical_name,
                type_id: held.type_id,
                type_name: self.type_name(held.type_id),
                aliases: &held.aliases,
                state: held.type_state.as_deref(),
                degree: self.degree[index as usize],
            },
            same_name,
            properties,
            relations,
        };
        let bytes = encode(&document)?;
        let listed = || document.properties.iter().flat_map(|group| &group.values);
        let summary = NodeDetailed {
            revision: document.meta.revision,
            node,
            limit: request.limit,
            after: request.after,
            same_name: document.same_name.clone(),
            properties: document.properties.len() as u64,
            values: listed().count() as u64,
            cut_values: listed().filter(|value| value.truncated).count() as u64,
            longest_value: listed()
                .filter_map(|value| match &value.value {
                    ProjectedValue::String(text) => Some(text.len() as u64),
                    _ => None,
                })
                .max()
                .unwrap_or(0),
            relations: document
                .relations
                .iter()
                .map(|group| DetailGroupCount {
                    type_id: group.type_id,
                    direction: group.direction,
                    total: group.total,
                    nodes: group.nodes.len() as u64,
                    next: group.next,
                })
                .collect(),
            truncated,
            detail_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }
}
