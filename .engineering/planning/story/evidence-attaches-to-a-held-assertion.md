---
format: aep.planning-md/3
id: story:evidence-attaches-to-a-held-assertion
kind: story
status: active
title: Evidence attaches to an assertion the store already holds
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o2
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-kernel/src/apply.rs
- confidence: inferred
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/cardinality.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/lifecycle.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/types.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
- confidence: inferred
  path: crates/ekr-sdk/src/document/mod.rs
- confidence: cited
  path: crates/ekr-sdk/src/document/transaction.rs
- confidence: inferred
  path: crates/ekr-sdk/src/read/kernel.rs
- confidence: cited
  path: crates/ekr-sdk/tests/document_drift.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: inferred
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-views/src/quality.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/explain.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/fixtures/conformance/manifest.json
- confidence: cited
  path: crates/ekr/tests/fixtures/conformance/scenarios
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: inferred
  path: docs/sdk.md
- confidence: cited
  path: systems/ekr/conformance/baseline.json
- confidence: cited
  path: systems/ekr/conformance/provenance.json
- confidence: cited
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/domains/graph.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 39
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T09:59:45Z", actor: "human:timo", revision: 38}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:06Z", actor: "human:timo", revision: 39}
---
## Context

A consumer asked on 2026-10-01. In its store about 67,500 of 69,900 assertions cite a whole source
file as evidence, because they came from a full run before per-message evidence existed. The
consumer can now name each assertion's exact message. The only route today is to add an assertion
citing the message and supersede the old one (`!SupersedeAssertion`, `docs/cli.md` operations
table): 67k supersessions of facts that did not change, each a new assertion and a history entry, at
the current commit cost. The consumer's side of that route is built and tested on a fixture and is
held off the live store until this ships.

No existing operation adds evidence to an assertion the store holds (`docs/cli.md`, operations
table: `AddEvidence` adds an entry, `AddAssertion` cites evidence at creation only). The design names
"seek additional evidence" as an operator action (design § on attention, line 1988) without an
operation for it.

## Build

A kernel operation that attaches evidence the store holds, or that `AddEvidence` adds in the same
transaction, to an assertion that is accepted and active:

- The assertion's subject, predicate, object, valid time, lifecycle and its original evidence stay
  as they are. The attachment is its own record: which assertion, which evidence, which revision.
  It is not an edit of the assertion.
- `ekr explain` lists the attached evidence with the revision that attached it; an explanation at an
  earlier revision does not list it.
- Specified in ESS before code (`systems/ekr/domains/kernel.yaml`): the new record (a noun), its
  relation to `Assertion` and `Evidence`, the operation kind after `WidenEdgeType`, and its
  refusals (unknown assertion, retracted or superseded assertion, evidence not retained or added,
  evidence already attached or already cited).
- A design section beside the one for `WidenEdgeType` states how the knowledge root covers
  attachments.
- The SDK's `Operation` gains the variant; `docs/cli.md` documents it.
- Coordinator decisions (2026-10-01, from the consumer's needs): `ekr quality`'s
  `assertions.with_item_evidence` counts attached evidence as it counts cited evidence; a supersession
  does not carry attachments to the replacement, which stay with the superseded assertion.

## Acceptance

- A transaction of `AddEvidence` plus the new operation commits on both providers; the assertion's
  claim fields and lifecycle are byte-identical before and after; `ekr explain` at the new revision
  lists both evidence entries, and at the revision before lists only the original.
- Each refusal above is a named code, with a conformance scenario.
- `ekr quality` counts an assertion whose message evidence was attached later in
  `with_item_evidence`; after a supersession, `ekr explain` of the replacement lists none of the
  original's attachments, and `ekr explain` of the original at the revision before the supersession
  lists them.
- 1,000 attachments in one transaction against a store of 70,000 assertions commit; the time is
  recorded here (no bound before `story:commit-cost-flat-with-store-size` lands).


## Coordinator decisions (2026-10-01, from the scope)

- The operation is `ekr.kernel.OperationKind` index **15**, after `AddAlias` (14); appending
  renumbers nothing, so no existing `validation_hash` changes.
- The attachment record is a collection of the canonical graph. `ekr.graph-document/2` gains it as
  an optional field omitted when empty, and the knowledge root encodes attachments only when there
  are any, so every existing graph, checkpoint and store encodes to the bytes it does today (a test
  holds a store of the base to its recorded roots).
- Refusals reuse `unresolved-assertion` and `unresolved-evidence`; new codes only for an assertion
  that is not active and for evidence the assertion already cites or has attached.
- Attaching to an assertion that the same transaction retracts or supersedes is refused.
- `ekr.explanation/2` gains an attachment link kind without a version bump; the SDK reads it as a
  typed link.
- `ekr quality` counts attached evidence in `with_evidence` as well as in `with_item_evidence`.
- The operation is admitted under every validation profile, as `AddAlias` is.
- The attachment checks look assertions up by id; they do not join the retract/supersede path that
  copies every claim (`validate/lifecycle.rs:20–27`), so the 1,000-attachment acceptance does not pay
  the graph's size.
- Out of this story: the views `describe` and `changes` reads showing attachments.

## Scope

Derived 2026-10-01 by `story-scoper` against `ekr-extract-06` at `d05f87948`; typed entries carry
the same marks. Confidence: medium (operation, validate, apply, SDK, CLI and ESS sites are forced by
the compiler and the `AddAlias` precedent; the graph-collection placement above settles the rest).

- ESS: `systems/ekr/domains/kernel.yaml` (`OperationKind` :49–66, a projection beside
  `AliasAdditionProjection` :657, `CanonicalOperationProjection` :699–713, `ExplanationLink` :972),
  cited; the record in `systems/ekr/domains/graph.yaml` beside `Assertion` :493 and `Evidence` :568,
  inferred.
- Kernel: `transaction.rs` :457–527, :754, :883; `validate/{reference,structural,cardinality,types}.rs`;
  `apply.rs` :181–345; `explain.rs` :140, :338–489; `tests/validation.rs` :1844–1890, cited;
  `validate/lifecycle.rs`, `checkpoint.rs` :591–617, `seed.rs` :730, :824, inferred.
- Graph and store: `crates/ekr-graph/src/canonical.rs` :440, `crates/ekr-store/src/snapshot.rs`
  :25–58, inferred; `crates/ekr-store/src/log.rs` :24–34, cited.
- Views: `crates/ekr-views/src/quality.rs` :143–156, `views.yaml` :1380–1386, cited.
- SDK: `document/transaction.rs` :159–561, `tests/document_drift.rs`, cited; `document/mod.rs`,
  `read/kernel.rs`, inferred.
- CLI and docs: `cli/explain.rs` :36–68, `cli/agent.rs` :273–343, :545–601, `tests/agent_cli.rs`,
  `docs/cli.md`, the design document (§ 102 after § 101), cited; `docs/sdk.md`, `CHANGELOG.md`,
  inferred.
- Conformance: kernel scenarios, fixtures and `manifest.json` (precedent: the `add-alias*`
  fixtures), regenerated by the coordinator.
- Would collide with, in wave extract-07: `story:extraction-verb-shares-the-sdk-path` on
  `docs/cli.md`, `cli/agent.rs`, `tests/agent_cli.rs`, the kernel conformance manifest and suites;
  `task:seed-document-bounds-alias-expansion` on `docs/cli.md`.
