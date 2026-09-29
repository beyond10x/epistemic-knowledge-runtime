---
format: aep.planning-md/3
id: task:sdk-close-reports-the-child-stderr
kind: task
status: implemented
title: ProcessSession::close reports the child's stderr when it exits non-zero
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T20:49:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T20:49:28Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-29T23:07:51Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer reported on 2026-09-29, while adopting the SDK: at 0.0.20 `ProcessSession::close()`
returns only the child's `ExitStatus` (`crates/ekr-sdk/src/session.rs:319`). The child's stderr is
collected by the session (`StderrTail`, `session.rs:140`) but never exposed on close, so when
`ekr session` exits non-zero at the end of its input (for example when it cannot write its
replay checkpoint) the consumer cannot say why. The consumer's own transport reports that stderr
and a test of theirs requires it; they keep that transport until the SDK does the same.

## Build

- `close()` returns, besides the exit status, the tail of the child's stderr (the same
  `STDERR_TAIL` bound the other failures use) when the child exits non-zero or is killed at the
  deadline. A zero exit carries no tail requirement.
- The shape is decided by the implementor and stated in `docs/sdk.md`: either a `Closed { status,
  stderr_tail }` value or an error variant for a non-zero exit carrying both. It is a breaking
  change of `close()`'s signature; the CHANGELOG says so.

## Acceptance

- A stand-in `ekr` that answers the handshake, then writes a line to stderr and exits 3 at end of
  input: `close()` reports exit 3 and the line.
- A real `ekr session` that exits 0: `close()` reports success, as today.
- A child killed at the close deadline reports that it was killed, with its stderr tail.
