---
format: aep.planning-md/3
id: story:extraction-valid-time
kind: story
status: draft
title: An extracted fact is valid from the time its evidence was observed
tags:
- consumer:cortex
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-sdk/src/document/graph.rs
- confidence: cited
  path: crates/ekr-sdk/src/extraction.rs
- confidence: inferred
  path: crates/ekr/tests/extraction_cli.rs
- confidence: inferred
  path: docs/cli.md
revision: 4
---
## Outcome

A fact applied through extraction is valid from the time its evidence was observed, not from an
unbounded past.

## Starting point (0.0.30)

`Assertion::new` in the extraction path sets no valid time (`crates/ekr-sdk/src/extraction.rs:560-583`;
`crates/ekr-sdk/src/document/graph.rs:273` leaves it unbounded). A consumer that extracts from
dated records (chat messages, ticket comments) loses when each fact was said, and timeline reads
cannot order them.

## Acceptance

An extraction document whose evidence entries carry an observed time applies facts whose
`valid_time.from` equals their cited evidence's observed time (the earliest, when a fact cites
several); a fact citing evidence with no observed time keeps today's behaviour.

## Consumer

`beyond10x/cortex`, `story:document-time-as-valid-time` (blocked on this through
`upstream-blocker:ekr-extraction-valid-time`).

## Scope

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-sdk` — cited
- **Files:** `crates/ekr-sdk/src/extraction.rs:560-591` (`Run::go`: the two `Assertion::new` calls at 563 and 575, then the evidence `citing` fold at 591) — cited
- **Files:** `crates/ekr-sdk/src/document/graph.rs:273` (`Assertion::new` sets `TemporalRange::UNBOUNDED`; `with_valid_time` at 294 already exists, so this file is probably read, not changed) — cited
- **Symbols:** `Run::go`, `Assertion::with_valid_time`, `TemporalRange::since` (`graph.rs:186`), `Evidence::observed_at` (`graph.rs:377`) — cited
- **Also likely:** `crates/ekr-sdk/src/extraction.rs:368-424` (`Claim` is keyed on `valid_time.from`, and `claim` / `held_because` decide `held`) — inferred, the key changes meaning when `from` stops being `None`
- **Also likely:** `crates/ekr/tests/extraction_cli.rs` — inferred, where apply-through-verb-and-SDK tests live; there is no apply test under `crates/ekr-sdk/tests/`
- **Documents:** `docs/cli.md:359-371` (`ekr apply-extraction` step 3, plus the held-claim rule that names valid time) — inferred
- **Documents:** `CHANGELOG.md` `[Unreleased]` — inferred
- **Confidence:** high — the story cites `extraction.rs:560-583` and `graph.rs:273`, and both read as described
- **Would collide with:** any unit touching `Run::go` fact assembly or the `Claim` / `held_because` keying in `crates/ekr-sdk/src/extraction.rs`; any unit editing the `apply-extraction` section of `docs/cli.md`; `CHANGELOG.md` `[Unreleased]`
- **Safety fact:** every evidence id a fact cites is listed in the document (`fact-evidence-unlisted`, `crates/ekr-sdk/src/document/extraction.rs:227-233`), and each listed item has a required `observed_at: Timestamp` (`graph.rs:377`), so the earliest observed time comes from `self.document.evidence` alone. Exposure: a document applied under 0.0.30 (`from = None`) and applied again after this change gets a different `Claim` key (`extraction.rs:368, 383, 480`), is not held, and a second active assertion is added — step 3, unproven
- **Coordinator decisions (2026-10-05):** (1) `observed_at` stays required; the Acceptance's "evidence with no observed time" branch cannot occur with today's types (`graph.rs:377`, `ekr-graph/src/evidence.rs:264`) and is not built. (2) Re-applying a document first applied under 0.0.30 must not add a second active assertion: a held claim whose `valid_time.from` is `None` matches the same claim with a bounded `from`; a test covers it. (3) Read-side ordering is not in scope; the Acceptance checks `valid_time.from` only.
