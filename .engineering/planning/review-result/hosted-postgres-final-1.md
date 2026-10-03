---
format: aep.planning-md/3
id: review-result:hosted-postgres-final-1
kind: review-result
status: active
title: Hosted PostgreSQL final independent review and real-provider execution
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

```findings
[]
```
