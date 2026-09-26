---
format: aep.planning-md/2
id: review-result:adversary-graph-pass-2
kind: review-result
status: active
title: Adversary, story:graph-model-and-assertions, pass 2
relations:
- reviews: story:graph-model-and-assertions
revision: 1
---
## Pass

Adversary pass 2 over `story:graph-model-and-assertions`, wave p1-04, against the unit work plus
correction round 1, uncommitted in worktree `ekr-impl-graph` at base `90c8c58`. Opus, 283k tokens,
30 tool uses, 3m52s.

193 cases before, 196 executed, 3 red in one new file,
`crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs`. `task check` exit 201 at the `test`
step; `fmt-check` and `clippy` pass.

**All eight pass-1 findings resolved or correctly deferred. No pass-1 `file:line` recurs.** The
reviewer judged its own three re-aimed cases: two were made stronger than the originals, the third
was redirected honestly because the state it described is now unrepresentable, and the fourth was
weakened in one specific way, which is finding 2 below.

The reviewer accepted the coordinator's call on pass-1 finding 5 — the retraction shape is design
§ 17's and not the crate's — and agreed that deleting `active()` was better than renaming it,
because a caller's missing "now" is then a compile error rather than a doc comment.

## What it could not break

The sealed `CanonicalTarget`: not nameable from outside, no generic impl to route a transient type
through, and a collection, a tuple, an associated type and a trait object all refused. All three
`trybuild` `.stderr` files name the error their case exists for. `Assertion` serde round-trips
without `#[serde(default)]`, so the reviewer wrote two cases, found them green, and deleted them.
The empty graph and the all-`Proposed` graph. `valid_at`'s half-open convention on both bounds and
both dimensions. The rewritten `snapshot_reads.rs`: seven cases before and after, nothing merged
away, the restated acceptance pinned in both directions. `RevisionEvent`'s six-variant round trip,
the byte-identical-payload case, and the thousand-rename case, all unmoved by correction 1. The
`FUSIONS` table: a stale entry turns it red.

## Findings

```findings
- file: crates/ekr-graph/tests/domain_projection.rs
  line: 377
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the extended field guard matches `pub {field}:` anywhere in the concatenated crate source with no tie between a declaration and its Rust type, so a new ekr.graph.Node.parent is reported carried by GraphRoot.parent and the doc's claim that a new field cannot be omitted is false for any name already in use"
- file: crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs
  line: 160
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the guard banning the old active() filter scans src/snapshot.rs alone while TemporalRange::is_open stays public and exported, so the same filter rebuilt in any of the other nine modules or in ekr-kernel leaves it green"
- file: crates/ekr-graph/src/assertion.rs
  line: 139
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "TransactionTime::new and TemporalRange::new accept an end preceding their start, producing a belief held at no instant and an assertion in canonical state that valid_at returns at no instant; no caller builds one in P1 and no layer is named as owning the check"
- file: crates/ekr-graph/tests/revision_events.rs
  line: 216
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the declaration-order scan sees only struct variants and guards vacuity against a constant six, so a seventh tuple or unit variant inserted mid-list is invisible to it and to every_variant_carries_its_declared_index_and_domain_name"
- file: crates/ekr-graph/src/assertion.rs
  line: 158
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "TransactionTime::held_at answers the transaction-time axis of a bitemporal read that does not exist and has no caller in any src/ in the workspace, surviving only because the public-surface guard counts a test as a user"
```
