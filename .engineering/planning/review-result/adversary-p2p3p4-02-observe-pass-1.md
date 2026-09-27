---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-02-observe-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit F (fixture records become observations)
relations:
- reviews: story:fixture-records-become-observations
revision: 1
---
## Adversary pass 1 — unit F (`story:fixture-records-become-observations`), HEAD `2e62665e`

Verdict: red. Cases executed 4→8, red 1
(`crates/ekr-observe/tests/adversary_observe.rs::one_source_record_observed_at_two_times_gets_one_id`).
Origin: introduced 4, pre-existing 0, undecided 0. Four mutants run on a scratch copy survive the
unit's own suite (blank branch deleted at lib.rs:113, UUID v8 builder swapped at :62, `FeedItem`
swapped at :128, key field order changed at :44); the three new green cases kill the first three.
Attacked and not broken: line numbering and trailing newline, key encoding injectivity (length-prefixed
strings, tagged optionals), duplicate and unknown fields, integer-only `captured_at`, same bytes twice.
Note without a case: a trailing `\r` stays in the hashed bytes, so CRLF files give other ids; nothing
reaches it.

Coordinator routing (2026-09-27):
- F1 → the case is wrong now. `observe.yaml:45-56` declares the key as source, source-native id and
  content hash, and the story declares `content_hash` as `ContentHash::of_bytes` over the line's bytes;
  the record's timestamp is part of the record (story: "one message: source identity, source-native id,
  timestamp, text"). Two lines that differ in their timestamp are two records. The implementor rewrites
  the case to assert that decision (different record bytes, different id) and states it in the key doc.
- F2 → no-op for the implementation; the three green cases stay.
- F3 → back to the implementor: pin a golden id for fixture line 1.
- F4 → the coordinator's: `crates/ekr-core/src/identity.rs` is outside the unit; its module doc gains
  the derived observation id (design § 56) at the wave close.

```findings
[
{"file":"crates/ekr-observe/src/lib.rs","line":124,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the key hashes the whole line including captured_at, so one source record captured twice gets two ids, against the key doc at :28 and design section 56; nothing reaches it yet"},
{"file":"crates/ekr-observe/src/lib.rs","line":113,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the unit's suite lets three mutants live (blank branch deleted, UUID v8 builder swapped at :62, FeedItem kind swapped at :128); the new green cases kill all three"},
{"file":"crates/ekr-observe/src/lib.rs","line":44,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"no pinned id vector, so reordering the key encoding changes every id and both suites stay green; determinism is only checked inside one process"},
{"file":"crates/ekr-core/src/identity.rs","line":5,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the ekr-core identity doc says ids are never derived and from_uuid says minted before, but this unit derives v8 observation ids through from_uuid"}
]
```
