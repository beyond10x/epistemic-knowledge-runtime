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
use ekr_ontology::Value;

use crate::assertion::Assertion;
use crate::canonical::{CanonicalGraph, CanonicalRef, CanonicalTarget};
use crate::edge::Edge;
use crate::node::Node;
use crate::root::GraphRoot;

/// A reference to something inside one transient root: design § 23.
///
/// Local to the root that holds it. It is deliberately *not* a
/// [`CanonicalDependency`](crate::CanonicalDependency): a candidate that has not been integrated
/// is not something canonical state may rest on.
pub struct LocalRef<T: CanonicalTarget> {
    id: T::Id,
    marker: PhantomData<fn() -> T>,
}

impl<T: CanonicalTarget> LocalRef<T> {
    /// A reference to the candidate of kind `T` with `id`, within the transient root that holds it.
    #[must_use]
    pub const fn new(id: T::Id) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }

    /// The id of the candidate it points at.
    #[must_use]
    pub const fn id(&self) -> T::Id {
        self.id
    }
}

impl LocalRef<Node> {
    /// The candidate node it points at.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.id
    }
}

impl<T: CanonicalTarget> std::fmt::Debug for LocalRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalRef").field("id", &self.id).finish()
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
        self.id == other.id
    }
}

impl<T: CanonicalTarget> Eq for LocalRef<T> {}

// No `sealed::Sealed` for `LocalRef`. It carried the seal of [`CanonicalDependency`] without
// implementing the trait, which nothing bounded on — and `canonical.rs` says why that is a defect
// rather than a spare line: "the compile-fail cases exist to be *read*, so the message they pin
// has to be true", and rustc prints the *marker*'s implementors in that message. A reader of
// `tests/compile_fail/canonical_dependency_is_sealed.stderr` was told a local reference is a
// canonical dependency. A candidate that has not been integrated is not something canonical state
// may rest on, which is the whole of design § 23.

/// A reference transient state may hold: design § 23.
///
/// The union of the two directions that are allowed — a candidate of its own root, or something
/// canonical it has resolved to. There is no third arm and no equivalent type on the canonical
/// side, which is the asymmetry the membrane *is*.
pub enum TransientRef<T: CanonicalTarget> {
    /// Something canonical state already holds.
    Canonical(CanonicalRef<T>),
    /// A candidate of this transient root.
    Local(LocalRef<T>),
}

impl<T: CanonicalTarget> TransientRef<T> {
    /// The id of what it points at, whichever side it points into.
    #[must_use]
    pub const fn id(&self) -> T::Id {
        match self {
            Self::Canonical(reference) => reference.id(),
            Self::Local(reference) => reference.id(),
        }
    }
}

impl TransientRef<Node> {
    /// The node it points at, whichever side it points into.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.id()
    }
}

impl<T: CanonicalTarget> std::fmt::Debug for TransientRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canonical(reference) => f.debug_tuple("Canonical").field(reference).finish(),
            Self::Local(reference) => f.debug_tuple("Local").field(reference).finish(),
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
///
/// # Its candidates carry `ekr_ontology::Value`, floats and all
///
/// The same three types canonical state holds, over a different value:
/// `architecture-decision-record:0005-float-is-not-canonical` as amended after adversary pass 1.
/// A float is not admissible in canonical state because it has no encoding both total and
/// faithful to equality — and nothing here is content-addressed, so that constraint has nothing
/// to buy on this side. An imported approximate measurement is exactly the candidate an
/// incubation root exists to hold, and refusing it here would invert what the root is for.
///
/// Nothing in this struct has a [`Canonical`](ekr_core::canonical::Canonical) implementation, and
/// that is the guarantee rather than an omission: `Node<Value>`, `Edge<Value>` and
/// `Assertion<Value>` are outside the bound those implementations carry, so a content address of
/// transient state does not compile.
/// `tests/compile_fail/transient_state_has_no_content_address.rs` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct TransientGraph {
    /// The root this state hangs off. Its `space` is [`Space::Transient`](crate::Space).
    pub root: GraphRoot,
    /// Its candidate nodes, by id.
    pub nodes: BTreeMap<NodeId, Node<Value>>,
    /// Its candidate edges, by id.
    pub edges: BTreeMap<EdgeId, Edge<Value>>,
    /// Its candidate assertions, by id.
    pub assertions: BTreeMap<AssertionId, Assertion<Value>>,
}

impl TransientGraph {
    /// The node a transient reference points at, looking in whichever space the reference names.
    ///
    /// Taking the canonical graph as an argument is the permitted direction of the membrane made
    /// explicit: transient state reads canonical state, and needs a borrow of it to do so. There
    /// is no method on [`CanonicalGraph`] that takes a `TransientGraph`, and
    /// `tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs` holds that there is no way
    /// to write one.
    ///
    /// The answer is a [`Resolved`] and not a node, because the two sides are no longer one type:
    /// a canonical node carries values canonical state admits and a candidate carries any value
    /// at all. Which side a reference landed on is part of the answer rather than something the
    /// caller has to remember it asked for.
    ///
    /// **Node references only**, since wave p1-14: a reference now holds the id of its kind, and
    /// [`Resolved`] answers nodes. It used to accept a `TransientRef<T>` of any kind and look its
    /// id up among nodes on both sides, which is the defect
    /// `task:canonical-reference-holds-a-node-id-for-every-target` closed on the canonical side.
    #[must_use]
    pub fn resolve<'a>(
        &'a self,
        reference: &TransientRef<Node>,
        canonical: &'a CanonicalGraph,
    ) -> Option<Resolved<'a>> {
        match reference {
            TransientRef::Canonical(reference) => {
                canonical.resolve(reference).map(Resolved::Canonical)
            }
            TransientRef::Local(reference) => {
                self.nodes.get(&reference.node()).map(Resolved::Local)
            }
        }
    }
}

/// What a transient reference resolved to, and which side of the membrane it came from.
///
/// The two are different types, not one: canonical state holds `Node<CanonicalValue>` and a
/// transient root holds `Node<ekr_ontology::Value>`
/// (`architecture-decision-record:0005-float-is-not-canonical`, as amended). A caller matches on
/// it, which is the membrane showing through a return type.
///
/// # There is no `id()` on this
///
/// It had one, answering the node's id from either arm on the grounds that "an id is an id on both
/// sides of the membrane". [`TransientRef`]'s [`PartialEq`] says the opposite about the same fact,
/// and it is right: it compares by what is pointed at **and** which side it is on, because a
/// candidate that happens to carry the same id as a canonical node is not that node. An accessor
/// that answers from both arms lets a caller drop the one distinction this type exists to carry —
/// and every caller in the suite did exactly that, so nothing asserted which side came back.
///
/// `crates/ekr-graph/tests/membrane.rs` now asserts the variant, over a candidate and a canonical
/// node that share an id.
#[derive(Debug, PartialEq)]
pub enum Resolved<'a> {
    /// A node of canonical state, reached through the permitted direction.
    Canonical(&'a Node),
    /// A candidate of this transient root.
    Local(&'a Node<Value>),
}
