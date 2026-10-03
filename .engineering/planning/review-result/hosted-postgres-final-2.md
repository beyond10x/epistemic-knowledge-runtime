---
format: aep.planning-md/3
id: review-result:hosted-postgres-final-2
kind: review-result
status: active
title: Hosted PostgreSQL correction review with focused regression evidence
relations:
- reviews: story:hosted-postgres-snapshot
revision: 1
---
unit: story:hosted-postgres-snapshot — frozen ekr-hosted-postgres working tree against 51d42cd1a8
verdict: nothing found
cases: executed 12→12, red 0; existing cases rerun, no new cases authored
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (report, execution log, private temporary directory)
needs-coordinator: finish broad regression/lint gates; integrate the reviewed candidate before HTTP wiring

Owners: 0 findings, 0 coordinator, 0 implementor.

1. Reviewer diff: none. No implementation or test files changed. This is the coordinator-requested final static delta review plus direct execution of an existing compiled suite; it is not a newly authored adversarial verifier, a human approval, or a claim that a full gate passed. The initial static-only record remains unchanged at `<cache>/ekr-hosted-runtime/serving/postgres-review.md`.

Candidate identity: base HEAD `51d42cd1a8`; SHA-256 of `git diff --binary 51d42cd1a8` is `012dde47414f99e41a59798cf984478450db688981d29ae076ebe5c0f0040713`. Untracked candidate files, which Git's ordinary diff excludes:

- `crates/ekr-kernel/tests/hosted_postgres.rs`: `3c674273811ffc1180ac9658d83ca7041dcb6cd24eddb13249819529c70992d9`
- `crates/ekr-kernel/tests/support/hosted_store.rs`: `d97e498d352a7170e488cc29a7c4acd9e436097fcf86b4840788db8aaae239df`
- `crates/ekr-store/src/postgres.rs`: `52657c0e262bad66a33f9dcebc260102ba33c7b23567c6ed078279a5f66bd9b1`
- `crates/ekr/tests/postgres_cli.rs`: `e26a2482d3139fb3325aefea8da7387fc6bbc90aefe7149934fa2a5390f4c2d4`

2. No test cases added, as assigned. The coordinator explicitly requested no Cargo invocation or source edits, and reuse of the already-built test executable. Its SHA-256 is `9790b2d664de9fb6bf938a83e46e76280e49ccef6ce2cef956a6a74fb371957d`. Credential files were neither read nor printed; only the assigned paths were passed through environment variables. The fixture owner confirmed no simultaneous PostgreSQL lane and reserved the fixture for this run; it was released immediately after completion.

3. Direct execution command (personal roots normalized):

```sh
EKR_TEST_POSTGRES_CONFIG=<cache>/ekr-hosted-runtime/postgres/fixture/app.json \
EKR_TEST_POSTGRES_OWNER=<cache>/ekr-hosted-runtime/postgres/fixture/owner.json \
EKR_REQUIRE_POSTGRES=1 \
TMPDIR=<cache>/ekr-hosted-runtime/serving/tmp \
/dev/shm/ekr-hosted-postgres-target/debug/deps/hosted_postgres-95c2f23d07589aa3 \
  --test-threads=1 --nocapture
```

Verbatim output:

```text
running 12 tests
test a_checkpoint_cannot_admit_a_copy_without_its_completion_receipt ... ok
test a_claimed_copy_can_be_captured_and_copied_again ... ok
test canonical_legacy_markers_are_carried_as_content_under_a_fresh_claim ... ok
test competing_ordinary_seed_is_not_poisoned_by_losing_copy ... ok
test copy_refuses_an_object_only_postgres_destination ... ok
test copy_refuses_nonempty_destination_without_mutation ... ok
test identical_seed_copies_cannot_finish_another_interrupted_history ... ok
test interrupted_postgres_copy_is_unreadable_after_reopen ... ok
test legacy_marker_evidence_cannot_complete_an_interrupted_copy ... ok
test postgres_hosted_open_refuses_schema_authority_and_unbounded_budget ... ok
test postgres_runtime_reopens_seed_and_committed_history ... ok
test sqlite_to_postgres_preserves_every_revision_schema_and_evidence_byte ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.78s
```

