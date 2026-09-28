//! The per-revision index the five bounded reads answer from, and the cache a host keeps it in.
//!
//! [`Index::build`] reads one [`LoadedRevision`] — the value [`crate::load`] returns — once:
//! adjacency and degree, the assertions by subject and by object, the lowercased names and
//! aliases, the nodes in degree order, and every section of `ekr.graph-overview/1` that does not
//! depend on the request (type counts, growth per revision from recorded time, the schema
//! lineage, the valid-time roles and the timeline buckets). After that each read costs its
//! answer, not the revision. It owns the loaded revision, so a host renders
//! `ekr.graph-projection/1` from the same load through [`Index::loaded`].
//!
//! [`IndexCache`] keeps the most recently used indexes by revision and head, so only the first
//! read of a revision pays for its load.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::{Arc, OnceLock};

use ekr_core::{AssertionId, EdgeId, NodeId, RevisionNumber, TypeId};
use ekr_graph::{CanonicalValue, Object, Predicate, Subject};
use ekr_kernel::Runtime;
use ekr_ontology::Ontology;

use crate::document;
use crate::query::{
    OverviewParts, OverviewRevision, OverviewRoles, OverviewSchema, OverviewSchemaVersion,
    OverviewTimeline, SchemaMember, TimelineBucket, TypeCount, TypeTiming,
};
use crate::{LoadedRevision, ProjectError};

/// One committed revision, indexed for the bounded reads.
pub struct Index {
    pub(crate) loaded: LoadedRevision,
    /// Every node's id, ascending: a node's position here is its index everywhere else.
    pub(crate) node_ids: Vec<NodeId>,
    pub(crate) node_index: HashMap<NodeId, u32>,
    /// Edges with the node as source or target, self-loops not counted.
    pub(crate) degree: Vec<u64>,
    /// `adjacency[offsets[n]..offsets[n + 1]]` is node n's `(other end, edge)` pairs in edge id
    /// order; a self-loop appears once.
    offsets: Vec<usize>,
    adjacency: Vec<(u32, u32)>,
    /// Every edge's id, ascending: an edge's position here is its index.
    pub(crate) edge_ids: Vec<EdgeId>,
    /// Assertion ids by subject node, by subject edge and by object node, each ascending.
    pub(crate) about_node: HashMap<u32, Vec<AssertionId>>,
    pub(crate) about_edge: HashMap<u32, Vec<AssertionId>>,
    pub(crate) referencing: HashMap<u32, Vec<AssertionId>>,
    /// Node indexes by degree descending, then id.
    pub(crate) by_degree: Vec<u32>,
    /// Each node's canonical name and aliases, lowercased, in node order.
    pub(crate) folded: Vec<(String, Vec<String>)>,
    pub(crate) overview: OverviewParts,
    /// Each node's type, in node order.
    pub(crate) node_type: Vec<TypeId>,
    /// The nodes of each type, in node order.
    pub(crate) by_type: HashMap<TypeId, Vec<u32>>,
    /// Whether each node is of an event type of the overview's roles, in node order.
    pub(crate) node_event: Vec<bool>,
    /// Each node's time and end, the timeline's (`views.yaml`, ekr.graph-timeline/1), in node
    /// order; `None` for a node with no time.
    pub(crate) node_time: Vec<Option<(i64, i64)>>,
    /// Each edge's source node and type, in edge order.
    pub(crate) edge_source: Vec<u32>,
    pub(crate) edge_type: Vec<TypeId>,
    /// The timeline's row types, ranked on first use.
    pub(crate) row_types: OnceLock<Vec<RowTypeRank>>,
}

/// One candidate row type of the timeline, with what ranks it (`ekr.views.TimelineRowType`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowTypeRank {
    pub(crate) type_id: TypeId,
    pub(crate) nodes: u64,
    pub(crate) reached: u64,
    pub(crate) weight: u64,
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Index")
            .field("revision", &self.revision())
            .field("head", &self.head())
            .field("nodes", &self.node_ids.len())
            .field("edges", &self.edge_ids.len())
            .finish_non_exhaustive()
    }
}

