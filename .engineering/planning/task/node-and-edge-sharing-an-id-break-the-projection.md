---
format: aep.planning-md/3
id: task:node-and-edge-sharing-an-id-break-the-projection
kind: task
status: implemented
title: A node and an edge that share one UUID are admitted, and the graph projection then refuses the revision
summary: 'Medium: the renderer buckets node and edge assertions under one string key'
tags:
- review-2026-09-28
- severity-medium
relations:
- serves: vision:o5
- derived_from: story:graph-projection-renderer
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:11Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Severity

Medium (the review's rating, kept). A revision the kernel committed cannot be projected, so
`ekr view` and the projection export fail for that revision and every later one that still holds
both records.

## The review (external review of EKR, 2026-09-28, verbatim)

> Medium — valid typed identities can break projection. A node and edge sharing the same UUID are
> admitted by the kernel, but the renderer combines their assertion buckets and refuses the
> projection: "3 assertions were placed and 4 listed." Reproduced on both providers.
> `crates/ekr-views/src/document.rs:390`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified, kernel side**: the structural validator keeps separate identity sets for nodes and
  edges and checks a new node id against `state.nodes` only and a new edge id against
  `state.edges` only (`crates/ekr-kernel/src/validate/structural.rs:312-326`), so it admits a `NodeId`
  and an `EdgeId` with one UUID. That the whole kernel then commits them is the review's
  observation, not mine.
- **Verified, renderer side**: `render` buckets node and edge assertions in one map keyed by the id
  text, `by_subject: BTreeMap<String, …>` (`crates/ekr-views/src/document.rs:411`, `:439`), and both
  a node and an edge read their assertions from that key (`document.rs:456`, `:473`). With a shared
  UUID each lists the other's assertions too, and the placed/listed check refuses
  (`document.rs:552-556`, message "{listed} assertions were placed and {assertion_count} listed").
- The review's `document.rs:390` falls in `declared`, not in the bucketing; the mechanism is at the
  lines above.
- **Inferred, not observed by me**: the exact counts in the review's message, and whether the
  bounded reads in `crates/ekr-views/src/index.rs` mix the two kinds the same way (not checked).

## Reproduce

Commit one transaction that creates a node with id U and an edge with the same UUID U, and one
assertion about each. Run `ekr view` or `ekr_views::project` for that revision on either provider.

## Acceptance

On both providers, a revision holding a node and an edge that share one UUID either projects, with
each record listing only the assertions whose subject is that record, or is never committed
because the kernel refuses the second identity with a named structural issue. A test in
`crates/ekr-views/tests` (or `crates/ekr-kernel/tests`, for the refusal) that fails before the fix
passes after it. Which side owns the rule is the implementor's call and is recorded in the result.
