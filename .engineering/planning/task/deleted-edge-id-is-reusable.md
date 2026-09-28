---
format: aep.planning-md/3
id: task:deleted-edge-id-is-reusable
kind: task
status: implemented
title: A deleted edge's id can be created again for a different relationship
summary: 'Medium: the identity-already-exists rule reads only the current state, so one EdgeId names two relationships over history'
tags:
- review-2026-09-28
- severity-medium
relations:
- serves: vision:o5
- derived_from: story:transaction-and-validators
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:51:38Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:51:38Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:11Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Severity

Medium (the review's rating, kept). No refusal and no replay failure: history stays replayable,
but one `EdgeId` names two different relationships at two revisions, so a reference to that id,
an assertion about it or an explain chain through it can mean the wrong one.

## The review (external review of EKR, 2026-09-28, verbatim)

> Medium — deleted edge identities can be reused for different relationships. Delete an edge, then
> create different endpoints under its old ID: validation, commit, and full replay all accept it.
> This undermines stable historical identity. `crates/ekr-kernel/src/validate/structural.rs:259`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified**: applying `DeleteEdge` removes the edge from the graph's map and keeps no record of
  its id (`crates/ekr-kernel/src/apply.rs:91-93`); `CanonicalGraph` holds nodes, edges, assertions
  and evidence and no deleted-identity set (`crates/ekr-graph/src/canonical.rs:440-455`).
- **Verified**: the "identity already exists" rule for `CreateEdge` asks only whether the current
  state holds the id (`crates/ekr-kernel/src/validate/structural.rs:320-326`), so an id deleted at
  an earlier revision is free again. The module's own statement of the rule is that a committed
  revision is immutable and "a later change is a new revision … never a create"
  (`structural.rs:17-18`).
- **Inferred**: full replay re-runs the same validators over the same state, so it accepts what
  validation accepted; the review says validation, commit and full replay all accepted it.
- The review's `structural.rs:259` points into the `ModifyProperty` owner check; the edge
  identity check is at the lines above.

## Reproduce

Create edge E between A and B and commit. Commit a transaction with `DeleteEdge(E)`. Commit a
transaction with `CreateEdge` id E between C and D (or another edge type). All three validate and
commit; `ekr snapshot --at` the first and the last revision shows E with different endpoints.

## Acceptance

A `CreateEdge` whose id any earlier revision of the store held is refused at validation with a
named structural issue, on both providers, and a store that already contains such a reuse still
replays (the rule applies to new transactions under a validation profile that says so, as the
v1/v2 split in `structural.rs` does). A test that creates, deletes and re-creates one edge id
fails before the fix and passes after it.


## State, 2026-09-28

- The validator is built and tested on branch `impl/deleted-edge-id` at `e26ff875` (pushed, not merged): `identity-previously-held`, node and edge ids in one space; 4 validator cases, red first; ekr-kernel 352 passed. No validation profile selects it yet.
- Held on a design point: the drafted wiring (`~/.local/state/ekr-offside/deleted-edge-id-profile-v3-wiring.patch`, a v3 profile `ekr.p3-deterministic/1` + `ekr.p2-apply/1`) makes v3 stores skip checkpoint restore and replay from the seed, because a checkpoint holds only the head's graph. That would undo the one-pass load (first `/overview` 0.8 s). Next: carry the held-id set in the checkpoint, after `task:checkpoint-graph-root-is-unbound` merges, then wire the profile.
- The review's bug is reproduced on both providers by the unit's end-to-end case under v2. Nodes have no delete gap (only DeleteEdge removes), but v1/v2 accept a node minted over an edge's id, live or deleted.
