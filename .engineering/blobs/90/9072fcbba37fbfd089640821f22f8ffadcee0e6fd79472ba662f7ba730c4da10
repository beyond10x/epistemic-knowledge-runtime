---
format: aep.planning-md/1
id: task:only-a-validated-transaction-commits
kind: task
status: implemented
title: A commit lands with no ValidatedTransaction anywhere in the process
relations:
- serves: vision:o2
revision: 4
---
## What this implements

`architecture-decision-record:0007-the-commit-path-is-the-kernels`. Carried as a task rather than a
story because it repairs an invariant the merged P1 core claims and does not hold; no story owns it.

## Acceptance

**A commit cannot land without a `ValidatedTransaction` existing in the process.**

The independent reviewer's case
`crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs::a_commit_lands_with_no_validated_transaction_anywhere_in_the_process`
is red today and goes green. It runs in the store's own test binary, which cannot link
`ekr-kernel`, so no `ValidatedTransaction` can exist there.

## Scope

- `crates/ekr/Cargo.toml` — drop the direct `ekr-store` dependency
- `crates/ekr/tests/story_contract.rs` — the edge table moves with it
- `crates/ekr-kernel/src/` — the public commit path taking a `ValidatedTransaction` by value, which
  is the caller that type has never had
- `crates/ekr-store/src/log.rs` — the fold stops treating an appended `TransactionValidated` as
  proof of validation, and learns it through a trait this crate declares and `ekr-kernel` implements
- `crates/ekr-store/tests/fold_rules.rs`, `crates/ekr-store/tests/lineage/mod.rs` — both pin the
  negation green today at `fold_rules.rs:459-510` and `lineage/mod.rs:60-92`
- `AGENTS.md` — invariant 1's second sentence narrowed to what holds

## Tests

- the reviewer's case above, green
- a case reading the dependency graph: no crate outside `ekr-kernel` declares `ekr-store`
- the trait the fold takes is implemented in `ekr-kernel` and nowhere else

## Notes

The trait in item 4 is also what `story:commit-and-revision-lineage` needs to apply operations at
all: `GraphOperation` lives in `ekr-kernel`, above both crates that would apply it, so the fold has
no type to read an operation as. Building it here is what makes that story's option B possible.
