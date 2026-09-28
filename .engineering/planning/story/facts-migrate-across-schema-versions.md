---
format: aep.planning-md/3
id: story:facts-migrate-across-schema-versions
kind: story
status: draft
title: Existing facts migrate to a new schema version and their history stays readable
relations:
- serves: vision:o6
- decomposes: epic:p5-frontier-schema-scheduler
revision: 1
---
## Context

When the schema advances, existing facts must follow it while their history stays queryable
(asked by a consumer instance, 2026-09-28). The roadmap gives `ekr-ontology` "compatibility
mappings, migration plans" (roadmap § 3) and P5 a `SchemaProposal` with a migration plan; no story
exists for moving existing facts to a new schema version.

## Build

A migration plan attached to a schema transaction maps facts written under version n to version
n+1 (property renames and re-typing, type merges and splits); applying it commits the migrated
facts as a revision whose assertions supersede the old ones, so every earlier revision still reads
as it was.

## Acceptance

- After a migration, a snapshot at the new head shows every affected fact under the new version,
  and a snapshot at the revision before shows it unchanged, on both providers.
- `ekr explain` on a migrated assertion names the assertion it supersedes and the schema change
  that caused it.
- A plan that leaves a fact unmapped is refused by name before anything commits.
