//! The other way a private field is not private: `serde`.
//!
//! A `#[derive(Deserialize)]` on `ValidatedTransaction` would be a public constructor that takes a
//! string — every field set by the caller, the validation never run, and no `unsafe` and no
//! visibility rule broken. The sibling case in this directory holds the struct literal; this one
//! holds the route round it, because closing one and leaving the other open closes nothing.
//!
//! `serde` and `serde_json` are dependencies of `ekr-kernel`, so this is the route a crate above
//! it actually has.

use ekr_kernel::ValidatedTransaction;

fn forge(recorded: &str) -> ValidatedTransaction {
    serde_json::from_str::<ValidatedTransaction>(recorded).expect("a transaction from a string")
}

fn main() {}
