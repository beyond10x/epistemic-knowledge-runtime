---
format: aep.planning-md/3
id: review-result:hosted-store-guard-final-1
kind: review-result
status: active
title: PostgreSQL constructor commit-authority guard review
relations:
- reviews: story:hosted-postgres-snapshot
revision: 1
---
unit: PostgreSQL constructor classification in the store commit-authority guard
verdict: pass, bounded static review
Owners: 0 findings, 0 coordinator, 0 implementor.

Reviewed commit ab16ae0f0c06a6200193bc0270fa914d3ad3af60, which adds only the
postgres/postgres_schema classifications and explanatory comments in
crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs:230–251.
Commit file-diff SHA256: 842054569746f33048ccfebe642b2e86acbe49421444abfcb6e3b566c1d55bc7.

Disclosure: this reviewer authored the earlier EKR PostgreSQL consumer implementation.
This is a separate static review of the coordinator's guard correction, not an independent
re-review of all PostgreSQL implementation behavior. No tests, builds, database calls or source
edits were performed during this review.

Classification is appropriate:
- eventlog.rs:547 postgres_schema calls PostgresEventStore::migrate with an empty projection
  specification list. The pinned provider at fe8a0a7 uses schema DDL and schema metadata,
  defaulting to RefusePopulated for predecessor blob migration. It neither supplies nor
  appends an EKR revision occurrence. This is an operational database writer, correctly
  classified here by the narrower criterion “cannot write an occurrence.”
- eventlog.rs:563 postgres opens/adopts an existing provider, assembles its handle and sets
  read-only/inventory/shutdown callbacks. The installed empty-inventory callback can allocate
  provider stream identity metadata before capture; that is not canonical history, and merely
  installing the callback does not run it. Provider open validates TLS/budget/role/schema and
  performs no occurrence append.
- These constructors expose the existing generic EventlogStore<PostgresEventStore> methods;
  no PostgreSQL-specific publication bypass is introduced. The generic publish path verifies
  the proposed retained history through CommitAuthority before native append (eventlog.rs:1892),
  initialize delegates through publish, and prepare/resume retain their existing checked paths.
  Existing story_contract coverage also includes eventlog-postgres in its native-writer guard.

Coverage is preserved: the exact declared-entry-point set equality remains, all forged commit
calls remain, and the test still asserts a seed head plus exactly three retained occurrences
(seed/proposal/validation, no commit). Only the two previously unclassified non-occurrence-writing
constructors were added. No new occurrence-writing entry point needs an additional forged-commit
invocation in this guard. This does not claim the SQLite-based attack itself executed against a
PostgreSQL fixture; that is a separate provider/runtime acceptance concern.

Evidence: inspected <cache>/release-gate/store-guard-red.log. The single guard failure lists
postgres and postgres_schema as the only extra declared names; the test reached that equality
after its forged publication assertions. No green execution is claimed here. Coordinator owns
the remaining package/full-gate runs and publication.

```findings
[]
```
