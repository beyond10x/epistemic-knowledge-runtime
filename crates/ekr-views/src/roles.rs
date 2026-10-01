//! The roles `ekr view` serves at `GET /roles` (`ekr.view-roles/1`): which node types the viewer
//! lays out as events, which as subjects and which as the observation, derived from one indexed
//! revision and from nothing else (`story:data-free-graph-viewer`).
//!
//! The body is `ekr.view-roles/1` and is **not** part of `ekr.graph-projection/1`:
//! `{"format":"ekr.view-roles/1","revision":N,"node_types":[{"type_id":…,"role":…}]}`, one entry
//! per placed type, ordered by `type_id`. A type the rule cannot place has no entry.
//!
//! # The rule
//!
//! It reads structure and time only: the revision's event types and observation type, and the
//! ontology's edge-type source and target types. No type, edge-type, property or entity name, and
//! no id, is ever compared to a constant, so renaming everything in a store changes nothing here.
//!
//! 1. **Events.** The event types are [`Index::event_types`]: EKR's one rule (`views.yaml`,
//!    `ekr.views.TypeTiming` `event`), the one the timeline and `ekr.ocel/1` read. The
//!    observation type is the overview's `roles.observation_type`, one of the event types, the
//!    one the viewer lays out as the observation.
//! 2. **Arcs.** For every edge type of the revision's ontology, first widen its `source_types`
//!    and its `target_types` each to every node type that conforms to one of them — the listed
//!    types and all their descendants through `parents`, transitively — since that is how the
//!    kernel checks an edge's endpoints (`Ontology::conforms_to`). Then every widened source type
//!    has an arc to every widened target type; a `symmetric` edge type adds the reverse arc too.
//!    An arc from a type to itself is dropped. Abstract types are types like any other here.
//! 3. **Sources.** A type's *sources* are the distinct other types that have an arc to it.
//!
//! Then each node type of the ontology gets the first of these that holds:
//!
//! | role | when |
//! |---|---|
//! | `observation` | it is the observation type |
//! | `event` | it is an event type |
//! | `subject` | it has at least one source |
//! | *(none)* | otherwise: it is absent from the body |
//!
//! So the `event` and `observation` entries together are exactly the event types the timeline
//! and `ekr.ocel/1` use, the `observation` entry is the overview's observation type, and a
//! subject is pointed at and is not an event type.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::TypeId;
use ekr_graph::CanonicalGraph;
use serde::Serialize;

use crate::Index;

/// The format literal every `/roles` body carries.
pub const ROLES_FORMAT: &str = "ekr.view-roles/1";

/// A role the viewer lays a node type out by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Laid out as an event.
    Event,
    /// Laid out as a subject.
    Subject,
    /// Laid out as the observation.
    Observation,
}

/// The structure the rule reads, and nothing else.
#[derive(Debug, Default)]
struct Shape {
    /// Every node type the ontology declares.
    types: BTreeSet<TypeId>,
    /// Each type's sources.
    sources: BTreeMap<TypeId, BTreeSet<TypeId>>,
    /// The event types: [`Index::event_types`].
    events: BTreeSet<TypeId>,
    /// The overview's observation type.
    observation: Option<TypeId>,
}

impl Shape {
    fn of(graph: &CanonicalGraph, events: BTreeSet<TypeId>, observation: Option<TypeId>) -> Self {
        let ontology = graph.ontology.to_document();
        let mut shape = Self {
            types: ontology
                .node_types
                .iter()
                .map(|declared| declared.id)
                .collect(),
            events,
            observation,
            ..Self::default()
        };
        // Every declared type that conforms to one of `declared`: the declared types and all their
        // descendants, which is how the kernel checks an edge's endpoints.
        let widened = |declared: &BTreeSet<TypeId>| -> BTreeSet<TypeId> {
            shape
                .types
                .iter()
                .copied()
                .filter(|&candidate| {
                    declared
                        .iter()
                        .any(|&allowed| graph.ontology.conforms_to(candidate, allowed))
                })
                .collect()
        };
        let mut arcs = Vec::new();
        for edge_type in &ontology.edge_types {
            let (sources, targets) = (
                widened(&edge_type.source_types),
                widened(&edge_type.target_types),
            );
            arcs.push((sources, targets, edge_type.symmetric));
        }
        for (sources, targets, symmetric) in arcs {
            for &source in &sources {
                for &target in &targets {
                    shape.arc(source, target);
                    if symmetric {
                        shape.arc(target, source);
                    }
                }
            }
        }
        shape
    }

