---
format: aep.planning-md/3
id: story:validation-findings-read
kind: story
status: active
title: A read returns the validation findings of committed transactions
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
scope:
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T20:35:58Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T20:35:58Z", actor: "human:timo", revision: 6}
---
## Context

A consumer instance's health report cannot read validation findings, and skips the figure. The
first draft of this story asked for the findings of committed transactions. The tree has none: a
transaction is sealed only when no validator raised an issue (`crates/ekr-kernel/src/validate/mod.rs:223`),
`CommitReceiptV1` and `ValidationReceiptV1` carry no issues (`crates/ekr-kernel/src/records.rs:237`,
`:299`), and there is no non-blocking warning tier. The findings that exist are those of rejected
transactions, kept in `RejectionRecordV1.issues` (`records.rs:423`) and already returned by
`Runtime::transactions()` through `TransactionRecord.rejection` (`crates/ekr-kernel/src/replay.rs:44`);
`ekr transactions` does not print them. Re-scoped 2026-09-29 by the coordinator to that reading.

## Build

A read verb that returns, for a range of basis revisions, each rejected transaction with its
validation issues (`ekr.kernel.ValidationIssueRecord`: validator, code, message), keyed on the
revision the rejection was validated against. Deterministic, one JSON document, specified in
`systems/ekr/domains/kernel.yaml` beside `ekr.kernel.Transactions` before code. Served by the one-shot
verb and by `ekr session`. No new record format.

A warning tier for committed transactions is out of scope; it would be a kernel validation change
with its own story.

## Acceptance

- On a fixture with planted rejections, the read lists every planted rejection with each of its
  issues, on both providers.
- A committed transaction never appears.
- Two reads of one range are byte-identical, before and after an unrelated commit.

## Scope

Derived 2026-09-29 by `story-scoper`, re-scoped by the coordinator. **cited** = read from the tree,
**inferred** = a reading that could be wrong.

- **Files:** `crates/ekr/src/cli/mod.rs` (`Command` enum, dispatch :499-577) — cited
- **Files:** `crates/ekr/src/cli/session.rs:230-250` (exhaustive `Command` match) — cited
- **Files:** `docs/cli.md` (verb table :117-135, a verb section, session verb list :527) — cited
- **Files:** a new verb module under `crates/ekr/src/cli/`, following `transactions.rs` — inferred
- **ESS domain:** `systems/ekr/domains/kernel.yaml`, a view beside `ekr.kernel.Transactions` (:1369) reusing `ekr.kernel.ValidationIssueRecord` (:374) — inferred
- **Conformance:** kernel suite scenarios in `crates/ekr/tests/fixtures/conformance/scenarios/`, regenerated `systems/ekr/conformance/suite.json` and `baseline.json` — inferred
- **Also likely:** `crates/ekr/src/conformance.rs`, `crates/ekr/tests/docs_cli.rs`, `crates/ekr/src/cli/agent.rs` — inferred
- **Not touched:** `crates/ekr/src/main.rs`, `crates/ekr/src/cli/mcp.rs` — cited
- **Confidence:** medium after the re-scope — the CLI part is forced by the code; the domain placement is inferred
- **Would collide with:** any unit adding an `ekr` verb; any unit changing `kernel.yaml` or the kernel suite
- **Open:** a rejection has no revision of its own; the key is its validation basis (`ValidationBasisV1`, `records.rs:168`), not yet read
