//! The roles `ekr view` serves at `GET /roles` (`ekr.view-roles/1`): which node types the viewer
//! lays out as events, which as subjects and which as observations, derived from the shape of one
//! indexed revision and from nothing else (`story:data-free-graph-viewer`).
//!
//! The body is `ekr.view-roles/1` and is **not** part of `ekr.graph-projection/1`:
//! `{"format":"ekr.view-roles/1","revision":N,"node_types":[{"type_id":…,"role":…}]}`, one entry
//! per placed type, ordered by `type_id`. A type the rule cannot place has no entry.
//!
//! # The rule
//!
//! It reads structure and time only: the ontology's edge-type source and target types, and the
//! revision's event types. No type, edge-type, property or entity name, and no id, is ever
//! compared to a constant, so renaming everything in a store changes nothing here.
//!
//! 1. **Arcs.** For every edge type of the revision's ontology, first widen its `source_types`
//!    and its `target_types` each to every node type that conforms to one of them — the listed
//!    types and all their descendants through `parents`, transitively — since that is how the
//!    kernel checks an edge's endpoints (`Ontology::conforms_to`). Then every widened source type
//!    has an arc to every widened target type; a `symmetric` edge type adds the reverse arc too.
//!    An arc from a type to itself is dropped. Abstract types are types like any other here.
//! 2. **Degree.** A type's *targets* are the distinct other types it has an arc to; its *sources*
//!    are the distinct other types that have an arc to it.
//! 3. **Events.** The event types are [`Index::event_types`]: EKR's one rule
//!    (`views.yaml`, `ekr.views.TypeTiming` `event`), the one the timeline and `ekr.ocel/1` read.
//!
//! Then each node type of the ontology gets the first of these that holds:
//!
//! | role | when |
//! |---|---|
//! | `event` | it is an event type |
//! | `observation` | it has no sources, and at least one of its targets is an event type |
//! | `subject` | it has at least one source |
//! | *(none)* | otherwise: it is absent from the body |
//!
//! So the `event` entries are exactly the event types the timeline and `ekr.ocel/1` use; an
//! observation points at events and nothing points at it; a subject is pointed at and is not an
//! event.

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
    /// Laid out as an observation.
    Observation,
}

/// The structure the rule reads, and nothing else.
#[derive(Debug, Default)]
struct Shape {
    /// Every node type the ontology declares.
    types: BTreeSet<TypeId>,
    /// Each type's targets.
    targets: BTreeMap<TypeId, BTreeSet<TypeId>>,
    /// Each type's sources.
    sources: BTreeMap<TypeId, BTreeSet<TypeId>>,
    /// The event types: [`Index::event_types`].
    events: BTreeSet<TypeId>,
}

impl Shape {
    fn of(graph: &CanonicalGraph, events: BTreeSet<TypeId>) -> Self {
        let ontology = graph.ontology.to_document();
        let mut shape = Self {
            types: ontology
                .node_types
                .iter()
                .map(|declared| declared.id)
                .collect(),
            events,
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
        if from == to {
            return;
        }
        self.targets.entry(from).or_default().insert(to);
        self.sources.entry(to).or_default().insert(from);
    }

    fn targets(&self, of: TypeId) -> impl Iterator<Item = &TypeId> {
        self.targets.get(&of).into_iter().flatten()
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
                let role = if self.events.contains(&of) {
                    Role::Event
                } else if !self.has_sources(of)
                    && self.targets(of).any(|target| self.events.contains(target))
                {
                    Role::Observation
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
        Shape::of(&self.loaded.graph, self.event_types()).roles()
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

    fn shape(types: &[u128], arcs: &[(u128, u128)], events: &[u128]) -> Shape {
        let mut shape = Shape {
            types: types.iter().copied().map(id).collect(),
            events: events.iter().copied().map(id).collect(),
            ..Shape::default()
        };
        for &(from, to) in arcs {
            shape.arc(id(from), id(to));
        }
        shape
    }

    #[test]
    fn the_rule_places_by_direction_and_the_event_types() {
        // 1 → 2 → 3, 2 an event type: 1 observes 2, 2 is an event, 3 a subject; 4 has only a
        // self-loop.
        let roles = shape(&[1, 2, 3, 4], &[(1, 2), (2, 3), (4, 4)], &[2]).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(3)), Some(&Role::Subject));
        assert_eq!(roles.get(&id(4)), None);
    }

    #[test]
    fn every_event_type_is_an_event_whatever_its_arcs_and_no_other_type_is() {
        // 2 is a sink, 5 has no arc: both event types, so both events.
        let roles = shape(&[1, 2, 5], &[(1, 2)], &[2, 5]).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(5)), Some(&Role::Event));
        // No event type: no event and no observation.
        let roles = shape(&[1, 2, 3], &[(1, 2), (2, 3)], &[]).roles();
        assert!(roles.values().all(|role| *role == Role::Subject));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn an_event_type_that_points_at_an_event_is_an_event_not_an_observation() {
        let roles = shape(&[1, 2, 3], &[(1, 2), (2, 3)], &[1, 2]).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Event));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(3)), Some(&Role::Subject));
    }
}