fn inconsistent(detail: String) -> ProjectError {
    ProjectError::Inconsistent(detail)
}

fn index_of(count: usize) -> Result<u32, ProjectError> {
    u32::try_from(count).map_err(|_| inconsistent(format!("{count} records exceed the index")))
}

impl Index {
    /// [`crate::load`]s revision `at` of `runtime`'s store (its head when `None`) and indexes it.
    ///
    /// # Errors
    ///
    /// Whatever [`crate::load`] or [`Index::build`] refuses.
    pub fn load(runtime: &Runtime, at: Option<RevisionNumber>) -> Result<Self, ProjectError> {
        Self::build(crate::load(runtime, at)?)
    }

    /// Indexes `loaded`. Pure: the same revision builds the same index.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] for an edge whose end, or an assertion whose subject, the
    /// revision does not hold, which a revision the kernel admitted never has.
    pub fn build(loaded: LoadedRevision) -> Result<Self, ProjectError> {
        let graph = &loaded.graph;
        let node_ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
        let mut node_index = HashMap::with_capacity(node_ids.len());
        for (at, id) in node_ids.iter().enumerate() {
            node_index.insert(*id, index_of(at)?);
        }
        let held = |id: NodeId, what: &str| {
            node_index.get(&id).copied().ok_or_else(|| {
                inconsistent(format!(
                    "{what} names node {id}, which revision {} does not hold",
                    graph.revision
                ))
            })
        };

        let mut edge_ids = Vec::with_capacity(graph.edges.len());
        let mut edge_type = Vec::with_capacity(graph.edges.len());
        let mut ends = Vec::with_capacity(graph.edges.len());
        let mut degree = vec![0_u64; node_ids.len()];
        let mut counts = vec![0_usize; node_ids.len() + 1];
        for edge in graph.edges.values() {
            let what = format!("edge {}", edge.id);
            let (source, target) = (
                held(edge.source.id(), &what)?,
                held(edge.target.id(), &what)?,
            );
            edge_ids.push(edge.id);
            edge_type.push(edge.type_id);
            ends.push((source, target));
            counts[source as usize] += 1;
            if source != target {
                counts[target as usize] += 1;
                degree[source as usize] += 1;
                degree[target as usize] += 1;
            }
        }
        let mut offsets = Vec::with_capacity(counts.len());
        let mut running = 0;
        for count in &counts {
            offsets.push(running);
            running += count;
        }
        let mut fill = offsets.clone();
        let mut adjacency = vec![(0_u32, 0_u32); running];
        for (edge, (source, target)) in ends.iter().enumerate() {
            let edge = index_of(edge)?;
            adjacency[fill[*source as usize]] = (*target, edge);
            fill[*source as usize] += 1;
            if source != target {
                adjacency[fill[*target as usize]] = (*source, edge);
                fill[*target as usize] += 1;
            }
        }
        let edge_index: HashMap<EdgeId, u32> = edge_ids
            .iter()
            .enumerate()
            .map(|(at, id)| index_of(at).map(|at| (*id, at)))
            .collect::<Result<_, _>>()?;

        let mut about_node: HashMap<u32, Vec<AssertionId>> = HashMap::new();
        let mut about_edge: HashMap<u32, Vec<AssertionId>> = HashMap::new();
        let mut referencing: HashMap<u32, Vec<AssertionId>> = HashMap::new();
        for claim in graph.assertions.values() {
            match &claim.subject {
                Subject::Node(node) => about_node
                    .entry(held(node.id(), &format!("assertion {}", claim.id))?)
                    .or_default()
                    .push(claim.id),
                Subject::Edge(edge) => {
                    let at = edge_index.get(&edge.id()).ok_or_else(|| {
                        inconsistent(format!(
                            "assertion {} is about edge {}, which revision {} does not hold",
                            claim.id,
                            edge.id(),
                            graph.revision
                        ))
                    })?;
                    about_edge.entry(*at).or_default().push(claim.id);
                }
                Subject::Type(_) => {}
            }
            if let Object::Node(target) = &claim.object {
                if let Some(at) = node_index.get(&target.id()) {
                    referencing.entry(*at).or_default().push(claim.id);
                }
            }
        }

        let mut by_degree: Vec<u32> = (0..index_of(node_ids.len())?).collect();
        by_degree
            .sort_unstable_by(|a, b| degree[*b as usize].cmp(&degree[*a as usize]).then(a.cmp(b)));
        let folded = graph
            .nodes
            .values()
            .map(|node| {
                (
                    node.canonical_name.to_lowercase(),
                    node.aliases
                        .iter()
                        .map(|alias| alias.to_lowercase())
                        .collect(),
                )
            })
            .collect();
        let node_type: Vec<TypeId> = graph.nodes.values().map(|node| node.type_id).collect();
        let mut by_type: HashMap<TypeId, Vec<u32>> = HashMap::new();
        for (at, type_id) in node_type.iter().enumerate() {
            by_type.entry(*type_id).or_default().push(index_of(at)?);
        }
        let edge_source = ends.iter().map(|(source, _)| *source).collect();

        let mut index = Self {
            node_ids,
            node_index,
            degree,
            offsets,
            adjacency,
            edge_ids,
            about_node,
            about_edge,
            referencing,
            by_degree,
            folded,
            node_type,
            by_type,
            node_event: Vec::new(),
            node_time: Vec::new(),
            edge_source,
            edge_type,
            row_types: OnceLock::new(),
            overview: OverviewParts {
                ontology: Err(String::new()),
                schema: Err(String::new()),
                node_types: Vec::new(),
                edge_types: Vec::new(),
                roles: OverviewRoles {
                    types: Vec::new(),
                    observation_type: None,
                },
                timeline: OverviewTimeline {
                    bucket_ms: DAY_MS as u64,
                    first: None,
                    last: None,
                    undated: 0,
                    buckets: Vec::new(),
                },
            },
            loaded,
        };
        let (overview, node_time) = index.overview_parts();
        let events: HashSet<TypeId> = overview
            .roles
            .types
            .iter()
            .filter(|timing| timing.event)
            .map(|timing| timing.type_id)
            .collect();
        index.node_event = index
            .node_type
            .iter()
            .map(|type_id| events.contains(type_id))
            .collect();
        index.node_time = node_time;
        index.overview = overview;
        Ok(index)
    }

