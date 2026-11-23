//! The one integration-test binary of `ekr-observe`: every test file under `tests/` is a module of
//! it. A new test file joins with a `#[path]` line below, in name order; the `#[path]` keeps a
//! file's own `mod` declarations resolving next to it, as when it was a binary of its own.
//! AGENTS.md, "The gate", says why and how.

// A support file that several test files load by `#[path]` is a module of each of them, and a file
// may wrap its cases in a module of its own name, as each did when it was a binary of its own.
#![allow(clippy::duplicate_mod, clippy::module_inception)]

#[path = "adversary_observe.rs"]
mod adversary_observe;
#[path = "adversary_pass2.rs"]
mod adversary_pass2;
#[path = "fixture_observations.rs"]
mod fixture_observations;
#[path = "redaction.rs"]
mod redaction;
