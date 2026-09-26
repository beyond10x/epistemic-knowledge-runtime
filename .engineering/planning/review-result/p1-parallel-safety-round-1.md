---
format: aep.planning-md/2
id: review-result:p1-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, P1 decomposition, round 1
tags:
- critic:aep-plan:plan-critic-parallel-safety
relations:
- reviews: story:workspace-crate-skeleton
- reviews: story:kernel-identity-and-hashing
- reviews: story:ontology-types-and-values
- reviews: story:graph-model-and-assertions
- reviews: story:transaction-and-validators
- reviews: story:eventlog-store
- reviews: story:commit-and-revision-lineage
- reviews: story:seed-and-explain
- reviews: story:ekr-cli
- reviews: story:ess-conformance-kernel
revision: 1
---
needs-revision

story:transaction-and-validators — its scope lists `crates/ekr-kernel/Cargo.toml` but not the shared root `Cargo.lock`, and it is the first consumer of `proptest` (line 48), a dependency `story:workspace-crate-skeleton` does not pre-declare among the shared deps (`uuid`, `sha2`, `hex`, `serde`, `serde_json`, `thiserror`, `eventlog-*`, workspace-crate-skeleton.md:51); `story:eventlog-store`, its wave-5 co-item, is likewise the first consumer of `eventlog-sqlite`/`eventlog-file` (eventlog-store.md:51) and also omits `Cargo.lock` from its scope (eventlog-store.md:13) — both stories independently regenerate the single workspace `Cargo.lock` at the same time and neither body names the other or the file — .engineering/planning/story/transaction-and-validators.md:48

What I read: 10 story bodies in full (`aep plan artifact show` equivalent via direct file read) for workspace-crate-skeleton, kernel-identity-and-hashing, ontology-types-and-values, graph-model-and-assertions, transaction-and-validators, eventlog-store, commit-and-revision-lineage, seed-and-explain, ekr-cli, ess-conformance-kernel; ran `aep plan artifact waves --kind story` (9 waves, 9 collisions, 0 unassessed) and attempted `aep plan artifact graph` (rejected `--kind`, not needed since `waves` already gave the dependency-derived ordering).

Surface establishment: 10 cited (every story's own machine `scope:` block plus prose `## Scope` section named its paths directly), 0 inferred-by-me, 0 unplaceable. The one collision I flag rests on a surface (`Cargo.lock`) neither story declares — that specific claim is **inferred**, from Cargo-workspace mechanics (a crate's first use of a not-yet-locked dependency rewrites the single root lockfile) plus the explicit `proptest`/`eventlog-sqlite`/`eventlog-file` mentions in the two bodies, not from either body citing `Cargo.lock` itself.

What I could not establish: whether `story:eventlog-store`'s use of `eventlog-sqlite`/`eventlog-file` was already fully resolved into `Cargo.lock` during wave 1 (i.e., whether workspace-crate-skeleton's empty crates actually exercise those `[workspace.dependencies]` entries) — I could not check this without a real `cargo build`, so the eventlog-store half of the Cargo.lock claim is weaker than the proptest half, which is unambiguous (proptest is not in the pre-declared shared-dependency list at all). Everything else in the set (the 9 CLI-reported collisions, all against `story:workspace-crate-skeleton`) is already resolved by the transitive `depends_on` chain into separate waves and is not a live parallel-safety defect — out of my lane to say more about whether that sequencing is the *right* split (design critic's question), only that it does keep those pairs apart in time.

```findings
- file: .engineering/planning/story/transaction-and-validators.md
  line: 48
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its scope lists crates/ekr-kernel/Cargo.toml but not the shared root Cargo.lock, and it is the first consumer of proptest (line 48), a dependency story:workspace-crate-skeleton does not pre-declare among the shared deps (uuid, sha2, hex, serde, serde_json, thiserror, eventlog-*, workspace-crate-skeleton.md:51); story:eventlog-store, its wave-5 co-item, is likewise the first consumer of eventlog-sqlite/eventlog-file (eventlog-store.md:51) and also omits Cargo.lock from its scope (eventlog-store.md:13) — both stories independently regenerate the single workspace Cargo.lock at the same time and neither body names the other or the file (inferred: Cargo-workspace lockfile mechanics, not stated in either body)"
```
