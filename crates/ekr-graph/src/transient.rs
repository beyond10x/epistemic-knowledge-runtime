//! Transient state: the side of the membrane that may depend on the other one.
//!
//! Design § 23–24. A transient root is where an observation becomes candidate nodes, candidate
//! entities and candidate relations, none of which is knowledge yet. It may point at canonical
//! state — an entity candidate resolving to a person the runtime already knows is the whole point
//! of the incubation step — and canonical state may not point back.
//!
//! The incubation forest that fills these roots is P3. The types are here now so the membrane is
//! typed from the beginning rather than retrofitted onto code that grew without it.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use ekr_core::{AssertionId, EdgeId, NodeId};

use crate::assertion::Assertion;
use crate::canonical::{sealed, CanonicalGraph, CanonicalRef, CanonicalTarget};
use crate::edge::Edge;
use crate::node::Node;
use crate::root::GraphRoot;

/// A reference to something inside one transient root: design § 23.
///
/// Local to the root that holds it. It is deliberately *not* a
/// [`CanonicalDependency`](crate::CanonicalDependency): a candidate that has not been integrated
/// is not something canonical state may rest on.
#[derive(Debug)]
pub struct LocalRef<T: CanonicalTarget> {
    node: NodeId,
    marker: PhantomData<fn() -> T>,
}

impl<T: CanonicalTarget> LocalRef<T> {
    /// A reference to `node` within the transient root that holds it.
    #[must_use]
    pub const fn new(node: NodeId) -> Self {
        Self {
            node,
            marker: PhantomData,
        }
    }

    /// The node it points at.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

impl<T: CanonicalTarget> Clone for LocalRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: CanonicalTarget> Copy for LocalRef<T> {}

impl<T: CanonicalTarget> PartialEq for LocalRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T: CanonicalTarget> Eq for LocalRef<T> {}

impl<T: CanonicalTarget> sealed::Sealed for LocalRef<T> {}

/// A reference transient state may hold: design § 23.
///
/// The union of the two directions that are allowed — a candidate of its own root, or something
/// canonical it has resolved to. There is no third arm and no equivalent type on the canonical
/// side, which is the asymmetry the membrane *is*.
#[derive(Debug)]
pub enum TransientRef<T: CanonicalTarget> {
    /// Something canonical state already holds.
    Canonical(CanonicalRef<T>),
    /// A candidate of this transient root.
    Local(LocalRef<T>),
}

impl<T: CanonicalTarget> TransientRef<T> {
    /// The node it points at, whichever side it points into.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        match self {
            Self::Canonical(reference) => reference.node(),
            Self::Local(reference) => reference.node(),
        }
    }
}

impl<T: CanonicalTarget> Clone for TransientRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: CanonicalTarget> Copy for TransientRef<T> {}

impl<T: CanonicalTarget> PartialEq for TransientRef<T> {
    /// By what is pointed at *and* which side it is on: a candidate that happens to carry the same
    /// id as a canonical node is not that node.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Canonical(left), Self::Canonical(right)) => left == right,
            (Self::Local(left), Self::Local(right)) => left == right,
            _ => false,
        }
    }
}

impl<T: CanonicalTarget> Eq for TransientRef<T> {}

/// Working state that has not crossed the integrity boundary: design § 23–24.
///
/// No `revision` and no `ontology`: a transient root is not revisioned — design § 35.2 reclaims
/// one outright rather than retracting from it — and its contents are candidates that have not
/// been type-checked against a schema version yet.
#[derive(Clone, Debug, PartialEq)]
pub struct TransientGraph {
    /// The root this state hangs off. Its `space` is [`Space::Transient`](crate::Space).
    pub root: GraphRoot,
    /// Its candidate nodes, by id.
    pub nodes: BTreeMap<NodeId, Node>,
    /// Its candidate edges, by id.
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Its candidate assertions, by id.
    pub assertions: BTreeMap<AssertionId, Assertion>,
}

impl TransientGraph {
    /// The node a transient reference points at, looking in whichever space the reference names.
    ///
    /// Taking the canonical graph as an argument is the permitted direction of the membrane made
    /// explicit: transient state reads canonical state, and needs a borrow of it to do so. There
    /// is no method on [`CanonicalGraph`] that takes a `TransientGraph`, and
    /// `tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs` holds that there is no way
    /// to write one.
    #[must_use]
    pub fn resolve<'a, T: CanonicalTarget>(
        &'a self,
        reference: &TransientRef<T>,
        canonical: &'a CanonicalGraph,
    ) -> Option<&'a Node> {
        match reference {
            TransientRef::Canonical(reference) => canonical.resolve(reference),
            TransientRef::Local(reference) => self.nodes.get(&reference.node()),
        }
    }
}
