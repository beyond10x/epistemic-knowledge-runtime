---
format: aep.planning-md/3
id: review-result:adversary-sdk-03-e-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-03 unit E, pass 1
relations:
- reviews: story:sdk-evidence-attachment
revision: 1
---
## Verdict

NEEDS-CHANGE on `f1aced77`, four findings; the operation-cap defect and the restart duplicates fixed in
`7ef3efea` (`EvidenceSet::from_store` rebuilds the set from `ekr snapshot`), the two unit gaps covered
by the adversary's cases in `40287683`. Held under attack: first, middle, last and all citers
rejected across batches, a batch-wide refusal, a Stale retry, a lost commit reply settled as
documented, one set reused across two runs, hashes equal to `ekr hash` for empty, 16,384-byte,
16,385-byte and non-UTF-8 payloads.

```findings
- file: crates/ekr-sdk/src/batch.rs
  line: 391
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "an entry moved by a rejection pushed a later proposal past the with_limits operation cap; submit checked only bytes"
- file: crates/ekr-sdk/src/evidence.rs
  line: 124
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no unit case called mark_committed, so a no-op passed"
- file: crates/ekr-sdk/src/evidence.rs
  line: 52
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "every unit item differed in observed-at time, so a key without source or time passed"
- file: crates/ekr-sdk/src/evidence.rs
  line: 57
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "EvidenceSet could not be rebuilt, so a consumer that restarts added a second entry for every committed item"
```