    fn arc(&mut self, from: TypeId, to: TypeId) {
        if from != to {
            self.sources.entry(to).or_default().insert(from);
        }
    }

    fn has_sources(&self, of: TypeId) -> bool {
        self.sources
            .get(&of)
            .is_some_and(|sources| !sources.is_empty())
    }

    /// The rule of the module docs, for every declared type it places.
    fn roles(&self) -> BTreeMap<TypeId, Role> {
        self.types
            .iter()
            .filter_map(|&of| {
                let role = if self.observation == Some(of) {
                    Role::Observation
                } else if self.events.contains(&of) {
                    Role::Event
                } else if self.has_sources(of) {
                    Role::Subject
                } else {
                    return None;
                };
                Some((of, role))
            })
            .collect()
    }
}

#[derive(Serialize)]
struct Placed {
    type_id: TypeId,
    role: Role,
}

#[derive(Serialize)]
struct Document {
    format: &'static str,
    revision: u64,
    node_types: Vec<Placed>,
}

impl Index {
    /// The roles of the indexed revision's node types, by type id: the rule of the module docs.
    #[must_use]
    pub fn view_roles(&self) -> BTreeMap<TypeId, Role> {
        Shape::of(
            &self.loaded.graph,
            self.event_types(),
            self.overview.roles.observation_type,
        )
        .roles()
    }

    /// The `ekr.view-roles/1` bytes of the indexed revision.
    #[must_use]
    pub fn view_roles_document(&self) -> Vec<u8> {
        let document = Document {
            format: ROLES_FORMAT,
            revision: self.loaded.graph.revision.get(),
            node_types: self
                .view_roles()
                .into_iter()
                .map(|(type_id, role)| Placed { type_id, role })
                .collect(),
        };
        serde_json::to_vec(&document).expect("a roles document always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> TypeId {
        format!("00000000-0000-4000-8000-{n:012}").parse().unwrap()
    }

    fn shape(
        types: &[u128],
        arcs: &[(u128, u128)],
        events: &[u128],
        observation: Option<u128>,
    ) -> Shape {
        let mut shape = Shape {
            types: types.iter().copied().map(id).collect(),
            events: events.iter().copied().map(id).collect(),
            observation: observation.map(id),
            ..Shape::default()
        };
        for &(from, to) in arcs {
            shape.arc(id(from), id(to));
        }
        shape
    }

    #[test]
    fn the_rule_places_by_the_event_types_the_observation_type_and_sources() {
        // 1 → 2 → 3, event types 1 and 2, 1 the observation type; 4 has only a self-loop.
        let roles = shape(&[1, 2, 3, 4], &[(1, 2), (2, 3), (4, 4)], &[1, 2], Some(1)).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(3)), Some(&Role::Subject));
        assert_eq!(roles.get(&id(4)), None);
    }

    #[test]
    fn every_event_type_but_the_observation_type_is_an_event_whatever_its_arcs() {
        // 2 is a sink, 5 has no arc: both event types, so both events; 3 is the observation.
        let roles = shape(&[1, 2, 3, 5], &[(1, 2), (3, 1)], &[2, 3, 5], Some(3)).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Subject));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(3)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(5)), Some(&Role::Event));
    }

    #[test]
    fn no_event_type_places_no_event_and_no_observation() {
        // 1 points at nothing that matters any more: with no event type it has no role.
        let roles = shape(&[1, 2, 3], &[(1, 2), (2, 3)], &[], None).roles();
        assert!(roles.values().all(|role| *role == Role::Subject));
        assert_eq!(roles.len(), 2);
        assert_eq!(roles.get(&id(1)), None);
    }
}
