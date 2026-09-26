---
format: aep.planning-md/2
id: review-result:p1-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, P1 decomposition, round 2
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
approve — no.

**needs-revision**

story:transaction-and-validators — its Notes (lines 66–67) claim `trybuild` is "already declared for `ekr-kernel` by `story:workspace-crate-skeleton`," but `story:workspace-crate-skeleton`'s dependency list gives `ekr-kernel` only `uuid`, `sha2`, `hex`, `serde`, `serde_json`, `thiserror`, dev `proptest` (workspace-crate-skeleton.md:55) and declares `trybuild` only as a dev-dependency of `ekr-graph` (workspace-crate-skeleton.md:57); this story's own scope nonetheless ships a `trybuild` compile-fail test under `crates/ekr-kernel/tests/compile_fail/` (transaction-and-validators.md:58) without adding `crates/ekr-kernel/Cargo.toml` or `Cargo.lock` to its scope, so — contrary to its closing sentence — it still needs to add a dependency in the same wave as its co-item `story:eventlog-store`, reopening the exact Cargo.lock collision round 1 named, through the one dependency the revision's pre-declaration pass missed — .engineering/planning/story/transaction-and-validators.md:66

What I read: `review-result:p1-parallel-safety-round-1` in full; `aep plan artifact show` on all 10 stories in the set (workspace-crate-skeleton, kernel-identity-and-hashing, ontology-types-and-values, graph-model-and-assertions, transaction-and-validators, eventlog-store, commit-and-revision-lineage, seed-and-explain, ekr-cli, ess-conformance-kernel); `aep plan artifact waves --kind story` (9 waves, 5 collisions, 0 unassessed); `grep -rn "trybuild"` / `"proptest"` and a dependency-crate-name sweep across `.engineering/planning/story/*.md` to check every other story's "adds no dependency beyond the skeleton's" claim against workspace-crate-skeleton's per-crate list.

Surface establishment: 10 cited (every story's own `## Scope` section names its paths directly, and the dependency claims are cited straight out of prose, not inferred from naming), 0 inferred, 0 unplaceable. The one live finding rests entirely on cited text (workspace-crate-skeleton.md:55/57 vs. transaction-and-validators.md:58/66); the one inferred link inside it — that adding a dependency to a crate's `Cargo.toml` regenerates the shared `Cargo.lock` — is Cargo-workspace mechanics, the same inference round 1 made and the revision itself accepted as true (it's the premise of the whole fix).

What I could not establish: nothing outside my lane. The wave-1 collisions the CLI reports (5, all against `story:workspace-crate-skeleton`) are the same as round 1 already cleared as non-live — each of the 9 later stories carries a transitive `depends_on` chain back to the skeleton (verified directly on `kernel-identity-and-hashing`, `ontology-types-and-values`, `graph-model-and-assertions`, `eventlog-store`, `ekr-cli`), which keeps the pair apart in time regardless of the shared file; whether that sequencing is the *right* split is the design critic's question, not mine.

```findings
- file: .engineering/planning/story/transaction-and-validators.md
  line: 66
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its Notes claim trybuild is already declared for ekr-kernel by story:workspace-crate-skeleton, but workspace-crate-skeleton declares trybuild only for ekr-graph (workspace-crate-skeleton.md:57) and only proptest for ekr-kernel (workspace-crate-skeleton.md:55); this story's own tests/compile_fail/ trybuild test (transaction-and-validators.md:58) is not covered by that pre-declaration, so the story still needs to add a dependency in the same wave as its co-item story:eventlog-store, reopening the Cargo.lock collision round 1 named"
```
