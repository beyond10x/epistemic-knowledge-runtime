//! `ProjectTimeline` and its format, `ekr.graph-timeline/1` ([`Index::timeline`]): one row per
//! subject — a node of the row type — with the events related to it within a number of hops,
//! counted per time bucket (`views.yaml`, `ekr.views.GraphTimelineV1`).
//!
//! Everything the walk reads was built with the index: each node's type, whether that type is an
//! event type of the overview's roles, each node's time and end, and each edge's source and type.
//! The row types are ranked once per revision, on first use. A request then costs the walks from
//! the subjects of its row type — each subject's neighbourhood within its hops — and the answer.

use std::collections::{BTreeMap, HashMap, HashSet};

use ekr_core::{EdgeId, NodeId, TypeId};
use serde::Serialize;

use crate::index::{bucket_start, bucket_width, Index, RowTypeRank, WEEKLY_SPAN_MS};
use crate::query::{bounded, encode, hash, Answer, LimitExceeded};
use crate::ProjectError;

/// The format literal of [`Index::timeline`]'s documents.
pub const TIMELINE_FORMAT: &str = "ekr.graph-timeline/1";

/// How many row types the timeline offers at most.
const ROW_TYPES: usize = 6;
/// The hops the row types are ranked at.
const RANKING_HOPS: u64 = 2;
/// No node reached it: the walk's start.
const START: u32 = u32::MAX;

/// `ekr.views.TimelineBucketWidth`: the bucket a request asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BucketWidth {
    /// A UTC day.
    Day,
    /// A week from Monday 00:00 UTC.
    Week,
}

/// A bounded `ProjectTimeline` request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineRequest {
    row_type: Option<TypeId>,
    hops: u64,
    limit: u64,
    bucket: Option<BucketWidth>,
    subject: Option<NodeId>,
}

impl TimelineRequest {
    /// The greatest admitted hop count.
    pub const MAX_HOPS: i64 = 3;
    /// The greatest admitted number of rows.
    pub const MAX_LIMIT: i64 = 500;

    /// `row_type` is the rows' type (the first listed row type when `None`); `hops` is 1 to 3,
    /// `limit` 1 to 500 rows; `bucket` the width (the span's when `None`); `subject` asks for that
    /// node's row alone, with its events.
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming the first broken input of `hops` and `limit`.
    pub fn new(
        row_type: Option<TypeId>,
        hops: i64,
        limit: i64,
        bucket: Option<BucketWidth>,
        subject: Option<NodeId>,
    ) -> Result<Self, LimitExceeded> {
        let hops = bounded("hops", hops, 1, Some(Self::MAX_HOPS))?;
        let limit = bounded("limit", limit, 1, Some(Self::MAX_LIMIT))?;
        Ok(Self {
            row_type,
            hops,
            limit,
            bucket,
            subject,
        })
    }

    /// The row type asked for.
    #[must_use]
    pub const fn row_type(&self) -> Option<TypeId> {
        self.row_type
    }

    /// The hop count.
    #[must_use]
    pub const fn hops(&self) -> u64 {
        self.hops
    }

    /// The most rows answered.
    #[must_use]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    /// The bucket width asked for.
    #[must_use]
    pub const fn bucket(&self) -> Option<BucketWidth> {
        self.bucket
    }

    /// The subject asked for.
    #[must_use]
    pub const fn subject(&self) -> Option<NodeId> {
        self.subject
    }
}

/// `ekr.views.SubjectsTimelined`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubjectsTimelined {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.head`.
    pub head: u64,
    /// `meta.row_type`.
    pub row_type: Option<TypeId>,
    /// `meta.hops`.
    pub hops: u64,
    /// `meta.bucket_ms`.
    pub bucket_ms: u64,
    /// Entries of `row_types`.
    pub row_types: u64,
    /// `meta.subjects`.
    pub subjects: u64,
    /// `meta.active`.
    pub active: u64,
    /// Entries of `rows`.
    pub rows: u64,
    /// `meta.events`.
    pub events: u64,
    /// The sum of the rows' totals.
    pub row_events: u64,
    /// The sum of the rows' cell counts.
    pub cells: u64,
    /// Entries of `strip`.
    pub strip: u64,
    /// The first row's id; `None` when there is none.
    pub first_row: Option<NodeId>,
    /// Entries of `events`.
    pub listed_events: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub timeline_hash: String,
}

// ---- the document -------------------------------------------------------------------------------

#[derive(Serialize)]
struct TimelineMeta {
    format: &'static str,
    revision: u64,
    head: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    row_type: Option<TypeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<NodeId>,
    hops: u64,
    limit: u64,
    bucket_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    first: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last: Option<i64>,
    subjects: u64,
    active: u64,
    events: u64,
}

