---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-03-view-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit V (ekr view)
relations:
- reviews: story:ekr-view-server
revision: 1
---
unit: story:ekr-view-server, worktree ekr-wave-b-v at head 374cd397 plus one untracked adversary test file
verdict: red
cases: executed 265→271, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/r2adv-alone.log, <scratch>/r2adv-flood.log, <scratch>/adv-suite-2.log, <scratch>/adversary-pass-2.md, <build-dir>/ (test artifacts), the harness's own background-task output file for the suite run
needs-coordinator: no

## 1. Diff

`git --no-pager diff --stat`: empty. `git status --short`: `?? crates/ekr/tests/adversary_v_view_r2.rs`, a test file and the only path touched. `cargo fmt -p ekr -- --check`: exit 0 (after `rustfmt` on that one file).

## 2. Cases added (crates/ekr/tests/adversary_v_view_r2.rs), each run alone before the suite

| case | asserts | now |
|---|---|---|
| `a_head_dribbled_a_byte_a_second_is_refused_within_its_5_s` | a head fed one byte a second is 400 within 7 s, as docs/cli.md and the module doc say ("not complete within 16 KiB or 5 seconds, is 400") | red |
| `dribbling_connections_up_to_the_descriptor_limit_do_not_starve_a_well_formed_client` | with `ulimit -n 48`, 60 dribbling connections do not stop a well-formed `GET /projection` getting 200 within 8 s | red |
| `the_answer_to_a_pipelined_request_survives_the_bytes_the_server_never_read` | three pipelined GETs, client reads after 500 ms: the first 200 arrives, no RST loss | green |
| `the_413_for_a_body_reaches_a_client_that_sent_the_body` | POST with a 2 KiB body sent, client reads after 500 ms: the refusal arrives | green |
| `a_commit_from_another_process_while_the_viewer_serves_lands_and_is_served` | both providers: `ekr commit` succeeds while `ekr view` serves, head projection moves to revision 1, `?revision=0` still shows revision 0 | green |
| `sigint_ends_the_viewer_and_leaves_the_file_store_byte_identical` | SIGINT ends the process within 5 s; every file name and byte of the file store is unchanged; a propose works afterwards | green |

Red runs, verbatim (`nice -n 19 cargo test -p ekr --locked --test adversary_v_view_r2 -- --exact <case>`):

```
thread 'a_head_dribbled_a_byte_a_second_is_refused_within_its_5_s' (2300139) panicked at crates/ekr/tests/adversary_v_view_r2.rs:191:5:
assertion `left == right` failed: after 12.4840467s of a head dribbled a byte a second the server had answered ""
  left: None
 right: Some(400)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 12.97s
```
```
thread 'dribbling_connections_up_to_the_descriptor_limit_do_not_starve_a_well_formed_client' (2336070) panicked at crates/ekr/tests/adversary_v_view_r2.rs:267:5:
assertion `left == right` failed: a well-formed GET /projection sent while 60 connections dribbled a head got no answer
  left: (Err(WouldBlock), None)
 right: (Ok(()), Some(200))
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 10.59s
```

Deviation, disclosed: the commit case's first version failed for a reason I did not claim. It compared the whole `?revision=0` projection with the pre-commit head projection, and `meta.head` differs (1 against 0) by design. I narrowed it to `meta.revision` and `nodes`. Run alone again it is green. The flood case was added after the first five had run alone. Its red run above is its first run.

## 3. Suite, after the cases existed

`nice -n 19 cargo test -p ekr --locked --no-fail-fast` (unit build dir, 2 jobs and 2 threads): `EXIT=101`. Summed `test result` lines: 269 passed, 2 failed, and the 2 failures are the red cases above. Log: `<scratch>/adv-suite-2.log`. `executed 265→271`: 265 is the implementor's reported gate (`<scratch>/r2-gate.log`, 265 passed, 0 failed), and it matches 271 minus the 6 cases added here. `story_contract` is green now, so the pass-1 needs-coordinator item is closed. After the run, `pgrep -x ekr` matched nothing, so no ekr process was left running.

## 4. Findings (tree: 374cd397 + adversary file)

