---
format: aep.planning-md/3
id: task:protocol-error-keeps-the-answer
kind: task
status: active
title: A protocol error keeps the start of the answer it could not read
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T18:15:38Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:39Z", actor: "human:timo", revision: 3}
---
## What is wrong

A consumer asked on 2026-09-30. When `ekr session` or a one-shot verb answers with a line that is not
JSON, `TransportError::Protocol` carries the verb, the parse error and the stderr tail, but not the
answer itself. The consumer's error then reads "…something that is not a reply (expected ident at
line 1 column 2); stderr tail: (empty)", and nothing says what the process printed. The consumer's
own session code keeps that line and a test of its own pins it, so its move onto `ProcessSession` is
parked until the SDK does the same.

Constructed at `crates/ekr-sdk/src/session.rs:517` and `:619` and at
`crates/ekr-sdk/src/read/one_shot.rs:94`; declared at `crates/ekr-sdk/src/transport.rs:111-120`.

## Build

`TransportError::Protocol` gains an `answer` field: the start of what the process printed, at most
400 bytes, cut on a UTF-8 boundary, with a marker when it was cut. The `Display` text names it. All
three construction sites fill it.

## Acceptance

- A session whose child prints `this is not a JSON answer` returns a `Protocol` error whose `answer`
  is that line and whose message contains it; the same holds for a one-shot read.
- An answer longer than 400 bytes is cut to at most 400 bytes on a character boundary and says it was
  cut.
- `docs/sdk.md` (or the crate docs) names the field; the SDK's existing transport tests pass.
