---
format: aep.planning-md/3
id: story:sdk-source-adapter-runner
kind: story
status: draft
title: The SDK runs a consumer's source adapter against a store
relations:
- depends_on: story:source-adapter-contract
- depends_on: story:observations-are-retained
- depends_on: story:sdk-session-transport
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr-sdk/examples/fixture_adapter.rs
- confidence: inferred
  path: crates/ekr-sdk/src/observe.rs
revision: 3
---
## Context

ADR 0012 keeps connectors in consumers; the adapter trait lives in ESS `ekr.observe` and in Rust
(`story:source-adapter-contract`), which leaves open "a library crate API, a CLI verb reading an
adapter's output, or both". The SDK is the library half.

## Build

`run_adapter(&mut impl SourceAdapter, &Session)`: poll from the checkpoint, send observation
candidates to the engine's recording verb over the session, store the checkpoint, read coverage.
The trait comes from `ekr-observe`, which builds as a dependency outside this workspace.

## Surface (inferred)

`crates/ekr-sdk/src/observe.rs`, `crates/ekr-sdk/examples/fixture_adapter.rs`.

## Acceptance

- A fixture adapter polled twice through the SDK records zero new observations the second time.
- An example crate depending only on `ekr-sdk` and `ekr-observe` compiles and polls.
- Coverage lists every declared unit with its checkpoint and last poll result.
