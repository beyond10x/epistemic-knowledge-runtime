//! A canonical reference names only a kind canonical state holds a map of.
//!
//! `CanonicalTarget` was implemented for `Observation`, `Support` and `GraphRoot`, and
//! `CanonicalGraph` holds none of them by id: observations are not canonical state, a support is
//! not stored beside the assertion it joins, and a graph has exactly one root. A reference of one
//! of those kinds is a reference nothing can resolve, so it is not a type.

use ekr_graph::{CanonicalRef, GraphRoot, Observation, Support};

fn to_an_observation(_: CanonicalRef<Observation>) {}

fn to_a_support(_: CanonicalRef<Support>) {}

fn to_a_graph_root(_: CanonicalRef<GraphRoot>) {}

fn main() {}
