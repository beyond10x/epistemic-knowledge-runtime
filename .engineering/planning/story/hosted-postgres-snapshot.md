---
format: aep.planning-md/3
id: story:hosted-postgres-snapshot
kind: story
status: active
title: Open hosted PostgreSQL stores and preserve an initial SQLite snapshot
relations:
- serves: vision:o5
- serves: vision:o2
- derived_from: task:hosted-postgres-copy-and-read-serving
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: crates/ekr-kernel
- confidence: cited
  path: crates/ekr-sdk
- confidence: cited
  path: crates/ekr-store
- confidence: cited
  path: crates/ekr/src/cli
- confidence: cited
  path: crates/ekr/tests/adversary_sdk01_h_replaced_store.rs
- confidence: cited
  path: crates/ekr/tests/adversary_tests_under_load.rs
- confidence: cited
  path: crates/ekr/tests/migrate_cli.rs
- confidence: cited
  path: crates/ekr/tests/postgres_cli.rs
- confidence: cited
  path: crates/ekr/tests/support/inode_tempdir.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/store.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T00:49:55Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T00:49:55Z", actor: "agent:codex", revision: 5}
---
## Outcome
Expose the existing eventlog PostgreSQL provider through the runtime, CLI and SDK; initialize an empty hosted store from one consistent SQLite snapshot. Existing filesystem stores remain compatible. This is the snapshot-only hosted foundation. Incremental suffix publication and network serving remain separate work.

## Acceptance
Named executable cases to implement against a real ephemeral PostgreSQL server using invented fixtures:
- postgres_runtime_reopens_seed_and_committed_history: verified TLS, application role without schema authority, bounded pool and restart preserve admitted head and historical reads.
- postgres_configuration_errors_never_expose_credentials: CLI arguments and errors carry configuration paths and sanitized diagnoses, never credentials.
- sqlite_to_postgres_preserves_every_revision_schema_and_evidence_byte: forced single SQLite image capture and existing kernel migration compare all logical roots, identities, schema history and retained payload bytes; source unchanged.
- copy_refuses_nonempty_destination_without_mutation: unrelated initialized target is refused; incomplete copy remains unreadable after restart.
- sdk_postgres_session_passes_only_configuration_paths: launched child has backend plus config-file references, compatible existing configuration constructors.
Tests assert surviving invariants after implementation; none deliberately fails or is ignored on the final branch. Explicit prerequisites may skip only outside the required real-PostgreSQL acceptance run.

## Scope
Confidence high; scoped through read-only story-scoper inspection.
- cited: crates/ekr-store/src/eventlog.rs and crates/ekr-kernel/src/runtime.rs currently dispatch File/Sqlite only.
- cited: crates/ekr-kernel/src/migrate.rs retains source identities and compares logical roots through admitted replay, with incomplete-copy markers.
- cited: crates/ekr-store/src/read_only.rs contains consistent SQLite image capture; ordinary multi-read inventory is not itself a snapshot.
- cited: crates/ekr/src/cli/mod.rs, crates/ekr/src/cli/migrate.rs and crates/ekr/src/cli/session.rs own configuration, destination selection and held readers.
- cited: crates/ekr-sdk/src/session.rs holds SDK subprocess configuration.
- inferred: dependency manifests, targeted provider/copy/CLI/SDK tests, docs/cli.md and systems/ekr/domains/store.yaml must follow those surfaces.

## Contract and boundary
Reuses existing store domain, runtime authority and preserving migration. No new knowledge entity. Configuration is an operational provider input. Kernel remains the only canonical writer. Verified TLS is mandatory for hosted mode; schema setup is explicit and separate from application open. Physical record hashes rewritten by existing legacy-envelope migration are reported, not falsely declared equal. Logical content roots and retained evidence remain equal.

## Authorization
Operator explicitly authorized implementation and publication of missing generic upstream capabilities. Fixtures, docs and commits remain entirely generic. Shared identity, paid extraction and deployment-specific details are outside this story.

## Review-driven copy contract
Real PostgreSQL testing exposed that the datafeed watermark is not a valid negative observation for destination emptiness; use the provider's consistent tenant capture before the atomic initializer guard. Independent correctness review also reproduced fixed completion-marker bytes appearing as ordinary retained evidence, incorrectly admitting an interrupted destination. Migration now requires a fresh per-copy claim in seed envelope /4 and a Canonical completion receipt bound to both claim and destination seed hash. Ordinary seeds remain /3. Old binaries explicitly refuse /4. Preserve legacy marker reading separately and test both evidence and arbitrary carried Canonical marker-shaped content, plus copying an already migrated source. Additional actual surfaces are seed.rs and commit.rs within the declared ekr-kernel scope. These corrections are pending execution, not completed acceptance.

## Compatibility execution finding
The required previous-release probe executed against 0.0.27 and observed an old reader returning a head for a /4 destination through its checkpoint shortcut. Envelope rejection alone was insufficient. Corrective contract: derive a /4-state flag from admitted seed on replay and checkpoint restore, emit checkpoint-binding/2 for all such states including later commits, and keep legacy fast-head limited to /1 so new-format reads pass through seed/completion admission. Test both old-reader rejection and an interrupted copy whose final receipt was not published after its checkpoint. Ordinary /3 fast-head remains unchanged. checkpoint.rs and replay.rs are additional paths within the existing kernel scope. This finding remains pending a green execution result.
