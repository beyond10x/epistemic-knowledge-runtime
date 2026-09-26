---
format: aep.planning-md/2
id: review-result:adversary-eventlog-store-pass-1
kind: review-result
status: active
title: Adversary, story:eventlog-store, pass 1
relations:
- reviews: story:eventlog-store
revision: 1
---
## Pass

Adversary pass 1 over `story:eventlog-store`, wave p1-05 unit 1, against the uncommitted unit work
in worktree `ekr-impl-store` at head `f981a07`. Opus, 147k tokens, 54 tool uses, 4m26s.

261 cases before, 266 executed, 5 red across two new files. `clippy` and `fmt-check` both exit 0, so
the gate's red is the assertions and nothing else.

One blocker, and it corrects a premise the coordinator wrote: `task:the-membrane-stops-at-the-store-boundary`
says nothing on a stored node records which space it came from. True of a node, and the crossing
does not take a node — it takes a `GraphDocument`, which is a `GraphRoot` plus four maps, and
`GraphRoot.space` is exactly that marker. The document **is** distinguishable and the crossing had
the marker in hand.

## What it could not break

The acceptance opens a real path twice for both providers rather than an in-memory store, so
"closed and reopened" means what it says. The root functions field by field: a node property value,
an edge property value, an assertion valid-time bound, a node alias, a node type state and an
evidence instant each move their root. A float inside a list inside a record in a node property is
refused with a path. `replay` really passes a limit of one where `fold` pages at a thousand, so the
shipped equality case is not a tautology. The placeholder sub-root is thirty-two zero bytes,
asserted against the literal, and no hash output can be all-zero. The amended dependency tables were
widened and not loosened, and still fail on drift in either direction. All 231 pre-existing cases.

## Findings

```findings
- file: crates/ekr-store/src/snapshot.rs
  line: 147
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "into_canonical copies the document's GraphRoot through without reading its space, so a document declaring Transient becomes canonical state, and the module doc's claim that documents are indistinguishable is pinned only for nodes"
- file: crates/ekr-store/src/snapshot.rs
  line: 133
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the crossing puts the caller's ontology beside the stored root's schema version without comparing them, so opening a store against another schema version silently re-types every folded record and moves no head root"
- file: crates/ekr-store/src/eventlog.rs
  line: 344
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "put keys on bytes alone and answers a later write with the earlier record, so canonical bytes that were cached first stay recorded as freely deletable, and the case named for the class writes two different payloads instead of the same one twice"
- file: crates/ekr-store/src/eventlog.rs
  line: 290
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the idempotency key is read from the stream version before the append and a landed append moves it, so a retried append computes a new key and writes the event a second time, which is the opposite of what the comment on that line claims"
- file: crates/ekr-store/src/log.rs
  line: 107
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "head's first sentence says None when nothing has been committed while the code returns the seed root for a seeded uncommitted lineage; the sentence is what should change, not the code"
- file: crates/ekr-store/src/log.rs
  line: 240
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "Fold::commit never applies a transaction's operations, so the folded knowledge root is always the seed's and the P1 exit criterion is reproducible only for lineages in which knowledge never changes"
```
