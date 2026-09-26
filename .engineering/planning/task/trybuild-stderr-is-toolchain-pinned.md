---
format: aep.planning-md/2
id: task:trybuild-stderr-is-toolchain-pinned
kind: task
status: implemented
title: The compile-fail expectations are pinned to one rustc's diagnostic wording
relations:
- serves: vision:o2
- derived_from: story:source-guard-debt
revision: 6
---
## What is wrong

`crates/ekr-graph/tests/compile_fail/*.stderr` was generated with `TRYBUILD=overwrite` on the
toolchain present on 2026-09-21. `trybuild` compares compiler output byte for byte, and rustc
rewords diagnostics between releases, so these files go red on a toolchain change for a reason that
has nothing to do with the membrane they exist to prove.

The workspace pins `rust-version` in `Cargo.toml`, which is a minimum rather than an exact
toolchain, and there is no `rust-toolchain.toml`.

## What closes this

One of: a `rust-toolchain.toml` pinning the exact version the gate runs, so the `.stderr` files
have a fixed target; or a trybuild configuration that matches on the error code rather than the
full text; or a documented step that regenerates them on a toolchain bump. Pick one and say why in
the commit.

Until then, a red `membrane` lane after a toolchain change is to be read as this task before it is
read as a regression.

## It is not only a toolchain bump, 2026-09-21

Wave p1-06 hit this without changing the toolchain. A hand-written generic `Deserialize` impl on
`CanonicalRef<T>` flipped rustc's incidental *"the following other types implement trait"* help from
the short form to `` `X` implements `Y` ``, and
`crates/ekr-kernel/tests/compile_fail/validated_transaction_cannot_be_deserialised.stderr` had to be
regenerated. The error code, the message and the span were byte-identical; only those eight help
lines moved.

**So the trigger is wider than this task assumed.** Any new impl of a trait a compile-fail case
mentions changes what rustc lists as implementors of that trait, and every one of those lists is
pinned byte for byte. A crate two modules away implementing `Deserialize` can turn a membrane case
red without touching the membrane.

That strengthens the case for matching on the error code rather than the full text, which is the
second option above — a `rust-toolchain.toml` pins the compiler and does nothing about this.

## Closure policy, 2026-09-22

The original absent-pin premise is obsolete: rust-toolchain.toml and the
correctness workflow already select the same exact compiler. The later
same-compiler trait-help churn is real. AGENTS.md now documents a bounded review
and refresh policy: compare the forbidden operation, primary diagnostic, code and
source expression; refresh only an affected target; review its diff and rerun
without overwrite. Successful compilation or an unrelated error never qualifies.
Existing snapshots need no change for this repair. The complete integration gate
must execute the existing graph and kernel compile-fail targets before closure.