    /// The revision indexed.
    #[must_use]
    pub fn revision(&self) -> RevisionNumber {
        self.loaded.graph.revision
    }

    /// The store's head when the revision was loaded.
    #[must_use]
    pub const fn head(&self) -> RevisionNumber {
        self.loaded.head
    }

    /// The loaded revision the index was built from.
    #[must_use]
    pub const fn loaded(&self) -> &LoadedRevision {
        &self.loaded
    }

    /// Node `node`'s `(other end, edge)` pairs, in edge id order.
    pub(crate) fn adjacent(&self, node: u32) -> &[(u32, u32)] {
        let node = node as usize;
        &self.adjacency[self.offsets[node]..self.offsets[node + 1]]
    }

    /// The overview's request-free sections, and each node's timeline time and end.
    fn overview_parts(&self) -> (OverviewParts, Vec<Option<(i64, i64)>>) {
        let graph = &self.loaded.graph;
        let by_type: BTreeMap<TypeId, Vec<&ekr_graph::Assertion>> =
            graph
                .assertions
                .values()
                .fold(BTreeMap::new(), |mut by_type, claim| {
                    if let Subject::Type(type_id) = claim.subject {
                        by_type.entry(type_id).or_insert_with(Vec::new).push(claim);
                    }
                    by_type
                });
        let ontology =
            document::project_ontology(&graph.ontology, &by_type).map_err(|e| e.to_string());

        let declared = graph.ontology.to_document();
        let mut node_types: Vec<TypeId> = declared.node_types.iter().map(|t| t.id).collect();
        node_types.sort_unstable();
        let mut edge_types: Vec<TypeId> = declared.edge_types.iter().map(|t| t.id).collect();
        edge_types.sort_unstable();
        let mut node_counts: HashMap<TypeId, u64> = HashMap::new();
        for node in graph.nodes.values() {
            *node_counts.entry(node.type_id).or_default() += 1;
        }
        let mut edge_counts: HashMap<TypeId, u64> = HashMap::new();
        for edge in graph.edges.values() {
            *edge_counts.entry(edge.type_id).or_default() += 1;
        }
        let count = |types: &[TypeId], counts: &HashMap<TypeId, u64>| -> Vec<TypeCount> {
            types
                .iter()
                .map(|type_id| TypeCount {
                    type_id: *type_id,
                    count: counts.get(type_id).copied().unwrap_or(0),
                })
                .collect()
        };

        let (roles, times) = self.roles(&node_types);
        (
            OverviewParts {
                ontology,
                schema: self.schema(),
                node_types: count(&node_types, &node_counts),
                edge_types: count(&edge_types, &edge_counts),
                roles,
                timeline: self.overview_timeline(),
            },
            times,
        )
    }