#[derive(Serialize)]
struct TimelineRowType {
    #[serde(rename = "type")]
    type_id: TypeId,
    nodes: u64,
    reached: u64,
    weight: u64,
}

#[derive(Serialize)]
struct TimelineCell {
    start: i64,
    #[serde(rename = "type")]
    type_id: TypeId,
    events: u64,
}

#[derive(Serialize)]
struct TimelineRow<'a> {
    id: NodeId,
    name: &'a str,
    total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    first: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last: Option<i64>,
    cells: Vec<TimelineCell>,
}

#[derive(Serialize)]
struct TimelineStep<'a> {
    edge: EdgeId,
    edge_type: TypeId,
    forward: bool,
    node: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    name: &'a str,
}

#[derive(Serialize)]
struct TimelineEvent<'a> {
    id: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    name: &'a str,
    start: i64,
    end: i64,
    distance: u64,
    path: Vec<TimelineStep<'a>>,
}

#[derive(Serialize)]
struct GraphTimelineV1<'a> {
    meta: TimelineMeta,
    row_types: Vec<TimelineRowType>,
    rows: Vec<TimelineRow<'a>>,
    strip: Vec<TimelineCell>,
    events: Vec<TimelineEvent<'a>>,
}

// ---- the walk -----------------------------------------------------------------------------------

/// How the walk reached a node: its hop, the node that reached it and the edge it crossed; the
/// subject has neither.
#[derive(Clone, Copy, Debug)]
struct Reach {
    distance: u64,
    from: u32,
    edge: u32,
}

/// What one walk found: the events in the order it reached them, and how it reached every node.
struct Walk {
    events: Vec<u32>,
    reached: HashMap<u32, Reach>,
}

/// How many distinct neighbours of a node are of a type, kept for the life of one request.
type Shares = HashMap<(u32, TypeId), usize>;

impl Index {
    /// The distinct neighbours of `node` of type `type_id`, in either direction.
    fn share(&self, node: u32, type_id: TypeId, shares: &mut Shares) -> usize {
        *shares.entry((node, type_id)).or_insert_with(|| {
            let mut others: Vec<u32> = self
                .adjacent(node)
                .iter()
                .map(|(other, _)| *other)
                .filter(|other| self.node_type[*other as usize] == type_id)
                .collect();
            others.sort_unstable();
            others.dedup();
            others.len()
        })
    }

    /// The events within `hops` of `root`, by the reference viewer's walk (`views.yaml`,
    /// ekr.graph-timeline/1).
    fn walk(&self, root: u32, hops: u64, shares: &mut Shares) -> Walk {
        let root_event = self.node_event[root as usize];
        let mut reached = HashMap::from([(
            root,
            Reach {
                distance: 0,
                from: START,
                edge: START,
            },
        )]);
        let mut events = Vec::new();
        let mut frontier = vec![root];
        for distance in 1..=hops {
            if frontier.is_empty() {
                break;
            }
            let mut next = Vec::new();
            for &node in &frontier {
                let event = node != root && self.node_event[node as usize];
                if root_event && node != root && !event {
                    continue;
                }
                let intermediate = node != root && !event;
                if intermediate {
                    let from = reached[&node].from;
                    if self.share(node, self.node_type[from as usize], shares) > 1 {
                        continue;
                    }
                }
                for &(other, edge) in self.adjacent(node) {
                    if reached.contains_key(&other) {
                        continue;
                    }
                    if intermediate && self.edge_source[edge as usize] == node {
                        continue;
                    }
                    let other_event = self.node_event[other as usize];
                    if event && !other_event {
                        continue;
                    }
                    if other_event && self.node_time[other as usize].is_none() {
                        continue;
                    }
                    reached.insert(
                        other,
                        Reach {
                            distance,
                            from: node,
                            edge,
                        },
                    );
                    next.push(other);
                    if other_event {
                        events.push(other);
                    }
                }
            }
            frontier = next;
        }
        Walk { events, reached }
    }

    /// The timeline's row types, ranked once per revision.
    pub(crate) fn row_type_ranks(&self) -> &[RowTypeRank] {
        self.row_types.get_or_init(|| self.rank_row_types())
    }

