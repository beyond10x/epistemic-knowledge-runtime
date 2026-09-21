//! Snapshot semantics: what a reader of canonical state actually holds. Design § 71.

use ekr_core::{RevisionNumber, Timestamp};

use crate::assertion::Assertion;
use crate::canonical::CanonicalGraph;

/// An immutable read of canonical state, at one revision: design § 71.
///
/// A borrow rather than a copy, which is what makes reads consistent, validation deterministic and
/// a stale commit detectable: a transaction records the revision it was validated against, and
/// design § 72's commit attempt compares that number to the current one.
///
/// P1's snapshot answers the one read design § 65's example needs and nothing else. The
/// `QueryScope` selector of design § 47 — canonical, transient, historical, with or without
/// evidence — is P4.
#[derive(Copy, Clone, Debug)]
pub struct GraphSnapshot<'a> {
    graph: &'a CanonicalGraph,
    revision: RevisionNumber,
}

impl<'a> GraphSnapshot<'a> {
    /// A snapshot of `graph`, at the revision the graph is at.
    #[must_use]
    pub const fn of(graph: &'a CanonicalGraph) -> Self {
        Self {
            graph,
            revision: graph.revision,
        }
    }

    /// The revision this snapshot reads.
    #[must_use]
    pub const fn revision(&self) -> RevisionNumber {
        self.revision
    }

    /// The state it reads.
    #[must_use]
    pub const fn graph(&self) -> &'a CanonicalGraph {
        self.graph
    }

    /// The assertions canonical state holds to have been true at `at`. **The only read.**
    ///
    /// One function with two roles, which is what bitemporality means. Asked at the caller's now
    /// it is the current-world query of design § 65 — "the current-world query returns Bob".
    /// Asked at any other instant it is the historical query — "historical query remains
    /// possible". There is nothing else to build: the runtime has no clock, so it cannot supply
    /// the "now" itself, and the caller that has one gets both reads from this.
    ///
    /// # There is no `active()`
    ///
    /// There was, and it filtered [`Assertion::is_current`] together with a valid time that had
    /// no *known* end. That is the same read as `valid_at(i64::MAX)` — measured, over records of
    /// every shape — so its "current world" was the world at the end of representable time, and
    /// two ordinary facts fell on the wrong side of it: a fixed-term appointment that is true
    /// today was excluded because its end is known, and an announced successor whose tenure has
    /// not begun was included. A public read whose name asserts something the runtime cannot know
    /// is worse than no read, so it was removed rather than renamed.
    /// `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` holds that none comes back.
    ///
    /// # What it filters
    ///
    /// [`Assertion::is_current`] — accepted, and still believed — and a valid time containing
    /// `at`. Valid time is half-open, so an assertion that ends at `at` does not answer and one
    /// that begins at `at` does, which is what keeps § 65's handover instant from returning two
    /// chief executives.
    ///
    /// A retracted or superseded record does not become answerable by being asked about in the
    /// past. Design § 34 makes *that* reconstructable by replaying the lineage to the revision in
    /// question, which is a different read from this one.
    #[must_use]
    pub fn valid_at(&self, at: Timestamp) -> Vec<&'a Assertion> {
        self.graph
            .assertions
            .values()
            .filter(|assertion| assertion.is_current() && assertion.valid_time.contains(at))
            .collect()
    }
}