| id | file:line | verdict | origin | severity | what was measured | what reaches it |
|---|---|---|---|---|---|---|
| F1 | crates/ekr/src/cli/view.rs:120 | confirmed | introduced | medium | `set_read_timeout(5 s)` bounds each `read`, not the head. A head fed one byte a second got no answer after 12.5 s. The doc at view.rs:32 and docs/cli.md promise 400 at 5 s. The bound is 16 KiB x 4.9 s, about 22 h per connection | any local process that can connect to 127.0.0.1:<port>. Browsers send whole heads, so a web page does not reach it |
| F2 | crates/ekr/src/cli/view.rs:101 | plausible | introduced | low | nothing caps connections or threads, so F1 connections pile up. With `ulimit -n 48`, 60 dribblers stopped `GET /projection` being answered within 8 s: accept hits EMFILE, and view.rs:112 sleeps and retries for as long as the flood keeps dribbling | the case built the low descriptor limit. Here the limit is 1,000,000 per process and 256,201 tasks per user, so the flood needs a local client holding very many sockets. No workflow found that lowers the limit |
| F3 | crates/ekr/src/cli/view.rs:300 | plausible | introduced | low | `own_host` requires `:<port>` literally. Browsers leave `:80` out of `Host`, so with `--port 80` every browser request would get 421. Not run (it needs a privileged port) | `--port 80` needs CAP_NET_BIND_SERVICE or a lowered `ip_unprivileged_port_start`. docs/cli.md shows 8080. Nothing found |

Fixes named, not applied:
- F1: keep a deadline from accept, set each read's timeout to what is left of it, and answer 400 once it passes.
- F2: cap concurrent connections with an atomic counter, and close the connection above the cap. The F1 deadline then limits how long a flood can hold the cap.
- F3: accept a missing port only when the port is 80, or say in the docs that port 80 is unsupported.

## 5. Attacked, not broken

- Pipelined requests: only the first is answered, then the server closes. The answer survives the unread bytes on loopback (case green).
- Body refusal: a 2 KiB body is refused and the refusal reaches the client. CL and TE are both refused as "announces a body". Duplicate `Content-Length: 0` is not treated as a body, and no other length is read. There is no proxy and no keep-alive, so smuggling has nothing to desync.
- obs-fold: httparse 1.10.1 rejects it in requests (400).
- Target bytes above 0x7f: httparse checks the path with `str::from_utf8` (lib.rs:919), so the unsafe path is not reachable.
- Absolute-form and percent-encoded targets: they never match a route (404), so they only fail closed.
- Host with uppercase, a trailing dot, IPv6 or a missing port: all 421, fail closed. The server binds only 127.0.0.1.
- Store-thread panic: `run` loops on the main thread. A panic there unwinds `main` and the process exits 101. The reply senders drop, connection threads see `recv` Err and return. So a panic kills the process and does not deadlock. No request input that panics `answer` was found.
- SIGINT: the process ends and the store is byte-identical, with no lock left behind. Concurrent commit on both providers: it lands and is served. Every request path calls only `project`, `snapshot` and `content`.
- Mutants were not executed. The dispatch brief says to edit the file and revert it; the charter forbids mutating a file under attack, and I followed the charter. A mutated scratch copy would rebuild every workspace crate into the unit's build dir and overwrite its `ekr` binary. Load average was 26 and `/` was 98% full (23G free). By inspection, the suite does not catch the removal of `set_write_timeout`.

## 6. Paths written outside the worktree

`<scratch>/r2adv-alone.log`, `<scratch>/r2adv-flood.log`, `<scratch>/adv-suite-2.log`, `<scratch>/adversary-pass-2.md`, `<build-dir>/` (the test binary and its dependencies for the new file), and the harness's own output file for the backgrounded suite run, under the shared TMPDIR.

## 7. Findings


## Coordinator routing (2026-09-27)

- F1 → back to the implementor: one deadline per connection, set at accept; every read waits only for the time left, so a head not complete 5 s after accept gets the 400 the docs promise.
- F2 → back to the implementor: at most 64 connections in flight; one over the cap is answered 503 at once and closed. With F1 no connection holds a slot past about 5 s, so a well-formed client is answered (200 or 503) within about 6 s. The adversary case is rewritten to assert that bound.
- F3 → back to the implementor: with `--port 80` a Host without a port is the server's own.
- Second pass: the correction is verified by the coordinator, no third attack.

```findings
[
{"file":"crates/ekr/src/cli/view.rs","line":120,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the 5 s head timeout is per read, so a dribbled head is never refused, contrary to docs/cli.md and view.rs:32"},
{"file":"crates/ekr/src/cli/view.rs","line":101,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"nothing caps connections or threads, so dribbling connections up to the descriptor limit starve every other client indefinitely"},
{"file":"crates/ekr/src/cli/view.rs","line":300,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"with --port 80 a browser's Host omits the port and every browser request would be 421"}
]
```
