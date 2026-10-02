---
format: aep.planning-md/3
id: review-result:adversary-extract-06-x-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit X, pass 1
relations:
- reviews: story:explain-reads-an-index
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit X (`impl/explain-index` at `4dc51319`; cases committed as `d64e68dd`, four red and ignored). `cargo test -p ekr-kernel`: 490 passed, 7 ignored.

The release bench misses the 0.5 s bound on every lane (one-shot 3.85–4.97 s, nearly all of it opening the store; session 0.52–0.56 s; MCP 0.44–0.65 s); answer sizes pass (23–118 KB). Four altered in-memory reads that base refused are now answered or refused with another code; reaching them needs a caller editing its own in-memory read, since a store altered on disk fails at open. Every link answered is still verified.

Held: same-instant commits, a commit at the seed's instant, a clock going backwards (refused before the index), retract-then-restate, a three-step supersession, `--documents` matching the old records on all three lanes.

```findings
- file: crates/ekr/tests/explain_by_reference.rs
  line: 520
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "1x CPU medians miss the 0.5 s bound on every lane (one-shot 3.85-4.97 s, session 0.52-0.56 s, MCP 0.44-0.65 s) and the case is ignored, so no gate sees it"
- file: crates/ekr-kernel/src/explain.rs
  line: 362
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: an origin receipt with an altered commit instant is refused as origin-missing where base refused commit-record-disagrees
- file: crates/ekr-kernel/src/explain.rs
  line: 335
  category: property
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the revision-keyed index keeps one of two records claiming the same revision, so a duplicated origin record is answered where base refused origin-ambiguous
- file: crates/ekr-kernel/src/explain.rs
  line: 362
  category: property
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a forged record adding the same assertion at another instant is never read and the chain is answered where base refused origin-ambiguous
- file: crates/ekr-kernel/src/explain.rs
  line: 397
  category: property
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a forged retraction of an active assertion is never read and the chain is answered where base refused proposal-record-disagrees
```
