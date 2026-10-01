---
format: aep.planning-md/3
id: story:explain-reads-an-index
kind: story
status: active
title: explain looks an assertion up and answers by reference
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/read.rs
- confidence: inferred
  path: crates/ekr-kernel/src/replay.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/explain.rs
- confidence: cited
  path: crates/ekr-sdk/src/read/kernel.rs
- confidence: cited
  path: crates/ekr-sdk/src/read/mod.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/read.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/explain.rs
- confidence: cited
  path: crates/ekr/src/cli/mcp.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: docs/guide.md
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:26:23Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:40Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
---
## Context

Performance audit 2026-09-29: `explain` parses every committed document per call and embeds whole
documents in its answer (`crates/ekr-kernel/src/explain.rs:381`): 4.6 / 10.7 / 31.7 s at 1× / 3× /
10×, answer 3.0 / 7.7 MB (MCP 6.0 / 15.4 MB). An agent calling the MCP `explain` tool pays that per
assertion.

## Build

- Replay records, per assertion, the transaction that committed it (and the ones that retracted or
  superseded it), so `explain` looks it up instead of parsing every document.
- The answer references proposals, receipts and evidence by hash and carries only the operations
  about the assertion; a flag or a separate read returns a whole document when asked. This changes
  the `explain` answer's format: specify it first (`kernel.yaml` `Explain`), version it, and keep the
  § 62 chain complete.

## Acceptance

- At the 1× shape, `explain` of an assertion answers in under 200 KB through the session and the
  one-shot verb, and under 400 KB through MCP (the audit measured MCP at twice the session size,
  6.0 against 3.0 MB); explain's own work on an open store, measured apart from opening it, takes
  under 0.5 s.
- Revised by the coordinator on 2026-10-01 (`review-result:adversary-extract-06-x-pass-1`): the
  per-call bound of 0.5 s was measured at 0.44–0.65 s through the session and MCP and about 4 s
  one-shot, nearly all of it opening the store (blob-integrity SHA-256). That cost is not
  explain's and moved to `task:store-open-verifies-blobs-once`.
- The § 62 chain (evidence, proposal, validation, commit, supersession) is the same for every
  assertion of the conformance fixtures as before, by id.
- MCP `explain` and the session verb answer the new format; the SDK's `Explanation` reads it.

## Scope

Derived 2026-09-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-kernel` explain projection — cited (`crates/ekr-kernel/src/explain.rs:381`)
- **Files:** `crates/ekr-kernel/src/explain.rs:194` (`VerifiedRead::explain`), `:367` (`committed()`, per-call parse at `:381`), `:89`/`:108` (`ExplanationLink`, `ExplanationResult`) — cited
- **Files:** `systems/ekr/domains/kernel.yaml:873-928`, `:1474` (`ekr.kernel.Explain`), `:1585` (`ekr.kernel.Explained`) — cited
- **Files:** `crates/ekr-kernel/src/replay.rs:73` (`ReplayState`), `:1060-1196` (the `RevisionCommitted` arm where the index would be recorded), `:868` — inferred
- **Files:** `crates/ekr-kernel/src/read.rs:34`, `:240`; `crates/ekr-kernel/src/checkpoint.rs:640` — inferred
- **Files:** `crates/ekr/src/cli/explain.rs`, `crates/ekr/src/cli/mod.rs:704` (`Command::Explain`), `:1038` — cited; the whole-document flag's placement inferred
- **Files:** `crates/ekr/src/cli/mcp.rs:553`, `:739`, `:930` — cited
- **Files:** `crates/ekr-sdk/src/read/kernel.rs:285-328`, `crates/ekr-sdk/src/read/mod.rs:458`, `:552` — cited
- **Files:** `crates/ekr/src/conformance.rs:849`, `systems/ekr/conformance/suite.json` (9 `ekr.kernel.Explain` scenarios) — inferred
- **Also likely (tests):** `crates/ekr-kernel/tests/explain.rs`, `adversary_p1_13_explain_1.rs`, `adversary_p1_13_explain_r2_bound.rs`, `crates/ekr/tests/{mcp,agent_cli,session}.rs`, `crates/ekr-sdk/tests/{read,adversary_read_enums}.rs` — inferred
- **Documents:** `docs/cli.md:231`, `:1005`, `:1931`; `docs/guide.md:566-600`, `:771`; `crates/ekr/src/cli/agent.rs:213-215` — inferred
- **Confidence:** high for the kernel, spec, CLI, MCP and SDK surfaces; medium for replay, read and checkpoint
- **Would collide with:** any unit touching `explain.rs` (the snapshot-cost work at `:301-346`), `ReplayState` or the `RevisionCommitted` arm, `checkpoint.rs`, `VerifiedRead`, the receipt/proposal records in `kernel.yaml` (`story:preparation-blobs-are-reclaimed`), `cli/mod.rs` dispatch, `cli/mcp.rs`, the SDK read types
- **Safety fact:** an incomplete index must make `explain` refuse, not return a shorter chain; the `origin-missing`/`origin-ambiguous` and `lifecycle-disagrees` refusals and `verify()` (`explain.rs:396`) stay — inferred
- **Not established:** whether the index is persisted in checkpoints or rebuilt on restore; how the answer is versioned (no `format` field today); whether the link count changes; how much of the 3.0 MB answer is proposal bytes versus evidence payloads; no benchmark harness exists for the 0.5 s / 200 KB acceptance
