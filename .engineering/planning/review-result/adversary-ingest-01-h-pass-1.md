---
format: aep.planning-md/3
id: review-result:adversary-ingest-01-h-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit H: blobs verified once per process'
relations:
- reviews: story:history-loaded-once-per-process
revision: 1
---
unit: story:history-loaded-once-per-process (ingest-01 unit H), commit 909a449a plus one untracked test file, worktree ekr-ingest-h
verdict: NEEDS-CHANGE
cases: executed 109→114, red 5
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (the assigned build dir)
needs-coordinator: none

## Cases added

`crates/ekr-store/tests/adversary_history_cache.rs` (untracked; no implementation file touched). All five red.

| case | asserts | red output |
|---|---|---|
| `adversary_history_second_handle_second_read_hashes_nothing_{sqlite,file}` | two handles on one store; drop the first, then a second `history()` on the second hashes nothing | `left: ReadWork { blobs_read: 0, blobs_hashed: 4, occurrences_read: 0 } right: ReadWork { … blobs_hashed: 0 … }` |
| `adversary_history_get_then_foreign_raise_still_reads_{sqlite,file}` | handle R calls `get(X)` while X is Provenance; another handle raises X to Canonical; R's `history()` needing X at Canonical succeeds as a new handle's does | `refused a history a fresh handle accepts: Err(Document("required-object-integrity"))` |
| `adversary_history_replaced_file_store_selected_read_refuses_like_head_read` | after the file store is replaced by a different history of the same length, `history_at(SEED)` refuses as `history()` does | head read: `Err(Backend("event store is unavailable: file history diverged from this handle's observed history"))`; selected read: `Ok(1)` |

Suite: `cargo test -p ekr-store --locked --no-fail-fast` exited 101: 109 passed, 5 failed, 1 ignored; all failures in `adversary_history_cache`.

## Findings

| # | file:line | verdict / origin | what breaks | what reaches it |
|---|---|---|---|---|
| F1 | `crates/ekr-store/src/eventlog.rs:604` | NEEDS-CHANGE / introduced, warning | a read at an earlier revision is answered from the handle's memory with no provider call, so the provider's "history diverged" refusal no longer fires for it | `history_at`, `replay(rev)`, `Runtime::read(Some)`, on a store replaced while a handle is open |
| F2 | `crates/ekr-store/src/eventlog.rs:1307` | INFEASIBLE / introduced, note | `get` fills the memo with the class read then; a raise by another handle is never seen and `history()` falsely refuses `required-object-integrity` | the public `ObjectStore`/`RevisionLog` API only |
| F3 | `crates/ekr-store/src/verified.rs:61` | INFEASIBLE / introduced, note | a second handle's copy of a registered blob is never registered, so after the first handle drops every later `content()` hashes again | library use with two handles |

## What held

- A hand-built `RetainedHistory` under a different hash key still goes through the address check; a dead registry entry falls back to hashing.
- Two stores in one process, one with a damaged blob: the mismatch is hashed and refused.
- Head reads after a file store is replaced still refuse; appends from another handle and a damaged new blob are handled.
- Selected reads at the seed match a new handle's result.
- `EventlogStore` is not `Sync`, so one handle is never used from two threads.

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 604
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a selected-revision read served wholly from the held prefix makes no provider call, so a handle whose head read is refused for a diverged file history still returns the old history from history_at
- file: crates/ekr-store/src/eventlog.rs
  line: 1307
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: get() now fills the per-handle memo with the class read then, so a retention raise by another handle is not seen and a later history() falsely refuses required-object-integrity
- file: crates/ekr-store/src/verified.rs
  line: 61
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a handle that loads a blob already registered by another live handle keeps an unregistered copy, and after that handle drops every content() on its histories hashes again
```
