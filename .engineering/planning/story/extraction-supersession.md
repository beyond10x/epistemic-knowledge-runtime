---
format: aep.planning-md/3
id: story:extraction-supersession
kind: story
status: draft
title: An extraction document can supersede an earlier assertion
tags:
- consumer:cortex
relations:
- depends_on: story:extraction-partial-apply
- depends_on: story:extraction-valid-time
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-integrate/src/extraction.rs
- confidence: inferred
  path: crates/ekr-integrate/tests/extraction.rs
- confidence: inferred
  path: crates/ekr-sdk/src/document/extraction.rs
- confidence: cited
  path: crates/ekr-sdk/src/extraction.rs
- confidence: inferred
  path: crates/ekr/tests/extraction_cli.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
revision: 4
---
## Outcome

An extraction document can state that a fact replaces an earlier one, and applying it supersedes
the earlier assertion instead of adding a second active one.

## Starting point (0.0.30)

The extraction path only declines to re-assert claims that are already retracted or superseded
(`crates/ekr-sdk/src/extraction.rs:94-108, 399-424`); it has no way to supersede. A consumer that
imports a registry (people and their teams) needs "this value replaced that one" when a source
changes, and today must write its own `SupersedeAssertion` transactions.

## Acceptance

An extraction document with a property fact marked as replacing the active value of the same
subject and property applies one `SupersedeAssertion`: afterwards exactly one assertion for that
subject and property is active, carrying the new value, and the old one is superseded with the new
fact's evidence. A replacement naming no active assertion is refused for that fact only, with a
reason.

## Consumer

`beyond10x/cortex`, `story:structured-from-files-and-drops` (blocked on this through
`upstream-blocker:ekr-extraction-supersession`).

## Scope

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-sdk` — cited, the story names `crates/ekr-sdk/src/extraction.rs:94-108, 399-424`
- **Files:** `crates/ekr-sdk/src/extraction.rs` — cited. Five places change: (1) `apply` builds the claim map from `graph.assertions.values()` at :454-491 and drops each `AssertionId`; supersession needs the id of the active assertion per subject and property. (2) `Run::go` builds each property fact's `Assertion::new` at :560-571 and pushes one operation per group at :603; a replacing fact needs `[AddAssertion, SupersedeAssertion]`. (3) `HeldReason` at :94-108. (4) `claim`/`held_because` at :365-424. (5) "A replacement naming no active assertion is refused for that fact only" goes in the `rejected` branch at :606-614.
- **Files:** `crates/ekr-sdk/src/document/extraction.rs:92-105` (`PropertyFact`, `deny_unknown_fields`) — inferred, the replacement marker needs a field here
- **Files:** `crates/ekr-integrate/src/extraction.rs:220-235` (`PropertyFact`, schemars) and `:665-678` (per-fact reader check) — inferred, the engine reader's twin type rejects unknown fields
- **Symbols:** `Supersession { assertion, by, effective_from }` (`crates/ekr-sdk/src/document/transaction.rs:373-397`), `Operation::SupersedeAssertion` (:179), `Assertion::with_valid_time` (`crates/ekr-sdk/src/document/graph.rs:294`) — cited, already exist
- **Also likely:** `systems/ekr/domains/integrate.yaml:178-189` (`ekr.integrate.PropertyFact`) and `:334-374` (`HeldReason`, `ExtractionReport`) — inferred
- **Also likely:** tests in `crates/ekr/tests/extraction_cli.rs` and `crates/ekr-integrate/tests/extraction.rs`; possibly `crates/ekr/tests/schema_cli.rs` if it pins the extraction JSON Schema — inferred
- **Documents:** `docs/cli.md:2063` (`facts` row), `docs/cli.md:376-385` (`ExtractionReport` table), `CHANGELOG.md` `[Unreleased]` — inferred
- **Confidence:** medium — the story names the primary file; the document-type and contract files follow from `deny_unknown_fields` on both `PropertyFact` twins
- **Would collide with:** any unit touching `Run::go`'s fact loop (:546-615), the claim/held logic (:365-491) or `ExtractionReport`; the `PropertyFact` types in `ekr-sdk/document/extraction.rs` and `ekr-integrate/extraction.rs`; the extraction sections of `docs/cli.md` and `integrate.yaml`
- **Safety fact:** the kernel accepts a supersession only if the replacement's `valid_time.from == Some(effective_from)` and the old assertion's `from <= effective_from` (`crates/ekr-kernel/src/validate/lifecycle.rs:118-127`); an `AddAssertion` in the same transaction counts while it is `Proposed` (`lifecycle.rs:44-52`), so the pair fits one transaction — step 2, unproven
- **Coordinator decisions (2026-10-05):** (1) `effective_from` is the replacing fact's `valid_time.from`, which `story:extraction-valid-time` sets from the earliest cited evidence; this story depends on it. (2) A refused replacement appears in `rejected` with a `<code>: <reason>` refusal, the mechanism of `story:extraction-partial-apply`; this story depends on it too. (3) The implementor traces property cardinality (`crates/ekr-kernel/src/validate/cardinality.rs:134-145`) for two active single-valued assertions inside one transaction and states the result.
