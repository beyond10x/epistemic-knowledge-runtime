//! Adversary, wave p1-06 pass 1: the serde boundary ADR 0008 says it states rather than hides.
//!
//! Two documents this unit wrote say the same thing, and it is the sentence this case reads.
//!
//! `crates/ekr-graph/src/canonical.rs:25-29`:
//!
//! > a [`CanonicalRef`] writes and reads the bare id it wraps, so a document deserialises into
//! > whatever type the caller names and **the kernel's reference validator is what holds the edge
//! > where no type can**.
//!
//! `task:canonical-state-references-are-typed` § Tests, third bullet:
//!
//! > the serde boundary stated rather than hidden: a document deserialises into whatever type the
//! > caller names, so **the kernel's reference validator keeps its refusal for ids arriving from
//! > outside Rust**.
//!
//! This binary is the path ids actually arrive on. It links `ekr-core`, `ekr-graph`,
//! `ekr-ontology` and `ekr-store`; it cannot link `ekr-kernel`, because `ekr-store` sits below it,
//! so no reference validator can run on this crossing — not in this process and not in any other,
//! since `GraphDocument::into_canonical` is called from `EventlogStore::fold_from` and nothing
//! above it is consulted.

#[allow(dead_code)]
mod fixture;

use ekr_core::NodeId;
use ekr_store::GraphDocument;

/// A canonical reference minted from bytes, pointing at a node the document does not carry, with
/// nothing on the path that could refuse it.
///
/// The document is serialised, its edge's `target` is replaced with an id nothing holds, and the
/// bytes are read back — which is what "arriving from outside Rust" means. After ADR 0008 the id
/// lands in a `CanonicalRef<Node>`, the type whose own module says canonical state may depend on
/// it. The crossing converts it and returns canonical state.
///
/// **The sentence was wrong and has been corrected; the behaviour has not, and this case now
/// asserts the behaviour.** The kernel's reference validator guards the *transaction* path and is
/// not reachable from the document path, so "keeps its refusal for ids arriving from outside Rust"
/// named a mechanism that is not on that path. ADR 0008 is narrowed to the transaction path by the
/// coordinator, and `ekr_graph::canonical`'s header, `CanonicalRef`'s `Serialize` doc,
/// `ValueSpace::node_ref`'s doc and `snapshot.rs`'s `narrow_edge` now say the seed path refuses
/// nothing.
///
/// A second validator *here* is not the fix — `snapshot.rs` § "What this crossing does not do"
/// says why one validator in the right place beats two in the wrong ones — and the fix is not this
/// wave's: `review-result:adversary-eventlog-store-pass-1`'s finding C, filed, held for the debt
/// wave, whose own case is `review_p1_the_seed_is_validated_by_nobody.rs`.
///
/// So this asserts what is true today, and **goes red the day finding C closes**, which is when
/// somebody should read it again and delete it. Every step of the adversary's crossing is kept: the
/// bytes go out, the target is replaced with an id nothing holds, and the bytes come back.
#[test]
fn a_document_naming_an_edge_target_it_does_not_carry_becomes_canonical_state_unrefused() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let bytes = GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");

    // Read back as the transient instantiation, which is the only thing `from_bytes` produces,
    // and point the edge somewhere nothing holds. This is a document, not a canonical value: its
    // `target` here is a bare `NodeId` and no canonical type has been named yet.
    let mut document = GraphDocument::from_bytes(&bytes).expect("the document reads back");
    let stranger = NodeId::mint();
    for edge in document.edges.values_mut() {
        edge.target = stranger;
    }
    assert!(
        !document.nodes.contains_key(&stranger),
        "the id is one the document does not carry"
    );

    // Out to bytes and back in, so that what crosses is bytes and not a value this test held.
    let tampered = document
        .to_bytes()
        .expect("the tampered document serialises");
    let crossed = GraphDocument::from_bytes(&tampered).expect("the tampered document reads back");
    let outcome = crossed.into_canonical(ontology);

    let canonical = outcome.expect(
        "nothing on the document path refuses a dangling reference; the day this stops being true \
         is the day finding C closed and this case should be read again",
    );
    let edge = canonical.edges.values().next().expect("the edge crossed");
    assert!(
        canonical.resolve(&edge.target).is_none(),
        "the reference the crossing minted resolves to nothing, which is what makes it dangling"
    );
    assert_eq!(
        edge.target.node(),
        stranger,
        "a document whose edge targets {stranger}, a node it does not carry, crossed \
         GraphDocument::into_canonical and became canonical state holding a CanonicalRef<Node> to \
         it, with nothing on the path that could refuse it: ekr-store sits below ekr-kernel, so the \
         reference validator is not reachable here in any process. \
         review-result:adversary-eventlog-store-pass-1's finding C is what closes this"
    );
}
