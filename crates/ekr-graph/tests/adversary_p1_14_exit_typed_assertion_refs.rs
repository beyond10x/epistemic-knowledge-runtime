//! Adversary, wave p1-14 unit p1-14-exit.
//!
//! `crates/ekr-graph/src/lib.rs:35-39`, written by this unit:
//!
//! > Every kind of reference canonical state holds goes through that machinery, since wave p1-14
//! > [...] so neither a bare id nor a [`TransientRef`] inhabits them. One compile-fail case per
//! > kind holds it.
//!
//! Canonical state holds three references to assertions that are bare `AssertionId`s:
//! `AssertionLifecycle::Superseded.by`, `Assessment::Disputed.competing_assertions` and
//! `EvidenceSource::GraphAssertion`. Each case in `tests/adversary_p1_14_exit_compile_fail/` writes a
//! candidate assertion's `LocalRef` id into one of them inside a `CanonicalGraph`. What this asserts
//! after the fix: none of the three compiles. Today all three do, so this is red.

#[test]
fn every_assertion_reference_canonical_state_holds_refuses_a_transient_identity() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/adversary_p1_14_exit_compile_fail/*.rs");
}
