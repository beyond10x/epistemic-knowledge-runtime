---
format: aep.planning-md/3
id: task:sdk-spawn-retries-a-busy-binary
kind: task
status: implemented
title: The SDK retries starting a binary that is busy being written
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T18:52:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T18:53:01Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-29T20:35:57Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Two CI runs on 2026-09-29 failed an `ekr-sdk` test with `ExecutableFileBusy` (ETXTBSY, os error
26) when the SDK started a binary the test process had just written:

- PR #51, run 36609731213: `a_replayed_recording_passes_with_no_ekr_binary` (a copied `ekr`);
  fixed in the test by linking instead of copying (`af4ea0c5`).
- main `d19820f1`, run 36613503758: `a_binary_that_never_answers_its_probe_is_killed_and_refused`
  (a stand-in script written with `fs::write`).

Linux refuses to execute a file while any process holds it open for writing. In a multithreaded
process, a thread that forks while another thread's write descriptor is open carries that
descriptor into the child until the child execs, so the fresh file is busy for that window. A
consumer that installs or writes its `ekr` binary and then starts a session from a program with
other threads meets the same refusal; it is not specific to tests.

## Build

- `EkrBinary` probes (`binary.rs`, the `Command::new(path)` spawn) and `ProcessSession::start`
  (`session.rs`) retry a spawn that fails with `ErrorKind::ExecutableFileBusy`, with a short
  bounded backoff (total under one second); any other spawn error is returned at once, as today.
- After the bound, the refusal is the existing `BinaryError::Run` / session start error carrying
  the ETXTBSY source, unchanged in shape.
- The two stand-in tests in `crates/ekr-sdk/tests/session.rs` and
  `crates/ekr-sdk/tests/adversary_session_transport.rs` keep writing their scripts; they now pass
  because the SDK retries.

## Acceptance

- A test holds a stand-in binary open for writing from another thread for about 100 ms, starts a
  probe, and the probe succeeds after the writer closes.
- A spawn error other than ETXTBSY (a missing file, a non-executable file) is returned on the
  first attempt, without delay.
- 20 consecutive runs of `cargo test -p ekr-sdk` pass.

## Surface

`crates/ekr-sdk/src/{binary,session}.rs`, `crates/ekr-sdk/tests/spawn_busy.rs` (new).