Exit: 0. Executed 12→12: before comes from the implementor's `focused-binding.log` summary; after comes from this direct run. The count is unchanged because this pass intentionally reran existing cases. With `EKR_REQUIRE_POSTGRES=1` and `--nocapture`, this run reported no prerequisite skip.

Separately inspected, not rerun: `<cache>/ekr-hosted-runtime/postgres/focused-binding.log` records 3 migration CLI + 4 PostgreSQL CLI + 12 hosted kernel + 2 local migration cases = 21 passed, 0 failed, 0 ignored. This includes the previous-release case, whose source checks ordinary `/3` acceptance, immediate `/4` refusal, refusal after a subsequent commit, and refusal after an actual newer checkpoint is written following restore. The coordinator identifies that run as using the required previous-release binary; this reviewer did not execute that binary or independently inspect its launch environment.

4. No additional concrete finding established.

5. Final delta inspected:

- Both checkpoint publication paths derive binding version from `ReplayState::migration_claim`. Ordinary seeds retain binding `/1`; claimed migrations use `/2`.
- The constant-work `head_by_binding` path recognizes only `/1`, so `/2` falls through to seed/receipt admission. The directly executed interruption case proves that an existing checkpoint cannot make an incomplete copy readable after reopen.
- Full seed replay obtains the claim from the admitted envelope. Checkpoint restore fills the claim from that same envelope before caching the reconstructed state. Both `ReplayState` constructors and all `checkpoint_binding` callers were enumerated; state cloning preserves the claim across subsequent replay/commits.
- `replay_from` calls migration completion admission before returning cached state. Named-envelope decoding and checkpoint outline decoding both require `/4` for a present claim, so the restore path does not silently erase the format boundary.
- The compatibility regression verifies that a later checkpoint was actually published and differs from its predecessor before retrying the older reader; it does not merely test another open of the original checkpoint.
- Existing migration tests cover fresh competing claims, ordinary competing seed, marker-shaped evidence, object-only destinations, repeat copies, complete retained history/schema/evidence and non-writing hosted reads. This pass reran those existing cases without modifying them.

Limits: no new mutation or adversarial case was authored, no full-suite/lint result is claimed, and no large production-shaped copy/load or hosted HTTP acceptance was performed here.

6. Outside-worktree writes:

- `<cache>/ekr-hosted-runtime/serving/postgres-final-review.md`
- `<cache>/ekr-hosted-runtime/serving/postgres-final-tests.log`
- `<cache>/ekr-hosted-runtime/serving/tmp/` (private disk-backed test temporary directory; test-owned temporary children are removed by their guards)

7. Subsequent fixture-only delta review, requested while the full gate runs:

No additional finding. No source or test edits and no execution by this reviewer in this addendum. The original candidate hashes and direct 12-case result above describe the earlier frozen candidate; this addendum extends review only to the three files below.

- `crates/ekr/tests/support/inode_tempdir.rs` adds one explicit `EKR_INODE_TEST_TMPDIR` override, falling back to `std::env::temp_dir()`. It creates an ordinary guarded temporary directory under that root and fails on an invalid/unwritable root; it adds no fallback that could hide a fixture failure. SHA-256: `c4779a4f254fbe2495439c97ba1c436c20150866f8e5682b3b78ed1f8b824aeb`.
- `crates/ekr/tests/adversary_sdk01_h_replaced_store.rs` uses that helper in `World::seeded`, so the actual reuse regression and its replacement source use the selected filesystem. The two replacement variants, existing reported skip path, three-call bound, expected head 2, vanished-node assertion, WAL checks and surviving-history checks are unchanged. SHA-256: `fd7a635fd4774c15b0ae25614e25d17a8799596ed1d015ec2614dc10de8d8ef3`.
- `crates/ekr/tests/adversary_tests_under_load.rs` uses the same helper and reports its actual root. Its 10 attempts, 20,000 candidate-directory ceiling and `staged > 0` assertion are unchanged; inability to reproduce inode reuse still fails the guard. SHA-256: `055b1d09fd4adb4e4cd4101e70c8959ce5d529b9e210ab23dc69d315de32867f`.

