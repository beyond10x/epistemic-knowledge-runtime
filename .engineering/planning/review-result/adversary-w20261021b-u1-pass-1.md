---
format: aep.planning-md/3
id: review-result:adversary-w20261021b-u1-pass-1
kind: review-result
status: active
title: Adversary pass 1 on wave 2026-10-21b U1 (profile v4)
relations:
- reviews: story:supersession-verdict-fixes-replay-safe
revision: 1
---
Owners: F1-F4 are the U1 implementor's (`bb5b5b05b`); F5 is the coordinator's (the story's
acceptance, already narrowed by its coordinator decision 2).

Adversary pass 1 (`aep:adversary`) on U1 of wave 2026-10-21b, `bb5b5b05b` on `6e687d99`. One new
test file, `crates/ekr-kernel/tests/adversary_w20261021b_supersession_v4.rs` (4 cases, sqlite and
file stores, v4 only): 3 red (F1-F3), 1 green guard (F4).
`pinned.sh cargo test --locked -p ekr-kernel --no-fail-fast`: 706 passed, 3 failed, 5 ignored, exit 101.

| # | where | finding | sev | coordinator's routing |
|---|---|---|---|---|
| F1 | `crates/ekr-kernel/src/validate/candidate.rs:386` | `admits` accepts `by == node`, so a held `X by X` (refused `supersede-into-itself`) makes all four v4 D8 messages name X as what the head will hold | warning | fix in U1: v4 messages are fixed once released (ADR 0102 d1) |
| F2 | `validate/reference.rs:329` | v4 `supersede-by-superseded-node` follows the kept node into the refused supersession's own node and advises superseding OTHER by OTHER | warning | fix in U1 |
| F3 | `validate/merge.rs:198` | under v4 the merge check's own superseded set skips a refused supersession's node values while its edges count live, dropping a `wrong-type` | note | fix in U1 (build the set from the candidate's retiring set) |
| F4 | `validate/mod.rs:255` | the public `Pipeline::supersession_exact` validate path had no test; the added guard covers it | note | kept as is (test green) |
| F5 | story acceptance | the conformance suite holds no v4 scenario | warning | no change: coordinator decision 2 narrowed it |

Also reported, outside the ADR: `MergeEntity.evidence` (`crates/ekr-kernel/src/transaction.rs:161`)
still collapses duplicate ids under v4. ADR 0102 d3 names three fixes; the coordinator leaves it for
a later profile, beside the merge follow-ups H6 and H7.

```findings
[{"file":"crates/ekr-kernel/src/validate/candidate.rs","line":386,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"admits accepts by == node, so a held X-by-X refused as supersede-into-itself drives all four v4 D8 messages to say the transaction supersedes X by X"},
{"file":"crates/ekr-kernel/src/validate/reference.rs","line":329,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"v4 supersede-by-superseded-node follows the kept node into the refused supersession's own node and advises superseding OTHER by OTHER"},
{"file":"crates/ekr-kernel/src/validate/merge.rs","line":198,"category":"property","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"under v4 the merge check's own superseded set still skips a refused supersession's node values while its edges count live, dropping a wrong-type the merge alone gets"},
{"file":"crates/ekr-kernel/src/validate/mod.rs","line":255,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"no existing test calls the public Pipeline::supersession_exact validate path, so a Released mutant there survived until the added guard"},
{"file":".engineering/planning/story/supersession-verdict-fixes-replay-safe.md","category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"acceptance requires the conformance suite to hold v4, but no v4 host or scenario was added"}]
```