    /// `ekr.views.OverviewSchema`: the lineage with what each version added and removed, and the
    /// revisions with what the overviewed state says existed by each one's commit.
    fn schema(&self) -> Result<OverviewSchema, String> {
        let loaded = &self.loaded;
        let graph = &loaded.graph;
        let mut versions = Vec::new();
        for (first, schema) in loaded.schemas.values() {
            if *first > graph.revision {
                continue;
            }
            let version = schema.version();
            let declared = members(schema);
            let inherited = match version.parent {
                None => BTreeMap::new(),
                Some(parent) => {
                    let (_, parent) = loaded.schemas.get(&parent).ok_or_else(|| {
                        format!(
                            "schema version {} names parent {parent}, which no listed revision is \
                             valid against",
                            version.id
                        )
                    })?;
                    members(parent)
                }
            };
            let difference =
                |from: &BTreeMap<String, (&'static str, String)>,
                 without: &BTreeMap<String, (&'static str, String)>| {
                    from.iter()
                        .filter(|(id, _)| !without.contains_key(*id))
                        .map(|(id, (kind, name))| SchemaMember {
                            id: id.clone(),
                            kind,
                            name: name.clone(),
                        })
                        .collect()
                };
            versions.push(OverviewSchemaVersion {
                number: version.number,
                id: version.id.to_string(),
                parent: version.parent.map(|parent| parent.to_string()),
                revision: first.get(),
                added: difference(&declared, &inherited),
                removed: difference(&inherited, &declared),
            });
        }
        versions.sort_by_key(|version| version.number);

        // Recorded times: an assertion's own; a node's, the least of the assertions about it; an
        // edge's, the least of the assertions about it and of the Relation assertions matching
        // its source, type and target.
        let mut assertion_times = Vec::with_capacity(graph.assertions.len());
        let mut node_times: HashMap<NodeId, i64> = HashMap::new();
        let mut edge_times: HashMap<EdgeId, i64> = HashMap::new();
        let mut relation_times: HashMap<(NodeId, TypeId, NodeId), i64> = HashMap::new();
        let earliest = |map_time: &mut i64, at: i64| *map_time = (*map_time).min(at);
        for claim in graph.assertions.values() {
            let at = claim.transaction_time.recorded_from.millis();
            assertion_times.push(at);
            match &claim.subject {
                Subject::Node(node) => {
                    earliest(node_times.entry(node.id()).or_insert(at), at);
                    if let (Predicate::Relation(type_id), Object::Node(target)) =
                        (&claim.predicate, &claim.object)
                    {
                        earliest(
                            relation_times
                                .entry((node.id(), *type_id, target.id()))
                                .or_insert(at),
                            at,
                        );
                    }
                }
                Subject::Edge(edge) => earliest(edge_times.entry(edge.id()).or_insert(at), at),
                Subject::Type(_) => {}
            }
        }
        let mut recorded_nodes: Vec<i64> = graph
            .nodes
            .keys()
            .filter_map(|id| node_times.get(id).copied())
            .collect();
        let mut recorded_edges: Vec<i64> = graph
            .edges
            .values()
            .filter_map(|edge| {
                let own = edge_times.get(&edge.id).copied();
                let matched = relation_times
                    .get(&(edge.source.id(), edge.type_id, edge.target.id()))
                    .copied();
                match (own, matched) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                }
            })
            .collect();
        assertion_times.sort_unstable();
        recorded_nodes.sort_unstable();
        recorded_edges.sort_unstable();
        let unrecorded_nodes = (graph.nodes.len() - recorded_nodes.len()) as u64;
        let unrecorded_edges = (graph.edges.len() - recorded_edges.len()) as u64;
        let by = |times: &[i64], at: i64| times.partition_point(|time| *time <= at) as u64;
        let revisions = loaded
            .revisions
            .iter()
            .filter(|entry| entry.number <= graph.revision)
            .map(|entry| {
                let at = entry.committed_at.millis();
                OverviewRevision {
                    number: entry.number.get(),
                    committed_at: at,
                    transaction_id: entry.transaction_id.map(|id| id.to_string()),
                    schema_version: entry.schema_version.to_string(),
                    nodes: unrecorded_nodes + by(&recorded_nodes, at),
                    edges: unrecorded_edges + by(&recorded_edges, at),
                    assertions: by(&assertion_times, at),
                }
            })
            .collect();
        Ok(OverviewSchema {
            versions,
            revisions,
            unrecorded_nodes,
            unrecorded_edges,
        })
    }

