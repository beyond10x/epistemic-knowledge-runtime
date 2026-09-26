---
format: aep.planning-md/2
id: review-result:p1-12-recovery-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-12 recovery unit
relations:
- reviews: story:commit-and-revision-lineage
revision: 1
---
Adversary pass 2 on unit p1-12-recovery (wave p1-12), `aep-drive:adversary`, against the
correction to `commit.rs:244-258` and `preparation.rs:624-640`.

Owners: 2 findings, 2 pre-existing.

Verdict: NEEDS-CHANGE. cases: executed 289→292, red 1.

Cases: `crates/ekr-kernel/tests/adversary_p1_12_recovery_r2_anchor_race.rs` — a second host with the
original anchor elects the seed inside this host's clock sample (red on both providers:
`Other("AuthorityMismatch")` instead of `PublicationInputConflict`; nothing written, the other seed
still resumes; same result on an `e6af002` copy). `crates/ekr-store/tests/adversary_p1_12_recovery_r2_identity.rs`
— a landed third attempt retried from a fresh handle is `AlreadyRecorded` (green); an identical
retained decision under another slot is refused as `occurrence-already-retained` (green, red on a
mutant that removes that branch).

Attacked without a break: a genuine retry is never refused by the identity check (providers dedupe
on the idempotency key before the expected version); the check holds at every attempt; the
Bootstrap prefix is empty so the mapping cannot hide canonical corruption; no third append path.

```findings
- file: crates/ekr-kernel/src/commit.rs
  line: 248
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the AuthorityMismatch-to-PublicationInputConflict mapping covers only the slot pre-read, so a different-anchor election landing during the clock sample still surfaces raw AuthorityMismatch from store.prepare on both providers
- file: crates/ekr-store/src/preparation.rs
  line: 637
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: no pre-existing case executes the occurrence-already-retained branch; removing it leaves both package suites green while an identical decision under an unused native key is recorded twice and history stops reading
```