    fn rank_row_types(&self) -> Vec<RowTypeRank> {
        let event_type = |type_id: &TypeId| {
            self.by_type[type_id]
                .first()
                .is_some_and(|n| self.node_event[*n as usize])
        };
        let every_event = self.by_type.keys().all(event_type);
        let observation = self.overview.roles.observation_type;
        let mut shares = Shares::new();
        let mut ranks: Vec<RowTypeRank> = self
            .by_type
            .iter()
            .filter(|(type_id, _)| {
                if every_event {
                    Some(**type_id) != observation
                } else {
                    !event_type(type_id)
                }
            })
            .map(|(type_id, nodes)| {
                let (mut weight, mut reached) = (0_u64, 0_u64);
                for &node in nodes {
                    let walk = self.walk(node, RANKING_HOPS, &mut shares);
                    let own: u64 = walk
                        .events
                        .iter()
                        .map(|event| {
                            if walk.reached[event].distance == 1 {
                                2
                            } else {
                                1
                            }
                        })
                        .sum();
                    weight += own;
                    reached += u64::from(own > 0);
                }
                RowTypeRank {
                    type_id: *type_id,
                    nodes: nodes.len() as u64,
                    reached,
                    weight,
                }
            })
            .collect();
        // weight / nodes descending, exactly: a before b when a.weight × b.nodes is greater.
        ranks.sort_by(|a, b| {
            (u128::from(b.weight) * u128::from(a.nodes))
                .cmp(&(u128::from(a.weight) * u128::from(b.nodes)))
                .then(a.type_id.cmp(&b.type_id))
        });
        let Some(top) = ranks.first().cloned() else {
            return ranks;
        };
        ranks.retain(|rank| {
            rank.weight > 0
                && 20 * u128::from(rank.weight) * u128::from(top.nodes)
                    >= u128::from(top.weight) * u128::from(rank.nodes)
        });
        ranks.truncate(ROW_TYPES);
        ranks
    }

    /// `ProjectTimeline`: this revision's `ekr.graph-timeline/1` for `request`. The row types
    /// are ranked on the index's first timeline; after that the answer costs the walks from the
    /// row type's subjects and the document.
    ///
    /// # Errors
    ///
    /// [`ProjectError::Inconsistent`] if the document cannot be encoded.
    pub fn timeline(
        &self,
        request: &TimelineRequest,
    ) -> Result<Answer<SubjectsTimelined>, ProjectError> {
        let graph = &self.loaded.graph;
        let ranks = self.row_type_ranks();
        let held = request
            .subject()
            .and_then(|subject| self.node_index.get(&subject).copied());
        let row_type = held
            .map(|subject| self.node_type[subject as usize])
            .or(request.row_type())
            .or_else(|| ranks.first().map(|rank| rank.type_id));

        // The span: the least time and the greatest end of the revision's timed events.
        let mut span: Option<(i64, i64)> = None;
        for (node, time) in self.node_time.iter().enumerate() {
            if let (true, Some((time, end))) = (self.node_event[node], time) {
                span = Some(span.map_or((*time, *end), |(first, last)| {
                    (first.min(*time), last.max(*end))
                }));
            }
        }
        let weekly = match request.bucket() {
            Some(BucketWidth::Day) => false,
            Some(BucketWidth::Week) => true,
            None => span.is_some_and(|(first, last)| {
                i128::from(last) - i128::from(first) > i128::from(WEEKLY_SPAN_MS)
            }),
        };
        let bucket_ms = bucket_width(span, weekly);

        let mut shares = Shares::new();
        let subjects: &[u32] = row_type
            .and_then(|type_id| self.by_type.get(&type_id))
            .map_or(&[], Vec::as_slice);
        let mut active: Vec<(u32, Vec<u32>)> = Vec::new();
        let mut union: HashSet<u32> = HashSet::new();
        for &subject in subjects {
            let walk = self.walk(subject, request.hops(), &mut shares);
            if walk.events.is_empty() {
                continue;
            }
            union.extend(walk.events.iter().copied());
            active.push((subject, walk.events));
        }
        active.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

        let cells = |events: &mut dyn Iterator<Item = u32>| -> Vec<TimelineCell> {
            let mut counted: BTreeMap<(i64, TypeId), u64> = BTreeMap::new();
            for event in events {
                if let Some((time, _)) = self.node_time[event as usize] {
                    *counted
                        .entry((
                            bucket_start(time, bucket_ms),
                            self.node_type[event as usize],
                        ))
                        .or_default() += 1;
                }
            }
            counted
                .into_iter()
                .map(|((start, type_id), events)| TimelineCell {
                    start,
                    type_id,
                    events,
                })
                .collect()
        };
        let name = |node: u32| {
            graph.nodes[&self.node_ids[node as usize]]
                .canonical_name
                .as_str()
        };
        let row = |subject: u32, events: &[u32]| -> TimelineRow<'_> {
            let times = events
                .iter()
                .filter_map(|event| self.node_time[*event as usize].map(|(time, _)| time));
            TimelineRow {
                id: self.node_ids[subject as usize],
                name: name(subject),
                total: events.len() as u64,
                first: times.clone().min(),
                last: times.max(),
                cells: cells(&mut events.iter().copied()),
            }
        };

