---
format: aep.planning-md/3
id: story:hosted-postgres-snapshot
kind: story
status: implemented
title: Open hosted PostgreSQL stores and preserve an initial SQLite snapshot
relations:
- serves: vision:o5
- serves: vision:o2
- derived_from: task:hosted-postgres-copy-and-read-serving
scope:
- confidence: cited
  path: .engineering/planning/story/workspace-crate-skeleton.md
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
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/migrate_cli.rs
- confidence: cited
  path: crates/ekr/tests/postgres_cli.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
- confidence: cited
  path: crates/ekr/tests/support/inode_tempdir.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/store.yaml
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T00:49:55Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T00:49:55Z", actor: "agent:codex", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-03T04:58:21Z", actor: "agent:codex", revision: 12, decided_on: {"recorded":{"test_result":5,"review_outcome":3}}}
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


## Reviewed integration result

Unit bf94be06bc is integrated at 37763e2ed2. The implementor report at
`<cache>/ekr-hosted-runtime/postgres/report.md` records 119 focused passes across 14 targets,
zero failed or ignored, including 13 real-provider cases and the required previous-release probe.
This resolves the pending copy-claim and checkpoint-admission findings above. Final formatting
and touched-package clippy passed. `review-result:hosted-postgres-final-2` records independent
inspection of the final corrections and the earlier direct provider rerun, without claiming
independent execution of the entire focused set. The broad collection was intentionally stopped
at a completed target boundary, exit 143; full combined workspace acceptance remains pending.
The corrected marker-evidence case also covers task:migration-marker-cannot-be-evidence.

## Final acceptance and release

The full local gate on 70761eaed64e3d5893a93d8ae3d5552c874a9987 passed: 2189 passed, 0 failed, 13 ignored across 358 completed runner summaries. All 10 recorded gate steps exited zero. Required real PostgreSQL and previous-release prerequisites were enabled; these scoped acceptance cases executed. Logs remain at `<cache>/ekr-hosted-runtime/release-gate/`. Existing ignored helper cases are not acceptance passes.

Exact-head GitHub Repository correctness and common / Security and privacy checks passed; correctness job https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37096359423/job/111126998509 completed successfully. Remote main and annotated 0.0.28 tag were verified at this commit. Release https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.28 was read back as published at 2026-10-03T04:56:40Z. This fulfills the scoped source-release contract, not a consumer deployment claim.

Acceptance cases are literal executable names unless otherwise noted. Kernel cases below live in `crates/ekr-kernel/tests/hosted_postgres.rs`; CLI/SDK cases in `crates/ekr/tests/postgres_cli.rs`.

| Acceptance line | Existing evidence and conclusion |
|---|---|
| `postgres_runtime_reopens_seed_and_committed_history` | Same-named real-provider case passes: admitted head and historical reads survive reopen and hosted read handles reject writes. `postgres_hosted_open_refuses_schema_authority_and_unbounded_budget` and `postgres_pool_overrides_cannot_remove_connection_or_timeout_bounds` pass for separate authority and finite bounds. Verified TLS configuration/opening is covered by the required TLS fixture plus the provider review, not by a new TLS protocol implementation. |
| `postgres_configuration_errors_never_expose_credentials` | Same-named CLI case passes; sanitized malformed-configuration error omits its synthetic secret-shaped input. Configuration path handling and sanitized provider error boundaries were inspected in `review-result:hosted-postgres-final-2`; this is not an exhaustive proof against arbitrary future diagnostics. |
| `sqlite_to_postgres_preserves_every_revision_schema_and_evidence_byte` | Same-named kernel case passes, advances the source after a single captured image, compares every captured revision's graph/roots, schema history, identities, retained bytes and hashes, and verifies original source events/bytes unchanged. `sqlite_to_postgres_cli_copies_and_reopens_one_history` asserts the machine-readable migration report. Logical roots remain equal; deliberately rewritten physical envelope hashes are mapped, not falsely asserted equal. |
| `copy_refuses_nonempty_destination_without_mutation` | Same-named case plus `copy_refuses_an_object_only_postgres_destination`, `interrupted_postgres_copy_is_unreadable_after_reopen`, `legacy_marker_evidence_cannot_complete_an_interrupted_copy`, `identical_seed_copies_cannot_finish_another_interrupted_history`, and `a_checkpoint_cannot_admit_a_copy_without_its_completion_receipt` pass. `previous_release_reads_ordinary_seeds_and_refuses_claimed_migrations` passes for the old-reader format boundary, including later commits/checkpoints. |
| `sdk_postgres_session_passes_only_configuration_paths` | Same-named real CLI/SDK subprocess case passes; existing constructor/configuration compatibility is also exercised by the combined SDK suites. |

Review anchor: `review-result:hosted-postgres-final-2`, with its exact candidate identities and bounded independent direct execution. Earlier incomplete broad runs remain historical evidence and are superseded for acceptance by the complete local gate, not reclassified as passes.
