//! AGENTS.md invariant 2, for the evidence a canonical assertion rests on.
//!
//! "Canonical knowledge depends only on canonical knowledge or retained admissible evidence."
//! `Assertion.evidence` was a set of bare `EvidenceId`s, so the one dependency the invariant names
//! by kind was the one that was not typed. In canonical state it is a set of
//! `CanonicalRef<Evidence>`: neither a bare id nor a transient reference to evidence inhabits it.

use ekr_core::EvidenceId;
use ekr_graph::{Assertion, Evidence, TransientRef};

fn cite_a_bare_evidence_id(assertion: &mut Assertion, evidence: EvidenceId) {
    assertion.evidence.insert(evidence);
}

fn cite_transient_evidence(assertion: &mut Assertion, evidence: TransientRef<Evidence>) {
    assertion.evidence.insert(evidence);
}

fn main() {}