        let mut listed_events = Vec::new();
        let rows: Vec<TimelineRow<'_>> = match (request.subject(), held) {
            (Some(_), None) => Vec::new(),
            (Some(_), Some(subject)) => {
                let walk = self.walk(subject, request.hops(), &mut shares);
                listed_events = self.listed(&walk);
                vec![row(subject, &walk.events)]
            }
            (None, _) => active
                .iter()
                .take(usize::try_from(request.limit()).unwrap_or(usize::MAX))
                .map(|(subject, events)| row(*subject, events))
                .collect(),
        };
        let strip = {
            let mut distinct: Vec<u32> = union.iter().copied().collect();
            distinct.sort_unstable();
            cells(&mut distinct.into_iter())
        };

        let document = GraphTimelineV1 {
            meta: TimelineMeta {
                format: TIMELINE_FORMAT,
                revision: graph.revision.get(),
                head: self.loaded.head.get(),
                row_type,
                subject: request.subject(),
                hops: request.hops(),
                limit: request.limit(),
                bucket_ms: bucket_ms.unsigned_abs(),
                first: span.map(|(first, _)| first),
                last: span.map(|(_, last)| last),
                subjects: subjects.len() as u64,
                active: active.len() as u64,
                events: union.len() as u64,
            },
            row_types: ranks
                .iter()
                .map(|rank| TimelineRowType {
                    type_id: rank.type_id,
                    nodes: rank.nodes,
                    reached: rank.reached,
                    weight: rank.weight,
                })
                .collect(),
            rows,
            strip,
            events: listed_events,
        };
        let bytes = encode(&document)?;
        let summary = SubjectsTimelined {
            revision: document.meta.revision,
            head: document.meta.head,
            row_type: document.meta.row_type,
            hops: document.meta.hops,
            bucket_ms: document.meta.bucket_ms,
            row_types: document.row_types.len() as u64,
            subjects: document.meta.subjects,
            active: document.meta.active,
            rows: document.rows.len() as u64,
            events: document.meta.events,
            row_events: document.rows.iter().map(|row| row.total).sum(),
            cells: document.rows.iter().map(|row| row.cells.len() as u64).sum(),
            strip: document.strip.len() as u64,
            first_row: document.rows.first().map(|row| row.id),
            listed_events: document.events.len() as u64,
            timeline_hash: hash(&bytes),
        };
        Ok(Answer { bytes, summary })
    }

    /// A walk's events with their paths, by time and then id.
    fn listed(&self, walk: &Walk) -> Vec<TimelineEvent<'_>> {
        let graph = &self.loaded.graph;
        let name = |node: u32| {
            graph.nodes[&self.node_ids[node as usize]]
                .canonical_name
                .as_str()
        };
        let mut events: Vec<TimelineEvent<'_>> = walk
            .events
            .iter()
            .filter_map(|event| {
                let (start, end) = self.node_time[*event as usize]?;
                let mut path = Vec::new();
                let mut at = *event;
                while let Some(reach) = walk.reached.get(&at).filter(|reach| reach.from != START) {
                    path.push(TimelineStep {
                        edge: self.edge_ids[reach.edge as usize],
                        edge_type: self.edge_type[reach.edge as usize],
                        forward: self.edge_source[reach.edge as usize] == reach.from,
                        node: self.node_ids[at as usize],
                        type_id: self.node_type[at as usize],
                        name: name(at),
                    });
                    at = reach.from;
                }
                path.reverse();
                Some(TimelineEvent {
                    id: self.node_ids[*event as usize],
                    type_id: self.node_type[*event as usize],
                    name: name(*event),
                    start,
                    end,
                    distance: walk.reached[event].distance,
                    path,
                })
            })
            .collect();
        events.sort_by(|a, b| a.start.cmp(&b.start).then(a.id.cmp(&b.id)));
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_names_the_first_broken_bound_of_hops_and_limit() {
        let refused = |hops, limit| TimelineRequest::new(None, hops, limit, None, None).err();
        assert_eq!(refused(0, 0).map(|e| e.parameter), Some("hops"));
        assert_eq!(refused(4, 1).map(|e| e.parameter), Some("hops"));
        assert_eq!(refused(1, 0).map(|e| e.parameter), Some("limit"));
        assert_eq!(refused(3, 501).map(|e| e.parameter), Some("limit"));
        assert!(refused(1, 1).is_none() && refused(3, 500).is_none());
        let broken = refused(4, 1).unwrap();
        assert_eq!((broken.minimum, broken.maximum), (1, Some(3)));
    }
}
