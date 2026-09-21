//! Canonical state, and the references it is allowed to hold.
//!
//! Design § 21 and § 23. The one-line version of both, which is AGENTS.md invariant 2:
//!
//! > Canonical knowledge depends only on canonical knowledge or retained admissible evidence. A
//! > `Canonical → Transient` reference is **unrepresentable at the type level**, not merely
//! > refused.
//!
//! That is what [`CanonicalDependency`] is for. It is sealed, so the set of references canonical
//! state may hold is closed at this crate: a crate above cannot answer a bound it does not satisfy
//! by implementing its way past it. `crates/ekr-graph/tests/compile_fail/` holds both halves as
//! build failures.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use ekr_core::{AssertionId, EdgeId, EvidenceId, NodeId, RevisionNumber};
use ekr_ontology::Ontology;

use crate::assertion::Assertion;
use crate::edge::Edge;
use crate::evidence::{Evidence, Observation, Support};
use crate::node::Node;
use crate::root::GraphRoot;

pub(crate) mod sealed {
    /// Implemented for the reference types of this crate and closed to every other crate.
    pub trait Sealed {}

    /// Implemented for the canonical entities of this crate and closed to every other crate.
    ///
    /// A second marker rather than a reuse of [`Sealed`]: one marker shared between the two
    /// public traits made rustc's own "the following types implement the trait" list on a sealing
    /// error name the entities as implementors of `CanonicalDependency`, which they are not. The
    /// compile-fail cases exist to be *read*, so the message they pin has to be true.
    pub trait SealedTarget {}
}

/// A kind of thing a canonical reference may point at.
///
/// The `T` of [`CanonicalRef`] is a marker with no data, and its whole job is to keep a reference
/// to one kind of thing out of a slot that wants another. Left unbounded, it did not do that job
/// at the one place it matters: `CanonicalRef<TransientRef<Node>>` was a well-formed type and a
/// [`CanonicalDependency`], so canonical state would accept a reference *to a transient
/// reference*. Nothing in the workspace builds one — but AGENTS.md invariant 2 says the crossing
/// is "unrepresentable at the type level, not merely refused", and "no caller does it yet" is not
/// that.
///
/// Sealed and implemented for the canonical entities of this crate. [`TransientRef`](crate::TransientRef),
/// [`LocalRef`](crate::LocalRef) and [`TransientGraph`](crate::TransientGraph) are not among them
/// and cannot be added from outside.
pub trait CanonicalTarget: sealed::SealedTarget {}

macro_rules! canonical_target {
    ($($type:ty),+ $(,)?) => {
        $(
            impl sealed::SealedTarget for $type {}
            impl CanonicalTarget for $type {}
        )+
    };
}

canonical_target!(
    Node,
    Edge,
    Assertion,
    Evidence,
    Observation,
    Support,
    GraphRoot
);

/// A reference canonical state is allowed to depend on.
///
/// Implemented for [`CanonicalRef`] and for nothing else, and sealed so that it stays that way.
/// A [`TransientRef`](crate::TransientRef) is deliberately absent: design § 23 says "no equivalent
/// `CanonicalGraph -> TransientRef` should exist", and a bound is how that is said to the
/// compiler rather than to a reviewer.
pub trait CanonicalDependency: sealed::Sealed {
    /// The node the reference points at.
    fn node(&self) -> NodeId;
}

/// A reference into canonical state, parameterised by what it points at.
///
/// Design § 23. `T` is a marker and carries no data: it is what keeps a reference to one kind of
/// thing out of a slot that wants another, at compile time. The [`PhantomData`] is
/// `fn() -> T` rather than `T`, so the reference is [`Copy`], [`Send`] and [`Sync`] whatever `T`
/// is — a reference is an id, and an id has no ownership of the thing it names.
#[derive(Debug)]
pub struct CanonicalRef<T: CanonicalTarget> {
    node: NodeId,
    marker: PhantomData<fn() -> T>,
}

impl<T: CanonicalTarget> CanonicalRef<T> {
    /// A reference to `node`.
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

// Implemented by hand rather than derived: `derive` would add a `T: Clone` bound, and `T` is a
// marker that is never held.
impl<T: CanonicalTarget> Clone for CanonicalRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: CanonicalTarget> Copy for CanonicalRef<T> {}

impl<T: CanonicalTarget> PartialEq for CanonicalRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T: CanonicalTarget> Eq for CanonicalRef<T> {}

impl<T: CanonicalTarget> sealed::Sealed for CanonicalRef<T> {}

impl<T: CanonicalTarget> CanonicalDependency for CanonicalRef<T> {
    fn node(&self) -> NodeId {
        self.node
    }
}

/// Integrated knowledge that has crossed the system's highest integrity boundary: design § 21.
///
/// Strongly typed, schema valid, referentially complete, provenance compliant, transactionally
/// committed, revisioned. This crate holds the shape; nothing here puts anything into it — only a
/// `ValidatedTransaction` the kernel built may, which is AGENTS.md invariant 1.
///
/// Design § 21 writes `revision: u64` and § 23 writes `root: GraphRoot`; both are here, because
/// the root says which space and schema version the state belongs to and the revision says which
/// point in the lineage it is.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalGraph {
    /// The root this state hangs off.
    pub root: GraphRoot,
    /// Its position in the revision lineage.
    pub revision: RevisionNumber,
    /// The schema its contents are valid against.
    pub ontology: Ontology,
    /// Its nodes, by id.
    pub nodes: BTreeMap<NodeId, Node>,
    /// Its edges, by id.
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Its assertions, by id.
    pub assertions: BTreeMap<AssertionId, Assertion>,
    /// The retained evidence its assertions rest on, by id.
    pub evidence: BTreeMap<EvidenceId, Evidence>,
}

impl CanonicalGraph {
    /// The node a canonical reference points at, or `None` if this state does not hold it.
    ///
    /// The bound is the membrane: a [`TransientRef`](crate::TransientRef) is not a
    /// [`CanonicalDependency`], so this does not compile for one. A dangling canonical reference
    /// is a different thing from a forbidden one, and is answered with `None` — the reference
    /// validator of design § 20 is what refuses it at commit time.
    #[must_use]
    pub fn resolve<R: CanonicalDependency>(&self, reference: &R) -> Option<&Node> {
        self.nodes.get(&reference.node())
    }
}
