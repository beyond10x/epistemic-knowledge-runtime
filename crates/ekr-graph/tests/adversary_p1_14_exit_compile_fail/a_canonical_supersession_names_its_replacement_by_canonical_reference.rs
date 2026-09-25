//! Adversary, wave p1-14 unit p1-14-exit.
//!
//! `crates/ekr-graph/src/lib.rs:35` says every kind of reference canonical state holds goes through
//! the `CanonicalRef` machinery, so neither a bare id nor a transient reference inhabits it. The
//! replacing assertion of a canonical, superseded assertion is a reference canonical state holds.
//! This writes the id of a *candidate* assertion — a `LocalRef` into a transient root — into it.
//! If the claim held, this would not compile.

use ekr_core::{AssertionId, RevisionNumber, Timestamp};
use ekr_graph::{Assertion, AssertionLifecycle, CanonicalGraph, LocalRef};

fn superseded_by_a_candidate(
    canonical: &mut CanonicalGraph,
    held: AssertionId,
    candidate: LocalRef<Assertion>,
) {
    canonical.assertions.get_mut(&held).unwrap().lifecycle = AssertionLifecycle::Superseded {
        by: candidate.id(),
        at_revision: RevisionNumber::SEED,
        effective_from: Timestamp::EPOCH,
    };
}

fn main() {}
