unit: C (`task:postgres-source-copy`), wave 2026-10-07c, commit 3c96c31e52 on unit/postgres-source-copy, tree ekr-w20261007c-c plus 2 untracked adversary test files
verdict: CONFIRMED (no blocking finding; 3 red cases: 2 contract drift, 1 test that cannot fail)
cases: executed 23→30, red 3
origin: introduced 9 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, ~/.cache/ekr-w20261007c-adversary-c/ (13 logs)
needs-coordinator: none

## 1. Diff

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short`:

```
?? crates/ekr-kernel/tests/adversary_w20261007c_c_capture.rs
?? crates/ekr-store/tests/adversary_w20261007c_c_docs.rs
```

Both are test files. This file is under `.engineering/drafts/` (ignored by `.git/info/exclude`).

## 2. Cases added (each run alone before the suite)

| file :: case | asserts | now |
|---|---|---|
| `crates/ekr-store/tests/adversary_w20261007c_c_docs.rs` :: `docs_cli_names_no_inventory_refusal_the_runtime_no_longer_emits` | no line of `docs/cli.md` names `postgres-inventory-requires-capture` while no runtime source emits it | red |
| same :: `the_begin_command_does_not_say_the_inventory_refuses_a_capture_today` | `systems/ekr/domains/cli.yaml` does not describe the removed refusal as current | red |
| `crates/ekr-kernel/tests/adversary_w20261007c_c_capture.rs` :: `the_one_capture_predicate_sees_a_feed_read_after_the_capture` | the unit's predicate `stream_reads() == StreamReads { captures: 1, .. }` is false after one capture plus a full feed read (`published_events()`) of the source | red |
| same :: `a_postgres_copy_reads_nothing_of_the_source_after_its_capture` | with a call-recording wrapper on the source: `capture()` is exactly one `inventory` call; a commit and a Proposed-only transaction written after the capture; `copy_into` makes zero calls into the source; the stage's kept roots equal the captured head's and its transaction set equals the source's at capture | green |
| same :: `a_sqlite_copy_reads_nothing_of_the_source_after_its_capture` | the same from a live (writing, not image) SQLite source | green |
| same :: `an_interrupted_stage_copy_cannot_be_captured_on_sqlite` / `_on_postgres` | after a copy interrupted after the seed and before the completion receipt, `capture()` of the stage tenant from a fresh writing handle refuses `migrate-incomplete`, checkpoint path and full replay | green |

Red output, docs file alone (`cargo test --locked -p ekr-store --test adversary_w20261007c_c_docs`):

```
---- the_begin_command_does_not_say_the_inventory_refuses_a_capture_today stdout ----
thread 'the_begin_command_does_not_say_the_inventory_refuses_a_capture_today' (85956) panicked at crates/ekr-store/tests/adversary_w20261007c_c_docs.rs:80:5:
systems/ekr/domains/cli.yaml describes the removed inventory refusal as current: ["  # capture, which the inventory refuses today (`postgres-inventory-requires-capture`, held by"]
---- docs_cli_names_no_inventory_refusal_the_runtime_no_longer_emits stdout ----
thread 'docs_cli_names_no_inventory_refusal_the_runtime_no_longer_emits' (85955) panicked at crates/ekr-store/tests/adversary_w20261007c_c_docs.rs:64:5:
docs/cli.md names a refusal no runtime source emits: ["`postgres-inventory-requires-capture`; its change feed is not a complete source snapshot."]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

Red output, predicate case alone (`EKR_REQUIRE_POSTGRES=1 … --exact the_one_capture_predicate_sees_a_feed_read_after_the_capture --test-threads=1`):

```
thread 'the_one_capture_predicate_sees_a_feed_read_after_the_capture' (97756) panicked at crates/ekr-kernel/tests/adversary_w20261007c_c_capture.rs:388:5:
assertion `left != right` failed: a complete feed read of the source after its capture leaves the unit's one-capture-and-no-feed-read predicate true
  left: StreamReads { revision: 0, checkpoint: 0, object: 0, feed: 0, captures: 1 }
 right: StreamReads { revision: 0, checkpoint: 0, object: 0, feed: 0, captures: 1 }
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.71s
```

