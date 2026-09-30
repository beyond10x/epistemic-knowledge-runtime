---
format: aep.planning-md/3
id: story:extraction-verb-shares-the-sdk-path
kind: story
status: proposed
title: The engine's extraction verb runs on the SDK in-process
relations:
- depends_on: story:sdk-resolve-and-batch
- depends_on: story:sdk-evidence-attachment
- depends_on: story:extraction-document-applies-to-a-store
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: cited
  path: Cargo.lock
- confidence: inferred
  path: crates/ekr-sdk/src/extraction.rs
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/extraction.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/extraction_cli.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures/conformance/manifest.json
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/provenance.json
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:26:22Z", actor: "human:timo", revision: 8}
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

## Scope

Derived 2026-09-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Decision (coordinator, 2026-09-30):** the shared apply routine lives in `ekr-sdk`
  (`crates/ekr-sdk/src/extraction.rs`) with the SDK's own mirror of the extraction document under
  `crates/ekr-sdk/src/document/`, as the SDK already mirrors the kernel's documents; the SDK may not
  link `ekr-graph` (`SDK_UNLINKED`, `crates/ekr/tests/story_contract.rs:887`) and `ekr-integrate`
  depends on it. The `ekr` verb runs the same routine over an in-process transport, and a drift test
  holds the SDK's document equal to `ekr schema`'s.
- **Primary surface:** `crates/ekr` CLI (`crates/ekr/src/cli`) and `crates/ekr-sdk` — cited (story Build)
- **Files (transport):** `crates/ekr/src/cli/session.rs` — `respond` :240 is the argv→`Answer` path the transport wraps (private, so the transport lives in this file or `crates/ekr/src/cli/session/`); `admit` :680 — cited
- **Files (verb):** `crates/ekr/src/cli/mod.rs` (`Command` :122, `Command::access` :422, `dispatch` :645) — cited; `crates/ekr/src/cli/extraction.rs` (new) — inferred
- **Files (dependency):** `crates/ekr/Cargo.toml` (no `ekr-sdk` dependency today), `Cargo.lock` — cited
- **Files (publication):** `crates/ekr/src/cli/agent.rs` (`GUIDE` ~:56), `docs/cli.md`, `crates/ekr/tests/agent_cli.rs` (~:241, ~:719) — cited (precedent `527bd4f88`)
- **Symbols:** `ekr_sdk::transport::Transport` (`transport.rs:51`), `Request`, `Reply` (`reply.rs:12`), session `Answer` (`session.rs:95`), `Resolver` (`resolve.rs:70`), `Batcher::commit_with_evidence` (`batch.rs:283`), `EvidenceSet` (`evidence.rs:60`) — cited
- **Tests:** `crates/ekr/tests/extraction_cli.rs` (verb equals SDK over a child session; writes limited to propose/validate/commit; empty `PATH`) — inferred name
- **Also likely (conformance):** `crates/ekr/src/conformance.rs`, `systems/ekr/domains/integrate.yaml`, `systems/ekr/conformance/{suite,baseline,provenance}.json`, `crates/ekr/tests/fixtures/conformance/manifest.json` — inferred
- **Confidence:** medium — the transport seam, the verb registration and the missing dependency are read from the tree; the verb's name is open
- **Would collide with:** any unit adding a CLI verb (`cli/mod.rs`, `session.rs` `admit`, `agent.rs`, `agent_cli.rs`, `docs/cli.md`); `ekr session` request handling; `crates/ekr/Cargo.toml`/`Cargo.lock`; the kernel suite; `integrate.yaml`; SDK `Transport`/`Reply`/`Resolver`/`Batcher` signatures
- **Not established:** `ekr` → `ekr-sdk` makes a cycle through `ekr-sdk`'s dev-dependency on `ekr`; Cargo accepts it, but in-crate `#[cfg(test)]` modules may see two `ekr-sdk` copies — unproven
