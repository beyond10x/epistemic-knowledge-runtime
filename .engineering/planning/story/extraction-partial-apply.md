---
format: aep.planning-md/3
id: story:extraction-partial-apply
kind: story
status: active
title: One bad extracted fact is skipped and the rest applies
tags:
- consumer:cortex
relations:
- serves: vision:o5
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-integrate/src/extraction.rs
- confidence: inferred
  path: crates/ekr-integrate/tests/extraction.rs
- confidence: inferred
  path: crates/ekr-sdk/src/document/extraction.rs
- confidence: cited
  path: crates/ekr-sdk/src/extraction.rs
- confidence: inferred
  path: crates/ekr/src/cli/extraction.rs
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/tests/adversary_x7_v_2.rs
- confidence: inferred
  path: crates/ekr/tests/extraction_cli.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:51:49Z", actor: "agent:claude", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-05T11:51:49Z", actor: "agent:claude", revision: 6}
---
## Outcome

One bad fact in an extraction document is skipped with a reason, and the rest of the document
applies.

## Starting point (0.0.30)

Reading and planning an extraction document refuse the whole document on the first bad fact
(`crates/ekr-integrate/src/extraction.rs:565-733`; `crates/ekr-sdk/src/extraction.rs` `Plan::of`,
234-353). A model-written document of a few hundred facts then applies nothing because of one
dangling reference or one undeclared property.

## Acceptance

An extraction document of 10 facts, 1 of which cites evidence the document does not carry, applies
9 facts, and the `ExtractionReport` lists the 1 skipped fact with its index and refusal code. A
caller that wants the old behaviour asks for it (strict mode) and gets the whole-document refusal.

## Consumer

`beyond10x/cortex` (one model call per batch; a whole-batch refusal today costs the batch).

## Scope

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-sdk` — cited
- **Files:** `crates/ekr-sdk/src/extraction.rs:234-353` (`Plan::of`) — cited
- **Files:** `crates/ekr-integrate/src/extraction.rs:565-733` (`ExtractionDocument::check`) — cited
- **Symbols:** `Plan::of`, `ExtractionReport` — cited
- **Also likely:** `crates/ekr-sdk/src/extraction.rs:434-453, 546-615` (`apply`: line 439 is `document.check()?`; `Run::go`'s per-fact loop already pushes `RejectedExtraction { item: facts[<i>] }` at 608-613) — inferred
- **Also likely:** `crates/ekr-sdk/src/document/extraction.rs:214, 227-252` (`from_yaml` calls `check()`, which refuses `fact-evidence-unlisted` for the whole document) — inferred
- **Also likely:** `crates/ekr-integrate/src/extraction.rs:529-533` (`read_extraction`) — inferred
- **Also likely:** `crates/ekr/src/cli/extraction.rs:49-71` (the verb runs gates 1 and 2 before `apply`) — inferred
- **Also likely:** `crates/ekr/src/cli/mod.rs:227-231, 884-886` (`Command::ApplyExtraction`, where a strict flag would go) — inferred
- **Also likely:** `crates/ekr/tests/adversary_x7_v_2.rs:303-325`, `crates/ekr/tests/extraction_cli.rs:559-643, 749`, `crates/ekr-integrate/tests/extraction.rs:131, 160` (tests that expect a whole-document refusal) — inferred
- **Documents:** `systems/ekr/domains/integrate.yaml:253-257, 300-375` (refusal codes, `RejectedExtraction`, `ExtractionReport`), `docs/cli.md:157, 370-391, 2086-2090`, `CHANGELOG.md` — inferred
- **Confidence:** medium — the story cites two refusal sites; the command line refuses at four, in order: `read_extraction` → integrate `check` (`cli/extraction.rs:49-55`), SDK `from_yaml` → `check()` (`cli/extraction.rs:57`), `apply` → `document.check()?` (`extraction.rs:439`), `Plan::of` (`extraction.rs:234-353`)
- **Would collide with:** any unit touching the SDK's extraction apply routine (`Run::go`'s fact loop, 546-615, or `HeldReason`/`held_because`), the integrate extraction reader's `check`, the `ExtractionReport` contract in `integrate.yaml`, or the `ekr` CLI `Command` enum
- **Safety fact:** a skipped fact fits the existing report shape: `RejectedExtraction` already carries `item: facts[<index>]` and `refusal: Option<String>` as `<code>: <reason>` (`crates/ekr-sdk/src/extraction.rs:121-134`, `systems/ekr/domains/integrate.yaml:304-315`), so non-strict mode needs no new wire field — step 2, unproven
- **Coordinator decisions (2026-10-05):** (1) Strict mode is both an SDK option and an `ekr apply-extraction --strict` flag; `--strict` keeps today's whole-document refusal. (2) The refusal code is the `<code>:` prefix of `refusal`; no new wire field. (3) All four gates agree: a per-fact defect found by gate 1 or 2 becomes that fact's refusal on the CLI and the SDK path alike; a document-level defect (format, unknown ontology) still refuses the whole document. (4) A skipped fact creates nothing that only it introduced. (5) `story:extraction-supersession` depends on this story (its per-fact refusal uses this mechanism).
