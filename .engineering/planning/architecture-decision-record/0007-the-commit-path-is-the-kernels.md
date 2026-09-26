---
format: aep.planning-md/2
id: architecture-decision-record:0007-the-commit-path-is-the-kernels
kind: architecture-decision-record
status: accepted
title: ADR 0007 — Only the kernel reaches a writer to canonical state
relations:
- decides: task:only-a-validated-transaction-commits
revision: 2
---
## Status

Proposed 2026-09-21, after an independent review of the merged P1 core found the invariant carried
by nothing. Accepted the same day.

## What the review found

`AGENTS.md` invariant 1: *"Only `ekr-kernel` constructs a `ValidatedTransaction`, and only a
`ValidatedTransaction` commits. No other crate holds a writer to canonical state."*

The first sentence holds and two `trybuild` cases prove it. **The second is carried by nothing.**

- `RevisionLog::append` (`crates/ekr-store/src/log.rs:142`) is public and takes a bare
  `RevisionEvent`.
- `Fold::apply` marks a transaction validated because a `TransactionValidated` event was appended —
  by anyone.
- **`ValidatedTransaction` has zero consumers in any `src/` in the workspace.** The kernel builds
  the only thing that may commit and nothing consumes it.
- `crates/ekr/Cargo.toml:20` declares `ekr-store` directly, so the binary can reach the writer
  without passing the kernel at all.
- The store's own suite pins the negation green: `tests/fold_rules.rs:459-510` and
  `tests/lineage/mod.rs:60-92` advance the head to revision 1 from three hand-written events.

## Why the obvious fix does not exist

`ekr-store` sits **below** `ekr-kernel` in the acyclic order `docs/roadmap.md` § 3 gives, so the
store cannot name `ValidatedTransaction` and `append` cannot take one. Inverting the order is a
cycle.

Sealing does not help either. A sealed trait in `ekr-store` is unimplementable outside `ekr-store`,
which excludes `ekr-kernel` along with everybody else. Rust has no way to say *only this other
crate*.

## Decision

**The guarantee is stated at the boundary it can actually hold, and the store stops being reachable
from outside.**

1. **`crates/ekr` drops its direct dependency on `ekr-store`.** The binary reaches persistence only
   through `ekr-kernel`. `crates/ekr/tests/story_contract.rs`'s edge table moves with it, and
   `story:workspace-crate-skeleton`'s constraint table with that.
2. **`ekr-kernel` gains the only public commit path**, taking a `ValidatedTransaction` by value.
   That is the function `ValidatedTransaction`'s zero consumers were waiting for.
3. **`ekr-store` keeps `append` as a port, and the fold stops trusting it.** A `TransactionValidated`
   event is no longer sufficient for the fold to treat a transaction as validated; the fold learns
   validation through a trait `ekr-store` declares and `ekr-kernel` implements, injected at
   construction. That trait is also what lets the fold apply operations, which
   `story:commit-and-revision-lineage` needs and cannot otherwise have.
4. **The invariant text is narrowed to what is true.** "No other crate holds a writer to canonical
   state" becomes "no consumer of this runtime can reach a writer to canonical state without a
   `ValidatedTransaction`; within the workspace `ekr-store` is the port and `ekr-kernel` is its only
   caller, held by a case."

## What this is and is not

**It is not a type-level guarantee inside the workspace.** A future crate added below `ekr-kernel`
and given an `ekr-store` dependency could call `append`. What holds is the dependency graph and a
case that reads it — the same mechanism `crates/ekr/tests/story_contract.rs` already uses for every
other edge.

Saying so is the point. The invariant claimed a type and delivered a convention, and a convention
that describes itself accurately is worth more than a type that is not there.

## The exit criterion

`crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs::a_commit_lands_with_no_validated_transaction_anywhere_in_the_process`,
written by the independent reviewer, is red today. It runs in the store's own test binary, which
cannot link `ekr-kernel`, so no `ValidatedTransaction` can exist in that process. **The wave is done
when that case is green.** It is the exit criterion rather than a claim about one.
