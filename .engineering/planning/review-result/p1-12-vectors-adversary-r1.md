---
format: aep.planning-md/1
id: review-result:p1-12-vectors-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-12 vectors unit
relations:
- reviews: story:version-persisted-contracts
- reviews: task:two-revision-events-have-no-discriminator
revision: 1
---
Adversary pass 1 on unit p1-12-vectors (wave p1-12), `aep-drive:adversary`, against the unit's 8
untracked `current_*` test files on `e6af002` (no `src/` change).

Owners: 3 findings, 3 pre-existing (the coordinator's identity obligation named only the `publish` path).

Verdict: CONFIRMED. cases: executed 294→298, red 2.

Cases in `crates/ekr-store/tests/adversary_p1_12_vectors_identity_preparation.rs` (2, red):
a retained `event_id` reused with different content through `prepare` + `resume`, the kernel's only
write path, is `Written` on both providers when the original was retained at attempt 1, after which
every history read refuses with `revision-envelope-disagrees`; at attempt 0 it is refused only as
`Backend("idempotency key … was already used for a different request")`. Green cases in
`crates/ekr-kernel/tests/adversary_p1_12_vectors_seed_{sensitivity,envelope}.rs`: six further seed
mutations each move exactly the sub-roots they feed; the retained seed envelope has the §89/§91.2
layers.

What reaches the red path: the public `RevisionLog::prepare`/`resume` API. The kernel mints every
event id with `EventId::mint()`, so no kernel command reuses one (inferred from
`commands.rs:252,319,417,438` and `commit.rs:267`).

Hand-decoded pinned vectors match the canonical tag table; no vector pins a bug.

```findings
- file: crates/ekr-store/src/preparation.rs
  line: 910
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: resume of a preparation whose decision reuses a retained event_id with different content writes a duplicate occurrence on both providers and every later history read refuses with revision-envelope-disagrees; no kernel path mints a reused id.
- file: crates/ekr-store/src/preparation.rs
  line: 447
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: at attempt 0 the reuse is refused only by the eventlog idempotency key as Backend(...), never with the §89 occurrence-identity-conflict code.
- file: crates/ekr-store/tests/current_occurrence_identity.rs
  line: 1
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the coordinator's identity obligation is exercised only through RevisionLog::publish, which no kernel command calls, so the kernel's prepare/resume write path has no identity case.
```
