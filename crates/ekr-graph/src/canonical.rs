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
//!
//! # And canonical state is made of them
//!
//! `architecture-decision-record:0008-canonical-state-references-are-typed`. The sealing above was
//! sound and had no callers: every reference *inside* [`CanonicalGraph`] was a bare
//! [`NodeId`], so the crossing the invariant calls unrepresentable was written with an id,
//! compiled, and was refused by the kernel at commit time — which is the word the sentence
//! excludes. [`ValueSpace`] is what gives the machinery its callers: the value a graph's entities
//! carry already says which side of the membrane the state is on, and this trait maps that value to
//! the reference that side may hold. [`Edge`], [`Assertion`], [`Subject`](crate::Subject),
//! [`Object`](crate::Object) and [`CanonicalValue`] reference through it.
//!
//! # The serde boundary, and what is actually on the other side of it
//!
//! A [`CanonicalRef`] writes and reads the bare id it wraps, so the wire shape is unchanged and a
//! document deserialises into whatever type the caller names. **The type holds inside Rust and
//! nothing else does.** Two paths reach canonical state from bytes, and they are not the same:
//!
//! * the **transaction** path — a proposal's ids are resolved by `ekr-kernel`'s reference
//!   validator before `canonical_assertion` mints a reference from one, so a dangling id is refused
//!   there;
//! * the **seed** path — the kernel validates a versioned bootstrap document and narrows its
//!   references only after the deterministic rules pass. Store replay delegates admission to
//!   that same kernel authority, including after restart. A store without it refuses the seed.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AssertionId, EdgeId, EvidenceId, NodeId, RevisionNumber};
use ekr_ontology::Ontology;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::assertion::Assertion;
use crate::edge::Edge;
use crate::evidence::{Evidence, Observation, Support};
use crate::node::Node;
use crate::root::GraphRoot;
use crate::value::CanonicalValue;

pub(crate) mod sealed {
    /// Implemented for the reference types of this crate and closed to every other crate.
    pub trait Sealed {}

    /// Implemented for the two values this crate's entities may carry, and closed to every other
    /// crate.
    ///
    /// A third marker, for the reason [`SealedTarget`] is a second one: the implementors rustc
    /// lists on a sealing error have to be the implementors of the trait the reader was bounded
    /// on.
    pub trait SealedSpace {}

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
///
/// # And the marker does not do that job yet
///
/// `task:canonical-reference-holds-a-node-id-for-every-target`. [`CanonicalRef<T>`] stores a
/// [`NodeId`] for **all seven** of the types below, and [`CanonicalGraph::resolve`] looks it up in
/// the `nodes` map — so a `CanonicalRef<Evidence>` answers a node. Exactly one of the seven,
/// `Node`, has a caller; the other six are available and wrong. Wave p1-06 made this marker the
/// bound on five implementations canonical state is now made of, which is why the task exists and
/// why it is worth more than it was. `crates/ekr-graph/tests/adversary_p1_06_reference_markers.rs`
/// is that measured.
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

/// The side of the membrane a graph's entities are on, and therefore the reference they hold.
///
/// `architecture-decision-record:0008-canonical-state-references-are-typed`.
/// [`Node`], [`Edge`] and [`Assertion`] are generic over the value they carry
/// (`architecture-decision-record:0005-float-is-not-canonical`, as amended), and that parameter
/// already says which space the state belongs to: canonical state holds [`CanonicalValue`], a
/// transient root holds [`ekr_ontology::Value`]. This trait is that fact made usable — it names,
/// for each of the two, the reference a node-valued field of that space may carry.
///
/// * [`CanonicalValue`] → [`CanonicalRef<Node>`]. Design § 23: canonical state depends only on
///   canonical state, so there is no way to write an [`Edge`] of it whose target is a candidate's
///   id. `tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs` is that as
///   a build failure.
/// * [`ekr_ontology::Value`] → [`NodeId`]. A transient root may point either way (§ 23–24) and its
///   contents are candidates; the incubation forest that resolves them is P3, and ADR 0008 leaves
///   the transient instantiation as it was rather than inventing its reference type a phase early.
///
/// Sealed. A crate above this one cannot add a third space, which is what keeps the set of
/// references canonical state may hold closed at this crate.
pub trait ValueSpace: sealed::SealedSpace {
    /// The reference a node-valued field carries in this space.
    type NodeRef: Canonical
        + Copy
        + std::fmt::Debug
        + Eq
        + Ord
        + Hash
        + Serialize
        + serde::de::DeserializeOwned;

    /// This space's reference to `node`.
    ///
    /// **Minting a reference is not resolving one**, on either side. It says which space the
    /// reference belongs to and nothing about whether that space holds the node — which is
    /// [`CanonicalGraph::resolve`]'s question, and, on the *transaction* path, `ekr-kernel`'s
    /// reference validator's on both the transaction and bootstrap paths.
    ///
    /// It exists because a caller generic over the space has no other way to build one: the
    /// concrete callers write [`CanonicalRef::new`] or the id itself. `ekr-kernel`'s
    /// `tests/validate_properties.rs` builds one proposal generator for both spaces and is what
    /// needs it.
    fn node_ref(node: NodeId) -> Self::NodeRef;
}

impl sealed::SealedSpace for CanonicalValue {}

impl ValueSpace for CanonicalValue {
    type NodeRef = CanonicalRef<Node>;

    // Spelled out rather than `Self::NodeRef`, which is ambiguous here: `CanonicalValue` has a
    // variant of that name, and it is the one this associated type exists to have parameterised.
    fn node_ref(node: NodeId) -> CanonicalRef<Node> {
        CanonicalRef::new(node)
    }
}

impl sealed::SealedSpace for ekr_ontology::Value {}

impl ValueSpace for ekr_ontology::Value {
    type NodeRef = NodeId;

    fn node_ref(node: NodeId) -> NodeId {
        node
    }
}

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

// Ordered and hashed by what is pointed at, by hand for the reason [`Clone`] is: `derive` would
// bound `T`, and `T` is a marker that is never held. Both exist because a reference is a field of
// [`Subject`](crate::Subject), which is a key in the domain's own sense — an assertion is looked
// up by what it is about.
impl<T: CanonicalTarget> PartialOrd for CanonicalRef<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: CanonicalTarget> Ord for CanonicalRef<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.node.cmp(&other.node)
    }
}

impl<T: CanonicalTarget> Hash for CanonicalRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

/// The id it wraps, and nothing else: the wire shape of canonical state is unchanged by this type
/// existing.
///
/// ADR 0008 states the serde boundary rather than hiding it. A stored document carries node ids,
/// and a reference read out of one is a reference the *caller* named the type of — so the
/// guarantee this type carries holds inside Rust and stops there. Which path the id arrived by
/// decides which validator resolves it. Both transaction and bootstrap admission belong to the
/// kernel; see this module's header.
impl<T: CanonicalTarget> Serialize for CanonicalRef<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.node.serialize(serializer)
    }
}

impl<'de, T: CanonicalTarget> Deserialize<'de> for CanonicalRef<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        NodeId::deserialize(deserializer).map(Self::new)
    }
}

impl<T: CanonicalTarget> Canonical for CanonicalRef<T> {
    /// The id, structurally: rule 5 of `ekr_core::canonical` makes a newtype structural, and this
    /// is one. **Every address that contained a `NodeId` before ADR 0008 contains the same bytes
    /// after it** — the marker is a [`PhantomData`] and is not encoded, because what a reference
    /// points at is the node and not the slot it sits in.
    fn encode(&self, out: &mut Encoder) {
        self.node.encode(out);
    }
}

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