Coverage was not weakened by this diff: the guard and the actual behavior case now select the same explicitly chosen filesystem, while all behavioral assertions remain. This delta changes no runtime file.

Inspected existing evidence, not rerun: `<cache>/ekr-hosted-runtime/postgres/inode-explicit.log` contains:

```text
running 3 tests
test adversary_h_closing_the_replaced_sqlite_store_leaves_the_wal_at_the_path_alone ... ok
test adversary_h_a_session_follows_a_replacement_once_another_process_committed_its_proposal ... ok
test adversary_h_mcp_answers_from_a_file_store_replaced_under_the_same_inode_without_a_restart ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.39s

running 1 test
freed inode handed back in 10 of 10 stagings under <cache>/ekr-hosted-runtime/postgres/tmp
test the_replaced_store_variant_that_can_skip_is_staged_at_least_once_in_ten ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

There is no skip diagnostic in that log. `<cache>/ekr-hosted-runtime/postgres/clippy-inode.log` records the targeted clippy invocation reaching `Finished dev profile`; the coordinator reported exit 0 and formatting exit 0. No new full-gate result is inferred. This addendum writes only the present review report and leaves the initial static record unchanged.

8. Final correction-delta review (static only; no Cargo or PostgreSQL execution by this reviewer):

No additional finding. The adaptations preserve the tested contracts instead of accepting arbitrary changed output. The coordinator supplied ownership of the AEP amendment; all source/test corrections below belong to the PostgreSQL implementor. This reviewer changed only the present report. HTTP preparation remains in its separate worktree and scratch patch.

| File | Owner | SHA-256 at review |
|---|---|---|
| `crates/ekr/src/cli/mod.rs` | PostgreSQL implementor | `fddb601215dad29da330a174e2b21ad61448b65c480a5aecacbcef95c48bcfc2` |
| `crates/ekr/tests/agent_cli.rs` | PostgreSQL implementor | `a643cc417bc80bf36fcedec4c8819d5fe5391864c4e71d0a7be7c71cfc45125d` |
| `crates/ekr/tests/story_contract.rs` | PostgreSQL implementor | `d702cb15fea21bd04911903d7deca16a72e39e77d17d180f9cb1cdea78cac0ae` |
| `crates/ekr-kernel/tests/hosted_postgres.rs` | PostgreSQL implementor | `975e30287fa369a1ed31987e6510f5926c7a0b5dff9a94a8a66a19fc44a0d9a0` |
| `crates/ekr-kernel/tests/adversary_c7_m.rs` | PostgreSQL implementor | `1a219f92592de0da22896b9eccf386c021769275f27b4e9b8fed5d154545e85d` |
| `crates/ekr-kernel/tests/adversary_seed_envelope_v3.rs` | PostgreSQL implementor | `addf9ba2c10ffd8547480e0858309868b0efeb95432d3f88a2812819053b803a` |
| `crates/ekr-kernel/tests/adversary_perf02_m_head_graph.rs` | PostgreSQL implementor | `d58a3ad347632f63a84a3088f12811a93c5f33f86ac83f5c5c7bea0f82c33e03` |
| `.engineering/planning/story/workspace-crate-skeleton.md` | Coordinator, AEP-managed amendment | `6c6bdc1a8265199808d931a7fcc12400e4b1d0790057b50b99bf203bf0ea21d4` |

Current tracked-diff SHA-256 against `51d42cd1a8`: `4f5f67f130f4d3f3fd17f2f938552a61c5457b623cfe795dafac1c4be6c4a966`. This includes the final removal of `.clone()` from two expected-root assignments: `Root` derives `Copy`, so the test expectations and assertions are unchanged. It supersedes the earlier diff fingerprint only for identifying this later candidate. `checkpoint.rs` and `replay.rs` still have the exact hashes recorded in the direct-run review (`c72542fc3ce575a7bd4be739631fe4e27df2a1840028eb545537afdac8f6171f` and `af3f40deb7f1a567682697dbb8d0591253c23cc490d9bd2f030c980ca97ecd30`). No runtime implementation correction was identified in the assigned delta; CLI help prose now reflects the already implemented behavior.

Coverage assessment:

- CLI help now names migrated `/4`, captured-source semantics, logical versus physical history, the separate PostgreSQL schema receipt and credential-file boundary. `agent_cli.rs` retains exact inventory equality, adds the new schema command's own-help requirements and corrects the migration format requirement. It removes no command row or help obligation.
- `story_contract.rs` admits only the named new dependencies at the store boundary and adds the PostgreSQL provider to both the immutable pin checks and the CLI source writer-access scan. The scan's own tests now reject direct use, alias and expression forms of the PostgreSQL provider. Existing matchers and kernel-only authority checks remain intact; there is no exemption for a PostgreSQL writer in CLI source.
- The new `PostgresPool` case exercises the public type through real `Runtime::postgres`: default bounds must first open successfully, then zero connection capacity, zero connect timeout and zero shutdown timeout must fail. This positive control means a blanket open failure cannot make all its assertions pass. It is representative coverage of explicit bound removal, not exhaustive coverage of every pool field.
- `adversary_c7_m` replaces whole physical-root equality with explicit comparison of every one of `Root`'s seven fields under the allowed transformation: revision and four logical roots unchanged; seed transaction equals the new seed hash; later transaction hashes unchanged; every parent is the hash of the preceding migrated root. It also checks revision/event identities, times and source/destination record mappings. Existing retained payload bytes, object class history, graph evidence/assertions and full-replay reopen checks remain. No logical root check is dropped.
- The formerly ignored marker-shaped-evidence case is enabled with its body retained. It still requires an actual Committed result and successful full-replay reopen on both local providers. This increases executed coverage; it introduces no skip or weaker replacement assertion.
- `adversary_seed_envelope_v3` now requires a distinct migrated seed hash, a `/4` envelope and a parseable claim. Removing only that claim and changing only the format back to `/3` must reproduce the ordinary envelope exactly. Seed input, graph and normalized root equality remain explicit. A blanket inequality alone would not satisfy the new helper.
- `adversary_perf02_m_head_graph` compares every migrated revision to the source root after only the expected seed transaction/parent mapping, with event/revision identities, commit times and both record hashes checked. Pending transaction-state comparisons remain. Its historical-graph and bounded-live-graph tests are unchanged.
- The AEP amendment names the new store dependencies and writer-boundary checks without changing the historical story's implemented status. It explicitly leaves corrected gate evidence pending; it does not claim this review or a full gate as an approval.

Inspected existing output at `<cache>/ekr-hosted-runtime/postgres/focused-corrections2.log`, not rerun. Its 14 runner summaries total 119 passed, 0 failed, 0 ignored, 0 filtered out:

| Target | Passed |
|---|---:|
| CLI `adversary_c7_m` | 2 |
| `adversary_migrate_v3_cli` | 4 |
| `agent_cli` | 29 |
| `docs_cli` | 18 |
| `migrate_cli` | 3 |
| `postgres_cli` | 4 |
| `public_surface` | 12 |
| `story_contract` | 14 |
| kernel `adversary_c7_m` | 5 |
| `adversary_perf02_m_head_graph` | 4 |
| `adversary_seed_envelope_v3` | 7 |
| `adversary_seed_envelope_v3_elected` | 2 |
| `hosted_postgres` | 13 |
| `migrate_store` | 2 |

The coordinator reports that lane exited 0. The log explicitly includes `evidence_whose_payload_is_the_migration_marker_commits`, `postgres_pool_overrides_cannot_remove_connection_or_timeout_bounds`, and the previous-release compatibility case as passing. The earlier independent direct run remains 12 cases; this reviewer does not relabel it as 13. `<cache>/ekr-hosted-runtime/postgres/clippy-corrections2.log` now reaches `Finished dev profile` after checking the corrected packages. The broad incomplete run and this focused 119-case run remain separate evidence; neither is presented here as a full-gate pass. No new outside-worktree path was written in this addendum.

```findings
[]
```
