---
format: aep.planning-md/3
id: review-result:hosted-context-guard-final-1
kind: review-result
status: active
title: PostgreSQL constructor runtime-context guard review
relations:
- reviews: story:hosted-postgres-snapshot
revision: 1
---
unit: PostgreSQL synchronous-runtime boundary guard correction
verdict: pass, bounded static review
Owners: 0 findings, 0 coordinator, 0 implementor.

Reviewed commit cc1c43db650e75bd7aef4e0e880b94eee23f917e, limited to
crates/ekr-store/tests/runtime_context.rs. File-diff SHA256:
f62f30fe18914d716f5866e7ce6dbee000f6975d4fef18b6bfcd93eb0a37c3bb.
Disclosure: this reviewer authored the preceding EKR PostgreSQL consumer implementation;
the coordinator authored this test correction. No builds, test executions, database calls or
repository edits were performed in this review.

The correction supplies actual coverage, not only allowlist names. The constructor case calls
postgres_schema and postgres in both reading=false and reading=true modes. Its check closure runs
both under ambient.enter() and inside ambient.block_on(), covering entered and actively running
Tokio contexts. The shared refusal helper requires StoreError::RuntimeContext and its exact
message; any successful open, configuration error, provider error or panic fails the case.

Both connection and CA paths are nonexistent children of a new TempDir. PostgresConfiguration
construction itself performs no I/O. Moving config.provider() ahead of new_runtime() would yield
a referenced-file error rather than the required RuntimeContext, so this fixture distinguishes
the intended ordering. The unchanged directory-empty assertion also rejects created artifacts.
Source corroborates that eventlog.rs:550 and :568 call new_runtime first, whose first operation
is ensure_sync_context, before credential reads, trust roots or provider connection. This is
source-plus-refusal evidence, not syscall instrumentation of every possible ignored read.

The every-file/every-SQLite guards continue exercising their existing operations and comparing
the exact declared-entry-point set. Their constructor list now names postgres/postgres_schema
because the dedicated constructor case directly covers them. No authority callback count,
retention assertion, recovery-after-refusal assertion, equality guard or previous case was removed.
No live PostgreSQL fixture is needed to test this pre-I/O boundary.

Coordinator reported the tail run's two stale-set failures; this review did not rerun that tail
or claim a corrected green result. Remaining execution evidence belongs under <cache>/release-gate
and must be recorded by the coordinator before release. Private instance preparation is unchanged.

```findings
[]
```