    /// `ekr.views.OverviewRoles`, by the reference viewer's valid-time rule (`views.yaml`,
    /// `ekr.views.TypeTiming`), and each node's time and end as the timeline places it: its least
    /// timestamp-like property value, else its earliest dated fact; its end the greatest of that
    /// and its dated facts.
    fn roles(&self, node_types: &[TypeId]) -> (OverviewRoles, Vec<Option<(i64, i64)>>) {
        let graph = &self.loaded.graph;
        let count = self.node_ids.len();
        // Each node's dated facts: how many, the earliest and the latest.
        let mut dated: Vec<(u64, i64, i64)> = vec![(0, i64::MAX, i64::MIN); count];
        // Node pairs a Relation assertion joins, subject to object.
        let mut related: Vec<(u32, u32)> = Vec::new();
        for claim in graph.assertions.values() {
            let subject = match &claim.subject {
                Subject::Node(node) => self.node_index.get(&node.id()).copied(),
                _ => None,
            };
            let object = match (&claim.predicate, &claim.object) {
                (Predicate::Relation(_), Object::Node(node)) => {
                    self.node_index.get(&node.id()).copied()
                }
                _ => None,
            };
            if let (Some(subject), Some(object)) = (subject, object) {
                related.push((subject, object));
            }
            if let Some(from) = claim.valid_time.from.map(|at| at.millis()) {
                let mut mark = |node: u32| {
                    let entry = &mut dated[node as usize];
                    *entry = (entry.0 + 1, entry.1.min(from), entry.2.max(from));
                };
                subject.into_iter().for_each(&mut mark);
                if object.is_some() && object != subject {
                    object.into_iter().for_each(&mut mark);
                }
            }
        }
        let mut partners: HashMap<u32, Vec<u32>> = HashMap::new();
        for (subject, object) in related {
            partners.entry(subject).or_default().push(object);
            partners.entry(object).or_default().push(subject);
        }

        let position: HashMap<TypeId, usize> = node_types
            .iter()
            .enumerate()
            .map(|(at, type_id)| (*type_id, at))
            .collect();
        let mut kinds: Vec<Option<usize>> = Vec::with_capacity(count);
        let mut timings: Vec<TypeTiming> = node_types
            .iter()
            .map(|type_id| TypeTiming {
                type_id: *type_id,
                nodes: 0,
                timestamped: 0,
                judged: 0,
                within_hour: 0,
                instant: 0,
                neighbour_types: 0,
                event: false,
            })
            .collect();
        for node in graph.nodes.values() {
            kinds.push(position.get(&node.type_id).copied());
        }
        let mut seen = vec![u32::MAX; node_types.len()];
        let mut times: Vec<Option<(i64, i64)>> = Vec::with_capacity(count);
        for (at, node) in graph.nodes.values().enumerate() {
            let stamp = node
                .properties
                .values()
                .flatten()
                .filter_map(|value| match value {
                    CanonicalValue::Integer(v)
                        if (1_000_000_000_000..10_000_000_000_000).contains(v) =>
                    {
                        Some(*v)
                    }
                    CanonicalValue::Timestamp(instant) => Some(instant.millis()),
                    _ => None,
                })
                .min();
            let (facts, earliest, latest) = dated[at];
            let time = stamp.or((facts > 0).then_some(earliest));
            times.push(time.map(|time| (time, if facts > 0 { time.max(latest) } else { time })));
            let Some(kind) = kinds[at] else { continue };
            let here = u32::try_from(at).unwrap_or(u32::MAX);
            let timing = &mut timings[kind];
            timing.nodes += 1;
            let timestamped = stamp.is_some();
            if timestamped {
                timing.timestamped += 1;
                timing.judged += 1;
                timing.instant += 1;
            } else if facts >= 2 {
                timing.judged += 1;
                if i128::from(latest) - i128::from(earliest) <= HOUR_MS {
                    timing.within_hour += 1;
                    timing.instant += 1;
                }
            }
            let neighbours = self
                .adjacent(here)
                .iter()
                .map(|(other, _)| *other)
                .chain(partners.get(&here).into_iter().flatten().copied());
            for other in neighbours {
                if let Some(other_kind) = kinds[other as usize] {
                    if seen[other_kind] != here {
                        seen[other_kind] = here;
                        timing.neighbour_types += 1;
                    }
                }
            }
        }
        for timing in &mut timings {
            timing.event = timing.judged > 0 && 5 * timing.instant >= 3 * timing.judged;
        }

        // The mean of neighbour_types / nodes over the event types, exactly: sum / events.
        let events: Vec<&TypeTiming> = timings.iter().filter(|timing| timing.event).collect();
        let mut sum = (Natural::from(0), Natural::from(1));
        for timing in &events {
            let (numerator, denominator) = sum;
            sum = (
                numerator
                    .times(timing.nodes)
                    .plus(&denominator.times(timing.neighbour_types)),
                denominator.times(timing.nodes),
            );
        }
        let (numerator, denominator) = sum;
        let mut observation: Option<&TypeTiming> = None;
        for timing in &events {
            // neighbour_types / nodes >= numerator / (denominator × events)
            let at_least_mean = denominator
                .times(events.len() as u64)
                .times(timing.neighbour_types)
                .compare(&numerator.times(timing.nodes))
                != Ordering::Less;
            if at_least_mean && observation.is_none_or(|best| timing.nodes > best.nodes) {
                observation = Some(timing);
            }
        }
        (
            OverviewRoles {
                observation_type: observation.map(|timing| timing.type_id),
                types: timings,
            },
            times,
        )
    }

