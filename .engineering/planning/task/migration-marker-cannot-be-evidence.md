---
format: aep.planning-md/3
id: task:migration-marker-cannot-be-evidence
kind: task
status: draft
title: Evidence whose bytes equal a migration marker commits
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:migrate-reads-a-current-store
revision: 1
---
## What is wrong

Wave correct-07's adversary pass on unit M (`review-result:adversary-correct-07-m-pass-1`, case
`evidence_whose_payload_is_the_migration_marker_commits` in `crates/ekr-kernel/tests/adversary_c7_m.rs`,
ignored with this task's id): a validated transaction whose evidence payload is exactly the bytes of
the migration-started marker (`crates/ekr-kernel/src/migrate.rs:92`) is refused at commit as
`migrate-incomplete`, because the store recognises the marker by its content hash. Any proposer can
submit such a payload; the store stays readable. Pre-existing.

## Build

The migration markers are recognised by something a payload cannot equal: an object of a class or
stream no proposer can write, or a marker whose bytes carry a value no evidence entry can hold.

## Acceptance

- The pinned case passes: evidence whose payload equals a marker's bytes commits, on both providers.
- A store whose migration began and did not finish is still refused as `migrate-incomplete`.
