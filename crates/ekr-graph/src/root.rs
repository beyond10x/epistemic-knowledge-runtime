//! Roots: the graph root a space is rooted at (design § 22–23) and the immutable revision root a
//! commit produces (design § 34).

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{ContentHash, GraphRootId, RevisionNumber, SchemaVersionId, Timestamp};
use serde::{Deserialize, Serialize};

/// Which knowledge space a root belongs to: `ekr.graph.Space`, design § 23.
///
/// The type-level version of the same distinction is [`CanonicalGraph`](crate::CanonicalGraph)
/// against [`TransientGraph`](crate::TransientGraph); this field is what a stored record carries.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Space {
    /// Integrated knowledge that has crossed the highest integrity boundary.
    Canonical,
    /// Working state that has not, and may never.
    Transient,
}

/// The root of one graph: `ekr.graph.GraphRoot`, design § 22–23 and § 69.
///
/// In P1 there is exactly one, the canonical core. Design § 29's `KnowledgeState` ladder is not a
/// field here: it arrives in P3 with the commands that move a root, and a state nothing can move
/// is a state that lies.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRoot {
    /// Its stable id — `root_id` in the domain, which names an identity per entity.
    pub id: GraphRootId,
    /// The space it belongs to.
    pub space: Space,
    /// The schema version its contents are typed by.
    pub schema_version_id: SchemaVersionId,
    /// The root it was derived from, if any.
    pub parent: Option<GraphRootId>,
    /// When it was created.
    pub created_at: Timestamp,
}

/// The immutable root a successful commit produces: design § 34.
///
/// Four sub-roots rather than one, because they are reclaimed and replayed on different schedules:
/// design § 37 gives ontology, knowledge, evidence and agent state different storage classes, and
/// a single hash over all of them would make a retention sweep of evidence look like a change to
/// knowledge.
///
/// `parent` is the *hash* of the previous root rather than its number, which is what makes the
/// lineage a chain a reader can verify rather than a sequence a writer asserts.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Root {
    /// Its position in the lineage: zero at the seed, one per commit.
    pub revision: RevisionNumber,
    /// The hash of the root this one follows, or `None` at the seed.
    pub parent: Option<ContentHash>,
    /// The ontology state at this revision.
    pub ontology_root: ContentHash,
    /// The graph state at this revision.
    pub knowledge_root: ContentHash,
    /// The retained evidence at this revision.
    pub evidence_root: ContentHash,
    /// The agent registry at this revision.
    pub agent_root: ContentHash,
    /// The transaction that produced it.
    pub transaction: ContentHash,
}

impl Canonical for Root {
    /// The seven fields in declaration order.
    ///
    /// Structural, with no discriminant: a `Root` is not a sum type, and rule 5 of
    /// `ekr_core::canonical` says a composite value's own field structure is what distinguishes
    /// it. The order is the contract — moving a field moves every revision address ever recorded.
    fn encode(&self, out: &mut Encoder) {
        self.revision.encode(out);
        out.option(self.parent.as_ref());
        self.ontology_root.encode(out);
        self.knowledge_root.encode(out);
        self.evidence_root.encode(out);
        self.agent_root.encode(out);
        self.transaction.encode(out);
    }
}