    /// `ekr.views.OverviewTimeline`: node-subject assertions per valid-time bucket per type.
    fn overview_timeline(&self) -> OverviewTimeline {
        let graph = &self.loaded.graph;
        let mut dated: Vec<(i64, TypeId)> = Vec::new();
        let mut undated = 0_u64;
        for claim in graph.assertions.values() {
            let Subject::Node(node) = &claim.subject else {
                continue;
            };
            let Some(held) = graph.nodes.get(&node.id()) else {
                continue;
            };
            match claim.valid_time.from {
                Some(from) => dated.push((from.millis(), held.type_id)),
                None => undated += 1,
            }
        }
        let first = dated.iter().map(|(at, _)| *at).min();
        let last = dated.iter().map(|(at, _)| *at).max();
        let weekly = matches!((first, last), (Some(first), Some(last))
            if i128::from(last) - i128::from(first) > i128::from(WEEKLY_SPAN_MS));
        let width = bucket_width(first.zip(last), weekly);
        let mut buckets: BTreeMap<(i64, TypeId), u64> = BTreeMap::new();
        for (at, type_id) in dated {
            *buckets
                .entry((bucket_start(at, width), type_id))
                .or_default() += 1;
        }
        OverviewTimeline {
            bucket_ms: width.unsigned_abs(),
            first,
            last,
            undated,
            buckets: buckets
                .into_iter()
                .map(|((start, type_id), assertions)| TimelineBucket {
                    start,
                    type_id,
                    assertions,
                })
                .collect(),
        }
    }
}

