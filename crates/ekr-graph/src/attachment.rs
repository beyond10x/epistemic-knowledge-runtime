//! Evidence attached to an assertion after the assertion was added: `ekr.graph.EvidenceAttachment`
//! (`story:evidence-attaches-to-a-held-assertion`, design § 103).
//!
//! An assertion cites the evidence it was added with, and that set is part of the assertion's
//! own encoding. Evidence found later — the exact message a claim rests on, where the claim first
//! cited a whole file — is not an edit of the assertion: it is a record of its own, kept by
//! [`CanonicalGraph`](crate::CanonicalGraph) beside the assertions, saying which evidence was
//! attached to which assertion at which revision. The assertion keeps its bytes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AssertionId, RevisionNumber};
use serde::{Deserialize, Serialize};

use crate::canonical::CanonicalRef;
use crate::evidence::Evidence;

/// One piece of evidence attached to an assertion: the evidence, and the revision whose commit
/// attached it. Filed under the assertion's id in [`Attachments`].
///
/// Ordered by evidence first, so the set an assertion holds lists its attachments in evidence-id
/// order, and an assertion holds a piece of evidence once — validation refuses a second
/// attachment of it (`evidence-already-attached`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachedEvidence {
    /// The evidence attached: retained evidence of the graph.
    pub evidence: CanonicalRef<Evidence>,
    /// The revision the attaching commit produced.
    pub revision: RevisionNumber,
}

impl Canonical for AttachedEvidence {
    /// The two fields in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.evidence.encode(out);
        self.revision.encode(out);
    }
}

/// Every attachment of a graph: each assertion that has evidence attached, by its id, with the set
/// of what was attached to it. An assertion with none has no entry.
pub type Attachments = BTreeMap<AssertionId, BTreeSet<AttachedEvidence>>;
