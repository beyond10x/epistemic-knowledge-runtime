---
format: aep.planning-md/1
id: review-result:adversary-eventlog-store-pass-2
kind: review-result
status: active
title: Adversary, story:eventlog-store, pass 2
relations:
- reviews: story:eventlog-store
revision: 1
---
## Pass

Adversary pass 2 over `story:eventlog-store`, wave p1-05 unit 1, against correction round 1,
uncommitted at head `f981a07`. Opus, 237k tokens, 43 tool uses, 3m59s.

279 cases before, 283 executed, 4 red across three new files. `fmt-check`, `clippy` and `spec-check`
all exit 0, so the red is the assertions. It did not run `task check`, because that gate's last step
is an `aep plan artifact` command its charter forbids it — which is correct and worth recording.

**All six pass-1 findings resolved.** All five pass-1 cases green without edit. Two were generalised
past what the reviewer reported, by the implementor rather than by a finding.

## What it could not break

It walked `GraphDocument` and `GraphRoot` field by field against the implementor's claim to have
enumerated the class exhaustively, and found exactly one entry misplaced — finding 10 below.
Everything else on the copied-through list genuinely has nothing to compare against. The four new
refusals each fire by their own route and each name what went wrong, across all four maps. Every
mutant the retention ladder's brief named is already guarded, including the derived-`Ord` trap. The
unreachability claim holds: the append uses an unconditional expectation so a conflict cannot arise,
and the key is the request hash so a mismatch needs a hash collision. The changed fold helper is a
fix and not a weakening. All 279.

One asymmetry it declined to raise as a finding: evidence filing is the only check that runs after
values are narrowed, so a document with a misfiled evidence and a float elsewhere reports the float.

## Findings

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 77
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "the crate writes ekr.store.ObjectRetentionRaised with a single storage_class field while the domain declares it with content_hash, from and to, so the stored bytes carry none of the declared names and a reader cannot see the ladder direction the declaration says is carried"
- file: systems/ekr/domains/store.yaml
  line: 80
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "the events section declaring the retention raise existed only in the coordinator tree, so the specification validator and the projection guard both ran against an implementation tree whose store domain declared no events at all"
- file: crates/ekr-store/src/eventlog.rs
  line: 398
  category: property
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "TransactionRejected and TransactionStale carry no minted id, so a second rejection of a re-proposed transaction is byte-identical to the first and the append returns Ok without writing it, and the fold then commits the transaction it refused"
- file: crates/ekr-store/src/snapshot.rs
  line: 38
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "root.parent is listed as having nothing in the document to disagree with, and root.id is three lines above it, so a root naming itself as its own parent crosses into canonical state"
```
