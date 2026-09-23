//! Adversary, wave p1-14 unit p1-14-exit: the two fields this unit typed keep their frozen bytes and
//! their serde wire shape.
//!
//! `crates/ekr-graph/tests/current_vectors.rs` pins an assertion whose subject is a *node* and whose
//! evidence set has one member, so neither the edge arm of `Subject` nor the order of a many-member
//! evidence set is held by a vector. These compare the typed fields against
//! `ekr_graph::legacy` — frozen from commit 73ab8b0, where both were bare ids — over many ids.
//! What they assert: the canonical bytes and the JSON of `Subject::Edge(CanonicalRef<Edge>)` equal
//! the frozen `Subject::Edge(EdgeId)`'s, and one decodes from the other; a set of
//! `CanonicalRef<Evidence>` encodes, serialises and iterates exactly as the set of `EvidenceId`s it
//! replaced; and a canonical assertion carrying one evidence reference twice is still refused.

use std::collections::BTreeSet;

use ekr_core::canonical::Canonical;
use ekr_core::{
    AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, Timestamp,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, CanonicalValue, Edge, Evidence,
    Object, Predicate, Subject, TemporalRange, TransactionTime,
};

#[test]
fn a_typed_edge_subject_keeps_the_frozen_bytes_and_wire_shape() {
    for _ in 0..256 {
        let edge = EdgeId::mint();
        let frozen = ekr_graph::legacy::Subject::<NodeId>::Edge(edge);
        let typed: Subject = Subject::Edge(CanonicalRef::<Edge>::new(edge));
        assert_eq!(typed.canonical_bytes(), frozen.canonical_bytes(), "{edge}");
        let frozen_json = serde_json::to_string(&frozen).unwrap();
        assert_eq!(
            serde_json::to_string(&typed).unwrap(),
            frozen_json,
            "{edge}"
        );
        let decoded: Subject = serde_json::from_str(&frozen_json).unwrap();
        assert_eq!(decoded, typed, "{edge}");
    }
}

#[test]
fn a_typed_evidence_set_keeps_the_frozen_bytes_order_and_wire_shape() {
    for size in [0usize, 1, 2, 3, 17, 64] {
        let bare: BTreeSet<EvidenceId> = (0..size).map(|_| EvidenceId::mint()).collect();
        let typed: BTreeSet<CanonicalRef<Evidence>> =
            bare.iter().copied().map(CanonicalRef::new).collect();
        assert_eq!(
            typed.canonical_bytes(),
            bare.canonical_bytes(),
            "size {size}"
        );
        assert_eq!(
            serde_json::to_string(&typed).unwrap(),
            serde_json::to_string(&bare).unwrap(),
            "size {size}"
        );
        assert!(
            typed
                .iter()
                .map(|cited| cited.id())
                .eq(bare.iter().copied()),
            "size {size}: iteration order moved"
        );
    }
}

#[test]
fn a_canonical_assertion_citing_one_evidence_twice_is_still_refused_on_decode() {
    let cited = EvidenceId::mint();
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: GraphRootId::mint(),
        subject: Subject::Edge(CanonicalRef::new(EdgeId::mint())),
        predicate: Predicate::Property(PropertyId::mint()),
        object: Object::Value(CanonicalValue::String("x".into())),
        evidence: BTreeSet::from([CanonicalRef::new(cited)]),
        proposed_by: AgentId::mint(),
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let json = serde_json::to_string(&assertion).unwrap();
    let back: Assertion = serde_json::from_str(&json).unwrap();
    assert_eq!(back, assertion);
    let once = format!("[\"{cited}\"]");
    assert!(json.contains(&once), "{json}");
    let twice = json.replace(&once, &format!("[\"{cited}\",\"{cited}\"]"));
    assert!(
        serde_json::from_str::<Assertion>(&twice).is_err(),
        "a duplicate evidence reference decoded: {twice}"
    );
}
