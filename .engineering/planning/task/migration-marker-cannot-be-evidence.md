---
format: aep.planning-md/3
id: task:migration-marker-cannot-be-evidence
kind: task
status: active
title: Evidence whose bytes equal a migration marker commits
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:migrate-reads-a-current-store
- decomposes: epic:zero-open-issues-2026-11
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
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

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-kernel` migration-marker recognition — cited
- **Files:** `crates/ekr-kernel/src/migrate.rs:92` (`MARKERS`; recognition in `required_markers` :118 and `finished` :145) — cited
- **Files:** `crates/ekr-kernel/tests/adversary_c7_m.rs:469` (`evidence_whose_payload_is_the_migration_marker_commits`) — cited
- **Symbols:** `MARKERS`, `legacy_markers`, `required_markers`, `finished`, `completion` in `migrate.rs` — cited
- **Also likely:** `crates/ekr-kernel/src/commit.rs:79` (only other caller of `required_markers`) — inferred
- **Also likely:** `crates/ekr-kernel/src/stage.rs:572` (writes the `completion` receipt) — inferred
- **Also likely:** `crates/ekr-kernel/tests/hosted_postgres.rs` (`migrate-incomplete` cases, `canonical_legacy_markers_are_carried_as_content_under_a_fresh_claim` :539) — inferred
- **Documents:** `systems/ekr/domains/kernel.yaml:334`, `docs/epistemic-knowledge-runtime-design.md:4930`, `CHANGELOG.md` — inferred, they state the `migrate-incomplete` rule
- **State in tree:** the pinned case is a plain `#[test]` with no `#[ignore]`, and its doc comment says the defect "once" occurred; the remaining work is probably verify and close — inferred
- **Confidence:** high — the story names the defect site and the test, and both resolve in the tree
- **Would collide with:** any unit touching `crates/ekr-kernel/src/migrate.rs`, the seed envelope's `migration` claim, or the commit-time required-object set in `commit.rs`
- **Safety fact:** new migrations are recognised by a `ekr.migration-finished/2` receipt bound to the seed's claim `EventId` and `seed_hash`, and legacy `/1` markers are dropped from the wanted set when an evidence payload has the same hash — step 2 (`<tree>/crates/ekr-kernel/src/migrate.rs:103-141`), unproven
