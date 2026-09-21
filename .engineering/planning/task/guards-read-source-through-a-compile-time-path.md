---
format: aep.planning-md/1
id: task:guards-read-source-through-a-compile-time-path
kind: task
status: draft
title: Twenty files locate the repository with a compile-time macro, so a test binary reads the checkout it was built in
relations:
- serves: vision:o2
revision: 1
---
## What happened

After wave p1-05 merged, `task check` on `main` failed at the `test` step with:

```
reading <worktrees>/ekr-wave-p1-05/crates/ekr-core/src: No such file or directory
```

That path is a worktree the wave had just deleted. The primary checkout was reading source out of a
tree that no longer existed.

## Why

**Twenty files read the repository's own source at run time through `env!("CARGO_MANIFEST_DIR")`** —
thirty-nine uses, and one of them, `xtask/src/main.rs:32`, is not a test. That macro is a *compile-time* constant: it bakes
in the path of whichever checkout compiled the binary.

`AGENTS.md` § The gate mandates `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/epistemic-knowledge-runtime`,
shared by every tree of this repository, precisely so a second checkout does not pay for a second
full build. So a test binary compiled in a worktree is reused by the primary checkout, and it keeps
reading the worktree's path.

While both trees exist the failure is **silent and worse than a crash**: the guard passes, reading a
different checkout's source. Every one of these guards holds a document against code, and a guard
reading the wrong tree's code is a guard that has stopped checking anything.

## How it presented

Only after the worktree was deleted, as three failures in `ekr-core`. `cargo clean -p ekr-core`
cleared it and the gate then ran green at 337 cases. Nothing in the wave was wrong; every unit's own
gate ran inside its own tree where the path was correct.

## Why it will recur

Every wave of this phase has created worktrees, shared that build directory, and deleted the trees at
the close. Five waves have done it. This is the first time a guard that reads source has existed in
enough crates for the stale path to surface — wave p1-05 added the nineteenth file.

## What closes this

The guards should locate the repository from something that is true at run time, not at compile
time. Candidates, cheapest first:

- walk up from `std::env::current_dir()`, which `cargo test` sets to the package root, until a
  directory containing `Cargo.lock` is found;
- or a `build.rs` that re-emits the path each build, which makes it a build input rather than a
  constant — this still bakes a path, but the fingerprint then changes when the path does;
- or `cargo clean -p <crate>` in the gate whenever the manifest directory differs from the recorded
  one, which is a workaround rather than a fix.

The first is a few lines in one shared helper, and nineteen files already duplicate the same
`crate_root()` function that would hold it.

## The rule it is in tension with

`AGENTS.md` § The gate says one tree at a time shares the build directory. That rule is right and is
not what should change. What should change is a test that assumes the tree it was compiled in is the
tree it is running for.


## Corrections, 2026-09-21, from the independent review

Three, all against the coordinator who filed this.

**The count was wrong.** Nineteen files and thirty-eight uses; it is **twenty and thirty-nine**,
measured. The twentieth is `xtask/src/main.rs:32`, which bakes the path into a non-test binary — so
the fix is not confined to test helpers.

**The cheapest fix was not listed.** `std::env::var("CARGO_MANIFEST_DIR")` reads the variable cargo
sets **per process** for `cargo test` and `cargo run`, so it is the run-time value for whichever tree
is running. It is a one-token change at each of the thirty-nine sites and needs no helper at all.
Prefer it to walking up from `current_dir()`.

**One option above does not work.** The `build.rs` that re-emits the path is fingerprinted the same
way the crate is, so it would not re-run when only the path changed. Strike it.

**The mechanism claim is unsettled.** This task asserted that a binary compiled in a worktree is
reused by the primary checkout because the build directory is shared, while `AGENTS.md` says unit
artifacts are keyed by a hash including the manifest path and therefore do not clobber. Both cannot
hold. The review argues cargo hashes path sources relative to the workspace root, so two checkouts
write the same filenames and do clobber, which is what makes the reuse possible. A probe settles it.
**Nothing in the fix depends on which way it lands** — the macro is a compile-time constant either
way — so the rule stands and the mechanism sentence is marked pending rather than guessed.
