---
format: aep.planning-md/3
id: story:store-quality-report
kind: story
status: draft
title: A read reports a store's quality beyond its size
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
scope:
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: inferred
  path: crates/ekr-views/src/quality.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/cli/quality.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/tests/quality_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/views-baseline.json
- confidence: inferred
  path: systems/ekr/conformance/views-suite.json
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 5
---
## Context

A store's quality is more than its size (asked by a consumer instance, 2026-09-28). Roadmap P6
plans epistemic health metrics (`ekr-metrics`, design § 61, § 75, A9); no story exists.

## Build

A read verb that reports, for a revision: share of assertions with evidence and with per-item
evidence, share of properties under a constraint, nodes of one type sharing an alias or a
canonical name, open ambiguities recorded by resolve, and validation refusals in the last N
transactions. Deterministic, one JSON document, specified in ESS before code.

## Acceptance

- On a fixture store with known counts, every figure equals the fixture's count, on both
  providers.
- Two reads of one revision are byte-identical.

## Consumer input (2026-09-29)

A consumer instance runs its own health report (node, edge and evidence counts, identity and schema
checks, 456 lines) and asks for it in the engine; this story is that report. Its data-free code
check is `story:store-reading-code-names-no-contents`, and its sampled fact-quality method is
`story:fact-quality-by-judged-sample`.

## Scope

Derived 2026-09-29 by `story-scoper`; figures narrowed by the coordinator the same day. **cited** =
read from the story or the tree, **inferred** = a reading that could be wrong.

- **CLI verb:** `crates/ekr/src/cli/mod.rs` (`Command` enum :101, `execute` match :499–577) — cited
- **Session verb:** `crates/ekr/src/cli/session.rs` `admit` (:230–251) matches `Command` with no `_` arm — cited
- **Docs:** `docs/cli.md` verb table (:117) and a verb section; `crates/ekr/tests/docs_cli.rs:12` — cited
- **ESS domain:** `systems/ekr/domains/views.yaml`, a command and a format such as `ekr.store-quality/1` — inferred, the per-revision determinism rules (views.yaml:30–75) fit a report of one revision
- **Conformance:** views scenarios, `views-suite.json`, `views-baseline.json` — inferred
- **Also likely:** `crates/ekr-views/src/{lib.rs,quality.rs}`, `crates/ekr/src/cli/quality.rs`, `crates/ekr/src/cli/agent.rs` (:58), `crates/ekr/tests/quality_cli.rs` — inferred
- **Confidence:** medium — verb, session and docs are forced by the code; the ESS home is inferred
- **Would collide with:** every unit adding a verb (`cli/mod.rs`, `session.rs`, `docs/cli.md`, `agent.rs`); every unit editing `views.yaml` or the views suite

Figures narrowed (coordinator, 2026-09-29), because two of the five have no revision-deterministic source:

- kept: share of assertions with evidence and with per-item evidence (per-item is 0 until `story:add-evidence-operation` lands); share of properties under a constraint; nodes of one type sharing an alias or a canonical name.
- moved: validation refusals are not a function of one revision (a later rejection changes the answer); they are `story:validation-findings-read`.
- moved: open ambiguities have no producer (`ekr_integrate::resolve` records nothing, `crates/ekr-integrate/src/lib.rs:44–48`); they belong to the extraction apply report (`story:extraction-verb-shares-the-sdk-path`).