(Line 388 at that run; 410 after a later edit to the green cases' head comparison, which first compared the whole `Root` and failed on `parent`, an address design § 107.1 says is the stage's own. That was my assertion's error, not a finding.)

## 3. Suite, after the cases existed

`EKR_REQUIRE_POSTGRES=1 EKR_TEST_POSTGRES_CONFIG=…/unit-c/app.json EKR_TEST_POSTGRES_OWNER=…/unit-c/owner.json CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=false cargo test --locked --no-fail-fast -p ekr-kernel --test stage --test hosted_postgres --test migrate_store --test adversary_w20261007c_c_capture -- --test-threads=1`, exit 101:

```
adversary_w20261007c_c_capture: test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.15s
hosted_postgres:                test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 29.66s
migrate_store:                  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.69s
stage:                          test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
```

`… cargo test --locked --no-fail-fast -p ekr-store --test store_inventory --test adversary_w20261007c_c_docs -- --test-threads=1`, exit 101:

```
adversary_w20261007c_c_docs: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
store_inventory:             test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.49s
```

executed 23 (unit's targets, my two files deselected) → 30 with them; red 3.

## 4. Findings

| # | severity | location | verdict / origin | failure | smallest fix | what reaches it |
|---|---|---|---|---|---|---|
| 1 | fix-in-unit-P-or-L | `docs/cli.md:3385-3386` | CONFIRMED / introduced | the user reference still says "Low-level PostgreSQL inventory also refuses `postgres-inventory-requires-capture`"; the unit removed that refusal and `docs_cli.rs` checks only the refusal table | delete that sentence (or say the inventory is one provider capture) | any reader of the hosted-copies section |
| 2 | note | `systems/ekr/domains/cli.yaml:38-40` | CONFIRMED / introduced | `ekr.cli.BeginStage`'s comment says the inventory "refuses today" the capture unit C adds | reword with unit S's specification fixes | readers of the specification |
| 3 | note | `crates/ekr-kernel/tests/hosted_postgres.rs:127-135`, `:873-880` | CONFIRMED / introduced | "one provider capture and no stream or feed read" is asserted with `StreamReads`, which counts neither `EventlogStore::published_events` (`read_feed`) nor `get_blob`; an inventory that also read the feed or a blob after its capture keeps both cases green (red case 3 runs that mutant) | count each `published_events` page as `feed` and add a blob-read counter, or hold the source behind a call-recording wrapper as `adversary_w20261007c_c_capture.rs` does | the two unit cases; implementation read correct |
| 4 | note | `crates/ekr-kernel/tests/hosted_postgres.rs:923-967` | CONFIRMED / introduced | `a_commit_racing_the_capture_is_not_in_the_stage` writes its racing commit inside the destination's `initialize`, after any read a copy could make at its start, and compares revisions only; a `copy_unchecked` that captured the source again first, or carried a later Proposed transaction, passes it. Not run against a mutated copy: shown by construction | adopt `a_postgres_copy_reads_nothing_of_the_source_after_its_capture` / `a_sqlite_…` (write between `capture()` and `copy_into`, wrapper counts, transaction set compared) | the unit's acceptance case |
| 5 | fix-in-unit-P-or-L | `crates/ekr-store/src/eventlog.rs:584-600`; eventlog-postgres `capture.rs:80-95`; `crates/ekr-store/src/postgres.rs:72-78` | INFEASIBLE / introduced | hypothesis, unmeasured: the inventory capture has no caps (`u64::MAX`) and holds the schema-wide exclusive publication advisory lock for the whole-tenant read; defaults `lock_timeout_ms` 2000 and `transaction_timeout_ms` 10000. A begin on a large PostgreSQL store would refuse on the deadline, and while it runs every publisher of every tenant in the schema waits and fails after 2 s | unit P measures capture time against store size and states the bound (ADR 0013 "Cost of begin" is unmeasured) | begin, once unit P calls `capture()` |
| 6 | fix-in-unit-P-or-L | `crates/ekr/src/cli/mod.rs:1232-1249`, `:1351-1353`; `docs/cli.md` § Common refusals | CONFIRMED / introduced | by reading: a host tenant carrying `ekr.stage:` reaches the CLI as `opening the provider: stage-tenant-reserved: …`, exit 1, filed as a provider fault; the page has no row for it. Not run (no `ekr` binary built in the tree) | unit L maps it to a named host-configuration refusal and adds the row | any `ekr` verb with such a tenant |
| 7 | note | `crates/ekr-kernel/src/migrate.rs:184-193`, `:204-213` | CONFIRMED / introduced | `CapturedStore` says it is "taken in one read of the provider", but `Commit::capture()` accepts any `Inventory` store; on a live SQLite or File handle `inventory()` is several reads (feed, revision stream, each object stream, blobs) | unit P's begin opens the SQLite source as an image (`sqlite_read_only`), or the type admits only image/capture sources | nothing found today: tests and `ekr migrate` use images |
| 8 | note | `crates/ekr-kernel/src/migrate.rs:207`; `crates/ekr-store/src/eventlog.rs:597-599` | CONFIRMED / introduced | "Writes nothing": on a PostgreSQL writing handle the capture first calls `stream_identity`, which inserts the tenant's capture identity row when absent (non-canonical metadata) | say so in the doc | every PostgreSQL capture from a writing handle |
| 9 | note | `docs/epistemic-knowledge-runtime-design.md` § 107.10, § 107.1 | CONFIRMED / introduced | § 107.10 places `a_stage_tenant_is_never_a_store_tenant` in `hosted_postgres.rs` (it is in `stage.rs`); § 107.1 "A copy into a second tenant of the same store is unexecuted" is now executed | record both in the unit's evidence or the next amendment | readers of the design |

## 5. Attacked, not broken

- Capture consistency: the PostgreSQL inventory is one repeatable-read capture after the publication gate; `captured_inventory` reads only the capture (events, blobs); `copy_into` makes zero calls into the source (wrapper, PostgreSQL and live SQLite); a commit and a Proposed transaction after the capture are not in the stage.
- Interrupted copy laundering: a stage interrupted after its seed or before its receipt refuses `migrate-incomplete` to `capture()` on a fresh handle, checkpoint and full paths, both providers.
- Marker evasion (case, prefix, embedded, Unicode, spaces): eventlog-sqlite and eventlog-postgres compare `tenant_id TEXT` byte for byte (no NOCASE or collation); `TenantId` admits only ASCII graphic and space; derived names carry the exact lowercase marker, so no variant equals one. All 8 `Runtime` openers call `admit_store_tenant` first; no product source outside `ekr-store` and `ekr-kernel` opens a store directly.
- Collision: name = marker + 64-hex store-tenant digest + 36-character UUID, fixed width, so two (store, stage) pairs differ unless the digest collides.
- SQLite migration regression: `migrate_store` 2/2, `store_inventory` 2/2, `hosted_postgres` 18/18, `stage` 1/1.

## 6. Paths written outside the worktree

`<home>/.cache/ekr-w20261007c-adversary-c/` holding `docs-red.log`, `capture-build.log`, `predicate-red.log`, `a_postgres_copy_reads_nothing_of_the_source_after_its_capture{,.2}.log`, `a_sqlite_copy_reads_nothing_of_the_source_after_its_capture{,.2}.log`, `an_interrupted_stage_copy_cannot_be_captured_on_{sqlite,postgres}.log`, `suite.log`, `suite-store.log`. Build output went to the tree's own `target/`. PostgreSQL tenants `adversary-c-<uuid>` and their stage tenants were written in schema `ekr_unit_c`.

## 7. Findings block

```findings
- file: docs/cli.md
  line: 3386
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the user reference says the low-level PostgreSQL inventory refuses postgres-inventory-requires-capture, which no runtime source emits after this unit
- file: systems/ekr/domains/cli.yaml
  line: 39
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: BeginStage's comment says the inventory refuses the PostgreSQL capture today
- file: crates/ekr-kernel/tests/hosted_postgres.rs
  line: 131
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: StreamReads counts neither published_events feed reads nor blob reads, so the one-capture-and-no-feed-read predicate stays true after a second read of the source
- file: crates/ekr-kernel/tests/hosted_postgres.rs
  line: 960
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the racing commit lands after any re-read a copy could make at its start and only revisions are compared, so a copy that re-captured the source or carried a later proposal passes
- file: crates/ekr-store/src/eventlog.rs
  line: 584
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: unmeasured hypothesis that an uncapped whole-tenant capture under the schema-wide publication lock exceeds the 10 s deadline on large stores and fails other tenants' publishers after the 2 s lock timeout
- file: crates/ekr/src/cli/mod.rs
  line: 1247
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: stage-tenant-reserved reaches the CLI as an unnamed provider fault and docs/cli.md has no refusal row for it
- file: crates/ekr-kernel/src/migrate.rs
  line: 213
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: capture accepts any Inventory store, and on a live SQLite or File handle the inventory is several reads, not the one read CapturedStore claims
- file: crates/ekr-kernel/src/migrate.rs
  line: 207
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: capture is documented as writing nothing but inserts the PostgreSQL capture identity row from a writing handle
- file: docs/epistemic-knowledge-runtime-design.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: section 107.10 places a_stage_tenant_is_never_a_store_tenant in hosted_postgres.rs and section 107.1 calls the same-store copy unexecuted
```

## Outcome

The three red cases pass after `PostgresStore::published_events()` counts each feed page it reads and `docs/cli.md` drops the removed inventory refusal; the `cli.yaml` wording was already corrected by unit S's adversary fixes. The stage-tenant-reserved CLI row and the capture-size measurement go to units L and P.
