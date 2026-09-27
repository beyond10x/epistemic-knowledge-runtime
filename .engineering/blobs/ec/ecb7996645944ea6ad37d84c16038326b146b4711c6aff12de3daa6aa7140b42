---
format: aep.planning-md/1
id: review-result:adversary-graph-pass-1
kind: review-result
status: active
title: Adversary, story:graph-model-and-assertions, pass 1
relations:
- reviews: story:graph-model-and-assertions
revision: 1
---
## Pass

Adversary pass 1 over `story:graph-model-and-assertions`, wave p1-04, against the uncommitted unit
work in worktree `ekr-impl-graph` at base `90c8c58`. Opus, 186k tokens, 43 tool uses, 4m45s.

188 cases before, 192 executed, 4 red in one new file,
`crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs`. `task check` exit 201 at the `test`
step; `fmt-check` and `clippy` pass.

## What it could not break

`Timestamp` across `i64::MIN`, `i64::MAX`, zero, negative, `+7`, `007`, `-0`, whitespace, empty,
overflow — the `RevisionNumber` text bar holds. `Encoder::variant` prefix-freedom against all
fourteen prior tags and under nesting. The half-open `[from, to)` convention, applied identically to
both valid-time bounds. `RevisionEvent` serde over all six variants. The `Timestamp` adoption in
`ekr-ontology`, including all ~40 mechanical `seed(id, 0)` call sites and the proptest strategy
rewrite. `Node` keeping its `NodeId` across a thousand renames, against the real type. Scope creep:
none — no `KnowledgeState`, no `QueryScope`, no writer. `Confidence` bounds. Both `trybuild`
`.stderr` files name the error their case exists for.

## Findings

```findings
- file: crates/ekr-graph/src/snapshot.rs
  line: 62
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "active() is the same read as valid_at(i64::MAX), so the doc's \"holds true now\" means the end of representable time: a fixed-term fact true today is excluded and an announced successor whose tenure has not begun is included"
- file: crates/ekr-graph/src/assertion.rs
  line: 292
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the AssertionStatus::Active conjunct of is_current() is unreachable because is_accepted() and the non-Active statuses are disjoint, so deleting it leaves the suite green and snapshot_reads.rs:239 does not test the filter it names"
- file: crates/ekr-graph/src/assertion.rs
  line: 259
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "graph.yaml:256 declares ekr.graph.Assertion.recorded_from as required Timestamp while transaction_time.from is Option, and the crate's own doctest at lib.rs:59 builds a record without one"
- file: crates/ekr-graph/src/assertion.rs
  line: 84
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "TemporalRange::is_open tests only the upper bound, so a record whose transaction time never started is reported by active() as the runtime's current belief"
- file: crates/ekr-graph/src/assertion.rs
  line: 136
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "collapsing ValidationState and AssertionStatus into one field makes a retraction erase the Accepted validators it retracts, against design section 36's \"without erasing that it once did\""
- file: crates/ekr-graph/tests/domain_projection.rs
  line: 199
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the entity-field guard covers five of eight entities and silently omits ekr.graph.Assertion and ekr.graph.Evidence, the two whose projection actually diverges"
- file: crates/ekr-graph/src/events.rs
  line: 95
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "variant_index() is a hand-written match on names, so the doc's claim that the index is the declared position is false, and revision_events.rs:74 pins a renumbering but cannot see a reorder"
- file: crates/ekr-graph/src/canonical.rs
  line: 91
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "CanonicalDependency is blanket over every T, so a CanonicalRef parameterised over a transient type is accepted by CanonicalGraph::resolve; no caller in the workspace does this, so nothing reaches it"
```
