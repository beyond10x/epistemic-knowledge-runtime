//! Adversary, wave p1-14 unit p1-14-exit.
//!
//! `crates/ekr-graph/src/lib.rs:35`: every kind of reference canonical state holds is typed. The
//! competing assertions of a canonical, disputed assertion are references canonical state holds.
//! This writes a candidate assertion's id — a `LocalRef` into a transient root — into them. If the
//! claim held, this would not compile.

use ekr_core::AssertionId;
use ekr_graph::{Assertion, Assessment, CanonicalGraph, LocalRef};

fn disputed_by_a_candidate(
    canonical: &mut CanonicalGraph,
    held: AssertionId,
    candidate: LocalRef<Assertion>,
) {
    canonical.assertions.get_mut(&held).unwrap().assessment = Assessment::Disputed {
        competing_assertions: vec![candidate.id()],
    };
}

fn main() {}
