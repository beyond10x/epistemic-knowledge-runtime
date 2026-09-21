//! AGENTS.md invariant 1: only `ekr-kernel` constructs a `ValidatedTransaction`.
//!
//! Design § 19: "Only a `ValidatedTransaction` may be committed. This uses Rust's type system as
//! part of the integrity boundary." A comment saying so is not that boundary — this is. The type
//! is public, because a committer above the kernel has to name it and read it; its fields are not,
//! so the only way one comes into existence is the pipeline that did the work.
//!
//! The case is written from an integration test, which is a separate crate, so it sees exactly
//! what any crate above the kernel sees.

use ekr_core::{ContentHash, RevisionNumber};
use ekr_graph::CanonicalValue;
use ekr_kernel::{GraphTransaction, ValidatedTransaction};

fn forge(transaction: GraphTransaction<CanonicalValue>) -> ValidatedTransaction {
    ValidatedTransaction {
        transaction,
        validated_against: RevisionNumber::SEED,
        validation_hash: ContentHash::of(&"nothing was validated".to_owned()),
    }
}

fn main() {}
