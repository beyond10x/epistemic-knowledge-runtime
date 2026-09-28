---
format: aep.planning-md/3
id: review-result:adversary-ingest-01-s-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit S: ekr session'
relations:
- reviews: story:ekr-session
revision: 1
---
unit: story:ekr-session (wave ingest-01, unit S), commit b9a53b91 plus the untracked adversary test file, worktree ekr-ingest-s
verdict: CONFIRMED
cases: executed 376→379, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: /dev/shm/ekr-ingest-s/adversary-s1, plus the assigned build dir
needs-coordinator: none

## Cases added

`crates/ekr/tests/adversary_session_pass1.rs` (untracked; no implementation file touched):

| case | asserts | now |
|---|---|---|
| `adversary_session_sees_a_commit_another_process_made_while_it_was_open` | a one-shot propose, validate and commit made while a session is open shows up in the session's next `head` (rev 1), `resolve` (Resolved) and `transactions`, on file and SQLite | green |
| `adversary_session_commit_on_a_stale_validation_answers_as_the_one_shot_commit` | session validates A at rev 0, another process commits B, then the session commits A: exit, stderr and `kind` match the same sequence one-shot on a twin store | green |
| `adversary_session_one_request_line_does_not_cost_its_whole_length_in_memory` | a 128 MiB `propose -` request line, streamed in 1 MiB chunks: exit 2, the session keeps serving, and peak memory grows by less than the line length | red: "the session's peak resident set grew by 255 MiB to answer one 128 MiB request line; the one-shot verb reads at most 8 MiB of the same input" |

Suite: `cargo test -p ekr --no-fail-fast` exited 101: 378 passed, 1 failed, 1 ignored; the only failing target is `adversary_session_pass1`.

## Findings

| # | file:line | verdict / origin | severity | measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr/src/cli/session.rs:75 | CONFIRMED / introduced | warning | peak memory grows by about 2× the line length: `read_until` buffers the whole line, then `from_slice` copies `stdin` into a String; no line cap; one-shot `propose -` reads at most 8 MiB | the documented `"stdin"` field |
| F2 | .engineering/planning/story/ekr-session.md:50 | CONFIRMED / introduced | warning | the acceptance asks for 1,000 resolves on the epic's 112 MB store; the commit reports only 1.4, 19 and 21 MB stores | the story's acceptance |
| F3 | docs/cli.md:510 | CONFIRMED / introduced | note | `["help"]` and `["help","head"]` answer `session-verb-refused` (the table names only `--help`); `["-V"]` after a verb and `["--sto"]` give a clap usage error; exit 2 in every case | clap's built-in `help` subcommand |

## What held

- Escape attempts refused with `session-option-refused` or a clap usage error: `--store=x`, `--full-replay=true`, `--sto`, `["--","head"]`, verb-level global options; clap reads no environment variables.
- No served verb writes outside the store or opens a second store.
- Framing: newline, NUL and U+2028 inside `stdin` are escaped; one line per answer, flushed; malformed input answered once and the loop keeps serving.
- No panic on `u64::MAX` for `--at`/`--against` or `i64` extremes for `--valid-at`.
- A commit by another process is seen; a stale commit answers as the one-shot does.

```findings
- file: crates/ekr/src/cli/session.rs
  line: 75
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a session buffers a whole request line and then copies its stdin string, with no cap, so peak memory grows by about 2x the line length (255 MiB for 128 MiB), where one-shot propose reads at most 8 MiB
- file: .engineering/planning/story/ekr-session.md
  line: 50
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance measurement on the epic's 112 MB store is missing; the commit reports only 1.4, 19 and 21 MB stores
- file: docs/cli.md
  line: 510
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the clap `help` subcommand is answered session-verb-refused, which the refusal table lists only for --help, and an unknown leading flag gets a clap usage error instead of session-verb-unknown
```
