---
format: aep.planning-md/2
id: review-result:adversary-invariants-pass-1
kind: review-result
status: active
title: Adversary, wave p1-06 invariant repair, pass 1
relations:
- reviews: task:only-a-validated-transaction-commits
- reviews: task:canonical-state-references-are-typed
revision: 1
---
## Pass

Adversary pass 1 over wave p1-06's invariant repair, against the uncommitted unit work in worktree
`ekr-impl-invariants` at base `f437031`. Opus, 190k tokens, 61 tool uses, 6m19s.

351 cases before, 356 executed, 5 red across three new files. Seven findings, two blockers.

**Both blockers are the guard that invariant 1 now rests on**, and the second is the coordinator's
error compounding: `AGENTS.md:92-96`, added in `c4e436e` — the commit this branch forked from — bans
locating the repository with the compile-time macro, for exactly the reason that a guard reading the
wrong tree reports clean. `crates/ekr/tests/story_contract.rs` does it, and this wave rewrote
invariant 1 to say its facts are "read off **this** tree" by that file.

## What it could not break

No path from `crates/ekr` to the store's writer: the manifest declares `ekr-store` in neither
dependency list, and the kernel re-exports only `Commit`, `CommitError` and `Validations`, so
`RevisionLog` cannot be brought into scope. The dependency-edge guard reads the manifests in both
directions rather than transcribing a list. Neither replacement case dropped a claim — patch 2
changes no assertion and its authority keys on a specific hash, so a fold that declined everything
still turns the stale case red; patch 1 moved the membrane claim into the `trybuild` case, whose
expectation pins `expected NodeId, found CanonicalRef<Node>` rather than a bare failure. The
regenerated expectation is the help block only. Content-address stability, carried by an existing
case. The wire shape is unchanged.

## Findings

```findings
- file: crates/ekr/tests/story_contract.rs
  line: 277
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the guard AGENTS.md invariant 1 now rests on matches the literal `impl CommitAuthority for`, so an implementation spelled with the trait's path in any src/ is invisible to it, and the workspace already contains that spelling in the coordinator's own applied patch"
- file: crates/ekr/tests/story_contract.rs
  line: 45
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the same guard locates the repository with the compile-time macro that AGENTS.md:92-96 forbade in the commit this branch forked from, and for exactly this reason, so a binary built in one checkout scans that checkout and reports clean while another tree runs it"
- file: crates/ekr-graph/src/assertion.rs
  line: 90
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the crate's own doc names Object of Value as the type of a claim in a transient root, and the new reference parameter defaults to a canonical reference without consulting ValueSpace, so that spelling is now a transient claim carrying a canonical reference"
- file: crates/ekr-graph/src/canonical.rs
  line: 68
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "the canonical reference stores a NodeId for all seven marker types, so a reference to evidence resolves against the nodes map and answers a node, and this wave made that marker the bound on five new impls"
- file: crates/ekr-graph/src/canonical.rs
  line: 25
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "ADR 0008 and its task both say the kernel's reference validator holds ids arriving from outside Rust, and the only such path is the seed fold, which sits below ekr-kernel and can reach no validator in any process, so a document's dangling edge target becomes a canonical reference unrefused"
- file: crates/ekr-store/src/log.rs
  line: 388
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a store opened without an authority folds no commit and reports no failure, so the default construction of a public store is the silent one, while the neighbouring bad input returns an error and bricks every later read"
- file: crates/ekr-store/src/log.rs
  line: 120
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "CommitAuthority, RecordedValidation and EventlogStore::under are new public surface carrying half of invariant 1 and appear in no store domain entry and no story"
```
