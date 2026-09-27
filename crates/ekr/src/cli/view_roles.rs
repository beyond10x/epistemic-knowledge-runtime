//! The roles `ekr view` serves at `GET /roles`: which node types the viewer lays out as events,
//! which as subjects and which as observations, derived from the shape of one loaded revision
//! and from nothing else (`story:data-free-graph-viewer`).
//!
//! The body is `ekr.view-roles/1` and is **not** part of `ekr.graph-projection/1`:
//! `{"format":"ekr.view-roles/1","revision":N,"node_types":[{"type_id":…,"role":…}]}`, one entry
//! per placed type, ordered by `type_id`. A type the rule cannot place has no entry.
//!
//! # The rule
//!
//! It reads structure only: the ontology's edge-type source and target types, which types carry
//! valid-time assertions, degree and direction. No type, edge-type, property or entity name, and
//! no id, is ever compared to a constant, so renaming everything in a store changes nothing here.
//!
//! 1. **Arcs.** For every edge type of the revision's ontology, every type in its `source_types`
//!    has an arc to every type in its `target_types`; a `symmetric` edge type adds the reverse arc
//!    too. An arc from a type to itself is dropped. Type ids are taken as written: a type's
//!    `parents` are not consulted.
//! 2. **Degree.** A type's *targets* are the distinct other types it has an arc to; its *sources*
//!    are the distinct other types that have an arc to it.
//! 3. **Timed.** A type is *timed* when some assertion the revision holds — whatever its
//!    assessment or lifecycle, property or relation — has a node of that type as its subject and a
//!    valid time with at least one bound (`from` or `to` set). Assertions about an edge or a type
//!    do not count.
//! 4. **Advancing.** A type is *advancing* when it is timed and has at least one target.
//!
//! Then each node type of the ontology gets the first of these that holds:
//!
//! | role | when |
//! |---|---|
//! | `observation` | it has no sources, and at least one of its targets is advancing |
//! | `event` | it is advancing |
//! | `subject` | it has at least one source |
//! | *(none)* | otherwise: it is absent from the body |
//!
//! So an observation points at events and nothing points at it; an event is timed and points on;
//! a subject is pointed at and is not an event.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::TypeId;
use ekr_graph::{CanonicalGraph, Subject};
use serde::Serialize;

/// The format literal every `/roles` body carries.
pub(super) const FORMAT: &str = "ekr.view-roles/1";

/// A role the viewer lays a node type out by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Role {
    Event,
    Subject,
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
    /// The timed types.
    timed: BTreeSet<TypeId>,
}

impl Shape {
    fn of(graph: &CanonicalGraph) -> Self {
        let ontology = graph.ontology.to_document();
        let mut shape = Self {
            types: ontology
                .node_types
                .iter()
                .map(|declared| declared.id)
                .collect(),
            ..Self::default()
        };
        for edge_type in &ontology.edge_types {
            for &source in &edge_type.source_types {
                for &target in &edge_type.target_types {
                    shape.arc(source, target);
                    if edge_type.symmetric {
                        shape.arc(target, source);
                    }
                }
            }
        }
        for assertion in graph.assertions.values() {
            let Subject::Node(node) = assertion.subject else {
                continue;
            };
            let bounded = assertion.valid_time.from.is_some() || assertion.valid_time.to.is_some();
            if let (true, Some(node)) = (bounded, graph.nodes.get(&node.node())) {
                shape.timed.insert(node.type_id);
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

    fn advancing(&self, of: TypeId) -> bool {
        self.timed.contains(&of) && self.targets(of).next().is_some()
    }

    /// The rule of the module docs, for every declared type it places.
    fn roles(&self) -> BTreeMap<TypeId, Role> {
        self.types
            .iter()
            .filter_map(|&of| {
                let role = if !self.has_sources(of)
                    && self.targets(of).any(|&target| self.advancing(target))
                {
                    Role::Observation
                } else if self.advancing(of) {
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

/// The roles of `graph`'s node types, by type id: the rule of the module docs.
pub(super) fn derive(graph: &CanonicalGraph) -> BTreeMap<TypeId, Role> {
    Shape::of(graph).roles()
}

/// The `ekr.view-roles/1` bytes for `graph`, the canonical state of one loaded revision.
pub(super) fn document(graph: &CanonicalGraph) -> Vec<u8> {
    let document = Document {
        format: FORMAT,
        revision: graph.revision.get(),
        node_types: derive(graph)
            .into_iter()
            .map(|(type_id, role)| Placed { type_id, role })
            .collect(),
    };
    serde_json::to_vec(&document).expect("a roles document always serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> TypeId {
        format!("00000000-0000-4000-8000-{n:012}").parse().unwrap()
    }

    fn shape(types: &[u128], arcs: &[(u128, u128)], timed: &[u128]) -> Shape {
        let mut shape = Shape {
            types: types.iter().copied().map(id).collect(),
            timed: timed.iter().copied().map(id).collect(),
            ..Shape::default()
        };
        for &(from, to) in arcs {
            shape.arc(id(from), id(to));
        }
        shape
    }

    #[test]
    fn the_rule_places_by_direction_and_time_alone() {
        // 1 → 2 → 3, 2 timed: 1 observes 2, 2 is an event, 3 a subject; 4 has only a self-loop.
        let roles = shape(&[1, 2, 3, 4], &[(1, 2), (2, 3), (4, 4)], &[2]).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
        assert_eq!(roles.get(&id(3)), Some(&Role::Subject));
        assert_eq!(roles.get(&id(4)), None);
    }

    #[test]
    fn a_timed_sink_is_a_subject_and_an_untimed_graph_has_no_events() {
        let roles = shape(&[1, 2], &[(1, 2)], &[2]).roles();
        assert_eq!(roles.get(&id(1)), None);
        assert_eq!(roles.get(&id(2)), Some(&Role::Subject));
        let roles = shape(&[1, 2, 3], &[(1, 2), (2, 3)], &[]).roles();
        assert!(roles.values().all(|role| *role == Role::Subject));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn a_timed_source_that_points_at_an_event_observes_it() {
        let roles = shape(&[1, 2, 3], &[(1, 2), (2, 3)], &[1, 2]).roles();
        assert_eq!(roles.get(&id(1)), Some(&Role::Observation));
        assert_eq!(roles.get(&id(2)), Some(&Role::Event));
    }
}
