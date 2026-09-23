//! Adversary, wave p1-14 unit p1-14-exit.
//!
//! `crates/ekr-graph/src/lib.rs:35`: every kind of reference canonical state holds is typed.
//! Retained evidence whose source is another assertion of the graph holds a reference to that
//! assertion, and retained evidence is canonical state (`CanonicalGraph::evidence`). This writes a
//! candidate assertion's id — a `LocalRef` into a transient root — into it. If the claim held, this
//! would not compile.

use ekr_core::EvidenceId;
use ekr_graph::{Assertion, CanonicalGraph, EvidenceSource, LocalRef};

fn retained_evidence_citing_a_candidate(
    canonical: &mut CanonicalGraph,
    retained: EvidenceId,
    candidate: LocalRef<Assertion>,
) {
    canonical.evidence.get_mut(&retained).unwrap().source =
        EvidenceSource::GraphAssertion(candidate.id());
}

fn main() {}
