---
format: aep.planning-md/3
id: story:extraction-verb-shares-the-sdk-path
kind: story
status: draft
title: The engine's extraction verb runs on the SDK in-process
relations:
- depends_on: story:sdk-resolve-and-batch
- depends_on: story:sdk-evidence-attachment
- depends_on: story:extraction-document-applies-to-a-store
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr/src/cli/session.rs
revision: 3
---
## Context

`story:extraction-document-applies-to-a-store` would otherwise re-implement the SDK's resolve,
batch and evidence code inside the binary. One algorithm with two transports avoids that.

## Build

Crate `ekr` implements `ekr_sdk::Transport` over the session's argv dispatcher, with no pipes. The
extraction verb uses `Resolver`, `Batcher` and evidence attachment through it. This transport is
not exported.

The verb applies an extraction document the consumer already produced. It starts no agent and no
model: a consumer runs its extractor in its own sandbox and hands the engine the document
(consumer input, 2026-09-29). Applying that document through the SDK over a child session stays
supported beside the verb.

## Surface (inferred)

`crates/ekr/src/cli/session.rs`, a new `crates/ekr/src/cli/<extraction verb>.rs`.

## Acceptance

- One fixture applied through the verb and through the SDK over a child session gives equal
  stores: same names, types, counts and evidence bytes, ids ignored.
- The verb's write requests are only propose, validate and commit.
- The verb spawns no child process: a test with an empty `PATH` applies a fixture.
