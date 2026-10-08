---
format: aep.planning-md/3
id: review-result:hosted-postgres-static-1
kind: review-result
status: active
title: Hosted PostgreSQL independent static correctness review
relations:
- reviews: story:hosted-postgres-snapshot
revision: 1
---
unit: story:hosted-postgres-snapshot — uncommitted ekr-hosted-postgres candidate against 51d42cd1a8
verdict: nothing found in the requested static review
cases: executed 0→0 by this reviewer, red 0; existing evidence records 11 hosted + 2 migration cases
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 report path
needs-coordinator: finish pending compatibility/regression/clippy gates and freeze exact candidate

1. Reviewer diff: none. No implementation or test files were changed. The candidate's existing diff includes the provider configuration, runtime/SDK/CLI integration, migration claims and tests; those edits belong to the implementor. This is the coordinator-requested read-only static pass, not a newly executed adversarial verifier or an approval.

2. No cases added or executed, per the assigned read-only/no-second-build restriction. No credentials or provider fixture configuration files were read. The candidate remained under active implementation during review; `inventory_requires_capture` and the previous-release case appeared during this pass, so no immutable commit is represented by this report.

3. Existing evidence inspected (not rerun): `<cache>/ekr-hosted-runtime/postgres/pg-kernel-collision-disk.log` records 11/11 hosted cases and 2/2 local migration regressions passing. These include marker-shaped evidence, Canonical legacy markers, identical-seed competing copies, ordinary competing seed, object-only destination refusal, captured-source advancement, per-revision graph/schema/identity/evidence comparisons and reopen. `<cache>/ekr-hosted-runtime/postgres/pg-cli.log` records 3/3 CLI/SDK cases; the newly added previous-release compatibility case is not in that log. Clippy was still compiling in the inspected log; no clippy pass is claimed.

4. No additional concrete defect established by this pass.

5. Boundaries inspected without establishing another break:

- `/4` binds a fresh claim into the atomically published seed. The completion object's content hash binds both claim and destination seed hash; source evidence containing legacy fixed marker bytes cannot complete that claim.
- Only `initialize` returning Written lets that migration continue. A losing initializer does not publish legacy start markers and cannot mark a competing history complete.
- `finished` runs before cached/checkpoint replay. The copying authority's temporary migrating flag permits its own verification; a separately opened authority still requires the receipt.
- Ordinary seeds remain `/3` with the migration field omitted. `/4` decoding requires a present claim and `/3` requires its absence, giving older readers a format boundary.
- SQLite CLI copying forces a consistent captured image. The tests advance the original after capture and compare every retained revision plus evidence. PostgreSQL as a source is explicitly refused; low-level PostgreSQL inventory now also refuses rather than trusting a lagging feed.
- Destination emptiness uses native provider capture, including objects, rather than only revision history. Its metadata allocation is explicitly documented; competing seed authority remains the atomic initialize operation.
- PostgreSQL reads reject provider writes at the shared publication seam; replay checkpoint writes are suppressed. Reader identity checks do not follow the configuration file inode. Writer sessions are explicitly distinct.
- Configuration parsing has bounded file reads and sanitized diagnostic paths. Verified TLS, schema selection and role/pool checks delegate to the provider. No credential values were accessed during review.
- SDK addition preserves file/SQLite variants and adds an explicit PostgreSQL backend. CLI schema management is a separate verb, excluded from session dispatch.

Residual evidence limits: no independent mutation/probe, no latest previous-release execution observed, no completed full regression/clippy gate observed, and no live large-store copy/load test. Those are unclaimed checks, not inferred failures. HTTP serving remains a separate story.

6. Outside-worktree write:
`<cache>/ekr-hosted-runtime/serving/postgres-review.md`

```findings
[]
```
