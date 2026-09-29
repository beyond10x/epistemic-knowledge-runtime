---
format: aep.planning-md/3
id: story:sdk-session-transport
kind: story
status: draft
title: The SDK drives one child ekr session with typed replies and a binary handshake
relations:
- depends_on: story:ekr-session
- depends_on: story:session-starts-before-a-store
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/ekr-sdk/Cargo.toml
- confidence: inferred
  path: crates/ekr-sdk/src/binary.rs
- confidence: inferred
  path: crates/ekr-sdk/src/lib.rs
- confidence: inferred
  path: crates/ekr-sdk/src/reply.rs
- confidence: inferred
  path: crates/ekr-sdk/src/session.rs
- confidence: inferred
  path: crates/ekr-sdk/src/transport.rs
- confidence: inferred
  path: crates/ekr-sdk/src/viewer.rs
- confidence: inferred
  path: docs/sdk.md
revision: 11
---
## Context

A consumer instance re-implements the session client every consumer needs (408 lines): spawning,
environment hygiene, per-request timeouts, a failure latch that never restarts, stderr capture,
the one-shot fallback above the 25,231,360-byte line cap, a session probe and binary choice. The
session protocol is documented in `docs/cli.md` and tested, so none of this is consumer-specific.

## Build

- `trait Transport`: argv plus optional stdin in, a `Reply` (exit, document, stderr) out.
- `ProcessSession`: `ekr session --create` from an explicit binary path the consumer passes; the
  SDK never searches `PATH` for `ekr`. The child environment is cleared and holds only
  `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND` by default, or exactly the list the consumer passes.
  Per-request timeout; failure latch; stderr tail; a one-shot process for a request over the line
  cap.
- Cancellation: a consumer-side cancel (a handle the consumer triggers from its SIGTERM or SIGINT
  handler) stops the child promptly and sets the same failure latch.
- Typed outcomes: `Refusal { code, reason }` parsed from `ekr: <code>: …` on exit 2, `Fault` on
  exit 1, and the `Outcome` (Validated, Rejected, Stale) on exit 0.
- `EkrBinary`: `--version` against a minimum, and a capability probe through `ekr operations`.
- `Viewer::spawn(port)`, which reads the `{"url"}` line.
- `RecordingTransport`, which records a real session and replays the recording, so consumer tests
  run with no `ekr` binary.
- A blocking API with no async runtime dependency: no `tokio`, not behind a default feature and not
  behind an optional one.

## Surface (inferred)

`crates/ekr-sdk/{Cargo.toml,src/lib.rs,src/transport.rs,src/session.rs,src/reply.rs,src/binary.rs,src/viewer.rs}`,
root `Cargo.toml` members, `docs/sdk.md`.

## Acceptance

- Hash, seed and the first commit run through exactly one `ekr` process.
- A session killed mid-request fails that call, naming the verb and the stderr tail; every later
  call fails without a restart.
- A request over 25,231,360 bytes returns the same document as the one-shot verb.
- An `ekr` below the minimum version is refused, naming both versions.
- By default the child's environment holds only `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND`; with
  an explicit list it holds exactly that list.
- With an empty `PATH` and an explicit binary path the session starts; with no binary path the SDK
  refuses rather than searching.
- A cancel during a long request ends the child within 1 s, fails that call, and fails every later
  call without a restart.
- A test replaying a recorded session passes with no `ekr` binary installed.
- `cargo tree -p ekr-sdk --all-features` lists no `tokio`.
- `Viewer::spawn(0)` returns a URL whose `/head` answers.

## Consumer input (2026-09-29)

The reviewed consumer instance pins one `ekr` per run and picks which installed version reads an
older run, so the binary path is the consumer's choice. Its runtime has no async dependency. It
asks for replay so its tests stop needing three built `ekr` versions in CI.
