unit: story:synchronous-runtime-refusal, working tree over d52142046b967e83577474e6348a96c1e9f34195
verdict: nothing found
cases: executed 66→71, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 assigned roots/groups; exact private path inventory retained separately
needs-coordinator: record review and run the integrated gate; no implementation correction identified

`git --no-pager diff --stat` at handback:

```text
 crates/ekr-store/src/eventlog.rs | 65 ++++++++++++++++++++++++++++++++--------
 crates/ekr-store/src/lib.rs      |  3 ++
 2 files changed, 56 insertions(+), 12 deletions(-)
```

Those are the **inherited coordinator implementation changes**, present at review start and byte-unchanged. The inherited untracked `tests/runtime_context.rs` is also unchanged. Git's unstaged stat omits untracked files; the authored test-only diff, measured with `git diff --no-index --stat /dev/null <test>`, is:

```text
 .../ekr-store/tests/adversary_runtime_context.rs | 313 +++++++++++++++++++++
 1 file changed, 313 insertions(+)
```

The only repository write by this reviewer is `crates/ekr-store/tests/adversary_runtime_context.rs`. `authored-test.patch` preserves it. No source, existing case, manifest, specification, planning record or commit was edited. The report covers the uncommitted implementation handed back on the base above, not a later integration commit.

## Cases and first executions

All five cases were written before any test execution in this review. Each then ran alone with its exact full name before the package suite. They were green on their first execution; no red behavior was found or claimed.

| Case, in adversary_runtime_context.rs | Boundary measured | First result |
| --- | --- | --- |
| entered_handle_constructor_refusal_preserves_existing_provider_bytes_and_missing_parents, line57 | Real prepopulated File/SQLite bytes and missing nested parents remain unchanged; an entered cloned Handle on another thread refuses before even invalid-tenant/provider work. Both stores reopen afterward. | 1 passed, exit0 |
| populated_file_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes, line202 | Existing seed plus committed revision means normal fold actually calls both supplied authority functions. Every public I/O path, via trait objects and inherent archival method, refuses in idle entered and running current-thread contexts without callbacks or any persisted-byte change. Retention and append identity remain available afterward. | 1 passed, exit0 |
| populated_sqlite_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes, line211 | Same populated-lineage checks against SQLite, including missing object, maximum revision, malformed/repeated initialization and normal append/retry after refusal. | 1 passed, exit0 |
| store_drop_during_caller_unwind_in_entered_handle_keeps_completed_data_reopenable, line238 | After a named refusal the caller deliberately panics; store destruction in the entered context causes no second panic, and both providers reopen with completed bytes intact. | 1 passed, exit0 |
| plain_worker_thread_can_open_and_write_while_another_thread_runs_tokio, line274 | Another thread's active runtime does not globally disable synchronous access: a genuinely unentered worker constructs, writes, reads and drops both providers normally. | 1 passed, exit0 |

The exact first-run command template was:

```console
cargo test -p ekr-store --offline --test adversary_runtime_context <full-case-name> -- --exact --nocapture
```

Each log reports verbatim:

```text
running 1 test
test <full-case-name> ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out;
```

The complete output, including runner durations and compilation paths, is in the five `first-*.log` files. The unwind case deliberately prints its two caught `synthetic caller unwind after named refusal` panic messages before its passing result. Those are expected test stimuli, not reported implementation failures. These provider-mechanics fixtures deliberately use substitute authority to count callbacks; they do not claim kernel seed-validation coverage.

## Suite and source verification

Baseline66 is the coordinator's handback count for the same command, retained in adjacent `store-test.log`/`store-test.status`; this reviewer did not run a pre-case baseline suite.

```console
cargo test -p ekr-store --offline
```

Exit0; package total **71 passed, 0 failed, 0 ignored**. The added binary's own summary is:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
```

Existing binaries retain counts1,2,2,1,7,18,9,1,15,2,6,2; the library and doc-test runners remain0. Together with the new5 these sum to71. Full runner output is retained in `store-suite.log`; no selected-zero result stands in for a new case.

```console
cargo clippy -p ekr-store --all-targets --offline -- -D warnings
cargo fmt --all --check
git diff --check
sha256sum -c <coordinator-source-manifest>
```

Clippy exit0. Initial formatter check found only this reviewer's `Arc` import ordering after direct rustfmt used a newer style edition; `fmt-first.log` preserves that result. Only the new test's import ordering was corrected, and final formatter check exits0. That formatting-only adjustment followed the behavioral suite/Clippy run. Diff check exits0. All three inherited source/test hashes still match the coordinator manifest:

```text
48bc6ce5371d8331d5fbd0cb70577c26261d8a76dfa4fd77afd5353f68101d54  crates/ekr-store/src/eventlog.rs
36173477d1d0cea8921904ca65a5b3ed35b0ca4368bd9ec6fa20c2880e726afb  crates/ekr-store/src/lib.rs
022b0f092c5357546a8dd60555d7c608f67d313318e88bbd486c9c5e376377c5  crates/ekr-store/tests/runtime_context.rs
```

Final authored test SHA-256:

```text
577368819d1fd036de2267d26eeaca8994f9aa0961f6ce81937742d68271f9da
```

## Findings and bounds

No implementation finding was measured in the assigned scope.

- Both constructors refused before source-path changes, including existing provider bytes, missing parents and invalid inputs.
- The nine public persistence entry points are constructors separately plus `store_graph`, `seed_bytes`, `append`, `fold`, `head`, `replay`, `initialize`, `put` and `get`; all nine I/O methods were exercised against populated stores. `under` only installs authority and is not an I/O path.
- Real populated-history authority callbacks were demonstrated to run synchronously outside Tokio, then remained untouched during refused calls; refusal left retention and append retry identity intact.
- Existing coordinator cases cover destruction inside a running current-thread executor; added cases cover caller unwind under an entered handle and later reopening.
- The current public CLI is synchronous. Reachability assessed here is the expressly supported public store API moved/called through normal Rust ownership from an entered Tokio thread, not a guessed existing async product caller.

No multithread-runtime feature, future durable writer, general async API, kernel authority semantics, production provider mutation or full workspace gate was added to this bounded review. No source mutation was performed. The coordinator's integrated gate remains required.

Procedural deviation: the first batched read ran one read-only `aep plan artifact show story:synchronous-runtime-refusal` while loading the adversary charter. This conflicts with that charter's no-AEP-command rule. No planning mutation occurred; subsequent story inspection used the file directly. This administrative error is disclosed separately from the empty implementation findings, and was reported to the coordinator before handback.

## Outside writes and handback

`outside-paths-private.txt` enumerates exact absolute scratch paths, the assigned Cargo target and TMPDIR, and the managed lease metadata root. It is private operational inventory, not public report material. Public-safe roots here are:

1. Assigned `runtime-context/adversary/`: five isolated logs, suite/Clippy/formatter logs, patch/stat/hash records, report/privacy/lease records and private path inventory.
2. Assigned isolated `b10x-target/ekr-p1-runtime-context-20260922`: Cargo compiler/test outputs.
3. Assigned completion `tmp/`: synthetic temporary File/SQLite fixtures created and normally removed by TempDir.
4. Worktree CLI session metadata for `codex-ekr-runtime-context-adversary`: acquired, heartbeated and released only this reviewer's lease.

No process started by this reviewer remains running at handback. Source/tree cleanup remains coordinator-owned. The tree and new test are retained; no commit or publication occurred. Owners: implementation and inherited cases — coordinator; new test and this report — adversary; final integration and publication — coordinator.

```findings
[]
```