const HOUR_MS: i128 = 3_600_000;
pub(crate) const DAY_MS: i64 = 86_400_000;
pub(crate) const WEEK_MS: i64 = 604_800_000;
/// 120 days: a span longer than this is bucketed by week.
pub(crate) const WEEKLY_SPAN_MS: i64 = 10_368_000_000;
/// Three days: the epoch fell on a Thursday, so a week starting on Monday is shifted by this.
const MONDAY_SHIFT_MS: i64 = 259_200_000;

/// The most buckets a span is cut into; past it the next coarser width is taken.
pub(crate) const MAX_BUCKETS: i128 = 200;

/// The start of the bucket of `width` holding `at`: its UTC day for a day, otherwise the start
/// of the `width`-long run of weeks from Monday 00:00 UTC that holds it; floor rounds toward
/// negative infinity.
pub(crate) fn bucket_start(at: i64, width: i64) -> i64 {
    let (at, width) = (i128::from(at), i128::from(width));
    let start = if width == i128::from(DAY_MS) {
        at.div_euclid(width) * width
    } else {
        (at + i128::from(MONDAY_SHIFT_MS)).div_euclid(width) * width - i128::from(MONDAY_SHIFT_MS)
    };
    i64::try_from(start).unwrap_or(i64::MIN)
}

/// The `n`th bucket width, finest first: a day, a week, 4 weeks, 13 weeks, 52 weeks, then 52
/// weeks doubled `n - 4` times.
fn nth_width(n: u32) -> i128 {
    let week = i128::from(WEEK_MS);
    match n {
        0 => i128::from(DAY_MS),
        1 => week,
        2 => 4 * week,
        3 => 13 * week,
        _ => (52 * week).saturating_mul(1_i128 << (n - 4).min(64)),
    }
}

/// The bucket width for `span` (`views.yaml`, ekr.views.OverviewTimeline): the first width, from
/// a week when `weekly` and a day otherwise, that cuts `[first, last]` into at most
/// [`MAX_BUCKETS`] buckets.
pub(crate) fn bucket_width(span: Option<(i64, i64)>, weekly: bool) -> i64 {
    let mut n = u32::from(weekly);
    loop {
        let width = nth_width(n);
        let fits = match (span, i64::try_from(width)) {
            (_, Err(_)) => true,
            (None, Ok(_)) => true,
            (Some((first, last)), Ok(width)) => {
                (i128::from(bucket_start(last, width)) - i128::from(bucket_start(first, width)))
                    / i128::from(width)
                    < MAX_BUCKETS
            }
        };
        if fits {
            return i64::try_from(width).unwrap_or(i64::MAX);
        }
        n += 1;
    }
}

/// Every type and property id `ontology` declares, with its kind and the name it gives it:
/// node types, then edge types, each by id, and a property under the first type declaring it.
fn members(ontology: &Ontology) -> BTreeMap<String, (&'static str, String)> {
    let document = ontology.to_document();
    let mut node_types: Vec<_> = document.node_types.iter().collect();
    node_types.sort_by_key(|declared| declared.id);
    let mut edge_types: Vec<_> = document.edge_types.iter().collect();
    edge_types.sort_by_key(|declared| declared.id);
    let mut members = BTreeMap::new();
    for declared in &node_types {
        members
            .entry(declared.id.to_string())
            .or_insert(("NodeType", declared.name.clone()));
    }
    for declared in &edge_types {
        members
            .entry(declared.id.to_string())
            .or_insert(("EdgeType", declared.name.clone()));
    }
    let properties = node_types
        .iter()
        .flat_map(|declared| declared.properties.values())
        .chain(
            edge_types
                .iter()
                .flat_map(|declared| declared.properties.values()),
        );
    for property in properties {
        members
            .entry(property.id.to_string())
            .or_insert(("Property", property.name.clone()));
    }
    members
}

