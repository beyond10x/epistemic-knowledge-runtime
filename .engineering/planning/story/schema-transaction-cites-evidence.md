---
format: aep.planning-md/3
id: story:schema-transaction-cites-evidence
kind: story
status: active
title: A schema transaction cites the evidence that introduced it
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o2
- depends_on: story:show-disputed-knowledge
- derived_from: release-plan:knowledge-inbox-schema-learning
scope:
- confidence: inferred
  path: crates/ekr
- confidence: inferred
  path: crates/ekr-kernel
- confidence: inferred
  path: crates/ekr-ontology
- confidence: inferred
  path: crates/ekr-sdk
- confidence: inferred
  path: crates/ekr-views
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T14:55:25Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-03T14:55:26Z", actor: "agent:codex-ekr-knowledge", revision: 10, decided_on: {"recorded":{"approval":1}}}
---
## Context

A consumer asked on 2026-10-01 for schema transactions that cite evidence.

The consumer grows its schema one type, edge type or property per version, each just before the
first fact that uses it. It wants each version to cite that fact's message, so that
`ekr ontology --at` and the viewer's schema lineage show which message introduced each type. Today
it keeps that link in its own `apply.json`.

Three rules block this. A transaction's `evidence` must be exactly the set its `AddAssertion`s
cite. `AddEvidence` is a data operation, so carrying it in a schema transaction refuses as
`MixedSchemaTransaction`. And `Transaction` denies unknown fields.

## Build

Let a schema transaction cite evidence the store retains, or carry an `AddEvidence`, without counting
as mixed. The version records the cited evidence, and the schema reads (`ekr ontology --at`, the
projection's schema lineage) show it. This is a schema-evolution change: it is specified in ESS
first, and its design section sits beside the schema-evolution sections.

## Acceptance

- A schema transaction that cites one retained evidence entry, and one that adds and cites one,
  each commit as a new schema version on both providers.
- `ekr ontology --at <that revision>` and the lineage name the evidence.
- A schema transaction that carries any other data operation is still refused as
  `MixedSchemaTransaction`.

## Knowledge inbox delivery scope

Reuse this story as D of release-plan:knowledge-inbox-schema-learning; do not replace its original consumer request. It follows story:show-disputed-knowledge's authority/evidence foundations and precedes approved proposal integration. Source contract commit 07604bf1a5e6c19ef86b78846dc99a603f3d2bfb, spec_digest 29e2f5d0af0bb99248bb46d0dc1f538dedab7d5d209ca07c0fce5d0ac60a1a4e, validates before implementation.

Named scenario schema_change_exposes_supporting_evidence runs against both file and SQLite, including reopen/full replay. Keep mixed schema/data refusals except explicitly admitted evidence operations. Expose supporting evidence through typed SDK, ontology CLI and read-only viewer schema history. Use generated contracts, preserve canonical evidence retention and ordinary validation; full task check plus real conformance closes this story.

## Scope

Scoper source review: ekr-sdk/src/document/transaction.rs:60,115–128 currently derives assertion/attachment evidence and refuses mixed schema transactions; ekr-kernel/src/validate/provenance.rs:98 and reference.rs:25 admit evidence; read.rs:93 schema history lacks per-version evidence links. Inferred implementation: kernel admission/replay under new profile, schema metadata, SDK supporting-evidence builder, cli/ontology.rs:52, ekr-views/src/document.rs:238,499 and index.rs:356. Extend schema_evolution.rs:382, schema_evolution_replay.rs:455 and add_evidence.rs:204,436. Preserve old-profile refusals and all other mixed schema/data refusals.


## Kernel checkpoint in progress

Kernel authority/evidence and SDK builder checkpoints are retained on managed branch ekr/schema-evidence-20261003. Presentation and reviewed corrections are committed as 4f2c29ab82f72942aeba2a2aeabaa934a03a64cc. The ESS-generated SchemaEvidenceEntry carries immutable manifest citations through ontology CLI, SDK typed reads, graph projection, overview and Rust-rendered inbox. Both providers pass the named Rust case after reopening and full replay, including historical selection, retained bytes and HTML escaping.

Review-result:schema-evidence-kernel-independent-r1 found a masked mixed-data refusal test. Clearing its unrelated manifest mismatch and requiring mixed-schema-transaction exposes removal of the actual guard on both providers; restored code passes. Review-result:schema-evidence-presentation-independent-r2 found a real explanation failure after a second authority transition. The regression reproduced validation-profile-disagrees; verified authority boundaries are now retained and explanations select the boundary active at validation. Review-result:schema-evidence-authority-fix-independent-r3 approves the bounded correction. These are independent source/log reviews, not independent test executions.

Curated executable evidence, specification digests and inspected viewer screenshot are retained in .engineering/reviews/knowledge-schema-evidence-presentation/README.md and its adjacent logs. Fresh generated contracts, specification/suite regeneration, focused regressions, formatting and touched-crate clippy passed. The authored ESS schema-evidence scenario and complete integrated gate remain required; story D and PR64 are incomplete.

Attribution correction: the earlier draft-to-proposed transition attributed to human:timo was issued by the coordinating agent without setting AEP_ACTOR. It relied on the existing operator execution request and was not a separate human action. The immutable transition is preserved; subsequent writes explicitly identify agent:codex-ekr-knowledge.

## Authored conformance checkpoint

The authored ESS scenario schema-change-exposes-supporting-evidence executes real SDK-built schema transactions on file and SQLite providers. It observes literal retained/inline evidence IDs from production rendering after reopening with full replay and verifies historical citation boundaries and retained payload hashes. Both native provider reports pass. Both the inert target and a real production mutation that drops citations fail that exact scenario; restored code passes. Original report/2 documents and complete selection lineage are retained under .engineering/reviews/knowledge-schema-evidence-conformance/.

The independent bounded source/log review is review-result:schema-evidence-conformance-independent-r4 and finds no actionable defects. This is not an independent execution. Fresh generated contracts, complete suite regeneration, the views inventory, the complete knowledge target and touched-target clippy passed; the full integrated task check remains pending.

The first uncommitted evidence imports captured local absolute report paths despite an explicit reference. Those unpublished records are preserved in private raw scratch, and the CLI imports were recreated using committed relative report/input paths. No committed record or historical transition was rewritten. The story remains active until the integrated gate and delivery requirements pass.

## Integrated gate checkpoint

D source is integrated locally at fe9efea8a; its authored schema evidence scenario and actual citation-removal mutation have passed on both native providers. Full-gate attempts then exposed mutation-control classification, tmpfs inode staging, and missing generated knowledge-command adapters. These are corrected without lowering any floor.

The final focused correction run records 35 Rust tests passed, zero failures. Kernel report/2 records cover the complete component inventory; fresh contracts/specification/suites and warnings-denied CLI clippy pass.

Independent review also found a write-before-refusal observation gap. A real appended-proposal probe reproduced it, removing only the guard made that probe fail, and the corrected adapter checks publication boundaries. Reviews r5/r6/r7 and disposition evidence are retained.

The full gate remains red: the integration suite reports unsupported retained-knowledge routes and unfinished E/F schema-proposal operations. This is pending implementation, not an upstream or operator blocker. D remains active until the integrated acceptance gate passes. Continue E/F under the existing sole-carrier authorization; do not close unrelated planning work or undraft PR64 yet.

Reports, mutation logs, first red gate attempts and final checks are in .engineering/reviews/knowledge-integrated-conformance/.