/// An unsigned integer of any size, little-endian in 32-bit limbs: enough to compare the
/// observation-type ratios exactly, as `views.yaml` requires, however many types there are.
#[derive(Clone, Debug)]
struct Natural(Vec<u32>);

impl Natural {
    fn from(value: u64) -> Self {
        let mut natural = Self(vec![(value & 0xffff_ffff) as u32, (value >> 32) as u32]);
        natural.trim();
        natural
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn times(&self, factor: u64) -> Self {
        let (low, high) = (factor & 0xffff_ffff, factor >> 32);
        self.times_limb(low).plus(&self.times_limb(high).shifted())
    }

    fn times_limb(&self, factor: u64) -> Self {
        let mut limbs = Vec::with_capacity(self.0.len() + 1);
        let mut carry = 0_u64;
        for limb in &self.0 {
            let product = u64::from(*limb) * factor + carry;
            limbs.push((product & 0xffff_ffff) as u32);
            carry = product >> 32;
        }
        limbs.push(carry as u32);
        let mut natural = Self(limbs);
        natural.trim();
        natural
    }

    fn shifted(&self) -> Self {
        if self.0.is_empty() {
            return self.clone();
        }
        let mut limbs = vec![0];
        limbs.extend(&self.0);
        Self(limbs)
    }

    fn plus(&self, other: &Self) -> Self {
        let mut limbs = Vec::with_capacity(self.0.len().max(other.0.len()) + 1);
        let mut carry = 0_u64;
        for at in 0..self.0.len().max(other.0.len()) {
            let sum = u64::from(self.0.get(at).copied().unwrap_or(0))
                + u64::from(other.0.get(at).copied().unwrap_or(0))
                + carry;
            limbs.push((sum & 0xffff_ffff) as u32);
            carry = sum >> 32;
        }
        limbs.push(carry as u32);
        let mut natural = Self(limbs);
        natural.trim();
        natural
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// The indexes of the most recently used revisions, keyed by revision and the head they were
/// loaded under, evicting the least recently used beyond its capacity.
#[derive(Debug)]
pub struct IndexCache {
    capacity: usize,
    /// Most recently used first.
    entries: VecDeque<Arc<Index>>,
}

impl IndexCache {
    /// The capacity `ekr view` keeps: three revisions.
    pub const DEFAULT_CAPACITY: usize = 3;

    /// An empty cache holding at most `capacity` indexes (at least one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::new(),
        }
    }

    /// How many indexes it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The index of `revision` loaded under `head`, if held; it becomes the most recently used.
    pub fn get(&mut self, revision: RevisionNumber, head: RevisionNumber) -> Option<Arc<Index>> {
        let at = self
            .entries
            .iter()
            .position(|index| index.revision() == revision && index.head() == head)?;
        let index = self.entries.remove(at)?;
        self.entries.push_front(Arc::clone(&index));
        Some(index)
    }

    /// Holds `index` as the most recently used, replacing one of the same revision and head and
    /// evicting the least recently used beyond the capacity. Returns it.
    pub fn insert(&mut self, index: Arc<Index>) -> Arc<Index> {
        self.entries
            .retain(|held| (held.revision(), held.head()) != (index.revision(), index.head()));
        self.entries.push_front(Arc::clone(&index));
        self.entries.truncate(self.capacity);
        index
    }

    /// The index of revision `at` of `runtime`'s store (its head when `None`): the held one if
    /// the store's head is still the one it was loaded under, else [`Index::load`]ed and held.
    ///
    /// # Errors
    ///
    /// [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`], or whatever
    /// [`Index::load`] refuses.
    pub fn index(
        &mut self,
        runtime: &Runtime,
        at: Option<RevisionNumber>,
    ) -> Result<Arc<Index>, ProjectError> {
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
        if let Some(index) = self.get(revision, head) {
            return Ok(index);
        }
        Ok(self.insert(Arc::new(Index::load(runtime, Some(revision))?)))
    }
}
