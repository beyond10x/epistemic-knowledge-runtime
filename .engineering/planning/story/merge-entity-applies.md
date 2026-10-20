---
format: aep.planning-md/3
id: story:merge-entity-applies
kind: story
status: active
title: MergeEntity joins duplicate nodes of one type or of related types, and rewrites every reference to the survivor
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: inferred
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-graph/src/supersession.rs
- confidence: cited
  path: crates/ekr-kernel/src/apply.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/cardinality.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/supersession.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/types.rs
- confidence: cited
  path: crates/ekr-kernel/tests/base_era_v1_replay.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/merge_entity.rs
- confidence: cited
  path: crates/ekr-sdk/src/document/transaction.rs
- confidence: inferred
  path: crates/ekr-sdk/src/read/kernel.rs
- confidence: inferred
  path: crates/ekr-sdk/src/read/views.rs
- confidence: inferred
  path: crates/ekr-sdk/src/resolve.rs
- confidence: inferred
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-views/src/changes.rs
- confidence: inferred
  path: crates/ekr-views/tests/changes.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/conformance.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures/conformance/manifest.json
- confidence: inferred
  path: crates/ekr/tests/fixtures/conformance/scenarios
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: docs/overview.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/provenance.json
- confidence: cited
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/conformance/views-baseline.json
- confidence: inferred
  path: systems/ekr/conformance/views-provenance.json
- confidence: cited
  path: systems/ekr/conformance/views-suite.json
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T10:50:38Z", actor: "agent:claude-ekr-session", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-20T10:50:39Z", actor: "agent:claude-ekr-session", revision: 10}
---
## Context

`MergeEntity { absorbed, into }` is declared (`systems/ekr/domains/kernel.yaml:632-638`) and refused
as `unsupported-operation` (`crates/ekr-kernel/src/validate/structural.rs:1108-1121`,
`apply.rs:486`). The design is `architecture-decision-record:0018-merge-and-split-identity`
(proposed 2026-10-19), which carries the three rules the operator cleared on 2026-10-14 on
`decision-blocker:merge-absorbed-node-fate`, `decision-blocker:merge-across-node-types` and
`decision-blocker:split-reverting-merge-identity`.

Narrowed on 2026-10-19: it builds on `story:supersede-node`
(`architecture-decision-record:0017-a-node-is-superseded-by-another`), which already gives removal
from every head read, the supersession record, repointing with no chain, alias resolution through
the record, the alias holders, `node-superseded` on reads, and views that skip assertions about a
retired node or edge. This story adds what a supersession does not: the shaped `EntityMerge` and the
gate on the bare shape; the parent-type and property rules; the in-place rewrite of edges and
assertions, duplicate collapse, and endpoint and cardinality revalidation; the merge lineage on the
record.

## Build (spec first)

1. `kernel.yaml`, `graph.yaml` and `views.yaml` first, green under `ess specify validate --path
   systems/ekr` (the pinned `ess` and the newest), with the conformance suites regenerated red:
   - `kernel.yaml`: `ekr.kernel.EntityMerge` gains `reason` and `evidence`, encoded only when
     present; the bare shape is documented as the P1 shape, refused `unsupported-operation` under
     every profile; validation rules and named refusals as ADR 0018 decision 6.
   - `graph.yaml`: the merge lineage (ADR 0018 decision 5) as an optional field of the
     superseded-node record, encoded only when present, with its relations to `ekr.graph.Edge` and
     `ekr.graph.Assertion`. No new `Node` lifecycle state.
   - `views.yaml`: a merge appears in changes.
2. `ekr-kernel` validation: the structural shape gate, keeping `structural.rs:1108-1121` byte-exact
   for the bare shape; `reference.rs:171` plus `node-superseded` for either side; the `parents`
   rule, undeclared properties and endpoint revalidation of rewritten edges and relation assertions
   (`validate/types.rs:397`, `:466`, `:511`); cardinality counted after the collapse
   (`validate/cardinality.rs:138`); property conflicts.
3. `ekr-kernel` apply and replay, both providers: apply (`apply.rs:486`) builds the record through
   `SupersedeNode`'s apply, then rewrites in place and fills the lineage; the asserted-edges lookup
   is invalidated by a merge that rewrites edge-subject assertions (`replay.rs:158-182`).
4. `ekr-views`: a `MergeEntity` arm in changes (`crates/ekr-views/src/changes.rs:487`).
5. `ekr-sdk` `Operation` mirror and agent help list `MergeEntity` as applied
   (`crates/ekr/src/cli/agent.rs:170-171` says it is not).
6. `docs/cli.md` (the operation, its refusals, a worked example `crates/ekr/tests/docs_cli.rs` runs on
   both providers), a dated design amendment to § 46, `CHANGELOG.md` under Unreleased.

## Acceptance

Kernel tests and conformance scenarios, on both providers, fixtures in the runtime's own vocabulary:

- `a-merge-of-same-type-duplicates-rewrites-every-reference`: no head edge or assertion names
  `absorbed`; reading it refuses as `node-superseded` naming `into`.
- `a-merge-keeps-assertion-ids-and-bytes`: a rewritten assertion keeps its id, evidence, assessment,
  lifecycle and both times; at the prior revision its subject is `absorbed`.
- `a-merge-collapses-a-duplicate-edge`, and `a-merge-refuses-conflicting-duplicate-edges`.
- `a-merge-that-would-exceed-edge-cardinality-is-refused`, naming both edges.
- `a-merge-into-a-parent-type-keeps-the-survivors-type`.
- `a-merge-into-a-parent-type-revalidates-endpoints`: a child-only end refuses as
  `edge-endpoint-type`.
- `a-merge-across-sibling-types-is-refused`.
- `a-merge-is-refused-while-absorbed-holds-an-undeclared-property`; after the property is cleared in
  an earlier revision, it commits.
- `a-merge-moves-absent-properties-and-refuses-conflicting-ones`.
- `a-merge-records-its-lineage`: each rewritten edge end, collapsed edge, retired joining edge,
  rewritten assertion field with its prior reference, and moved property.
- `a-merge-of-a-survivor-repoints-earlier-records`.
- `a-merge-of-an-absorbed-node-is-refused`, and a merge into one is refused too.
- `a-merged-alias-resolves-to-the-survivor`.
- `a-bare-merge-keeps-its-retained-verdict`: the base-era fixture reopens unedited
  (`crates/ekr-kernel/tests/base_era_v1_replay.rs`), and the bare shape is refused with P1's message
  under every profile.
- `a-merge-is-refused-by-name`: one case per refusal.
- `a-merge-replays-to-identical-roots`, with and without `--full-replay`; a store that never merges
  keeps byte-identical roots.

## Out of scope

`SplitEntity`, its record and a read of the lineage (ADR 0018 decision 4 fixes only its identity
rule); `RetypeNode`; more than one pair per operation; bulk or predicate merges; duplicate detection.

## NodeRef property values

Added 2026-10-19 from `review-result:adversary-w20261019a-u1-pass-1` (J1): after `SupersedeNode`,
a NodeRef property value written before the supersession keeps naming the superseded node, as
assertions do (ADR 0017 decision 3; `crates/ekr-kernel/src/apply.rs:333` at `7f0deb561`). A merge
rewrites every reference to `absorbed`, so its rewrite includes NodeRef values in node and edge
properties, recorded in the lineage beside the rewritten assertion fields, with an acceptance case
`a-merge-rewrites-node-ref-values`.

## Scope

Derived 2026-10-20 by `aep:story-scoper` at `efe53653` (0.0.40). Every line is **cited** (read from
the story, ADR 0017/0018 or the tree) or **inferred**. Line numbers are as of `efe53653`; where the
story's numbers drifted, the current ones are given (`apply.rs:486` is now `:559`,
`structural.rs:1108-1121` is `:1118-1145`, `changes.rs:487` is `:551`, `reference.rs:171` is `:193`,
ADR 0018's `replay.rs:1187-1211` is `:1202-1215`).

- **Primary surface:** `crates/ekr-kernel` (validation, apply, replay) — cited
- **Spec:** `kernel.yaml:71`, `:637-642` (`ekr.kernel.EntityMerge`), `:733-736`, `:814`;
  `graph.yaml:380-384`, `:390` (`SupersededNodeRecord`), `:525-550` (`ekr.graph.SupersededNode`);
  `views.yaml:1991-1993` (`ChangeKind`), `:2008-2025` (`GraphChange`) — cited
- **Suites:** `systems/ekr/conformance/suite.json`, `views-suite.json`, regenerated red — cited;
  baselines, provenance, `crates/ekr/tests/fixtures/conformance/{scenarios/,manifest.json}`,
  `crates/ekr/tests/conformance.rs` — inferred
- **Wire shape:** `crates/ekr-kernel/src/transaction.rs:125-137` (`EntityMerge`, derives `Copy`),
  `:811-814`, `:898-905` (encode), `:983` — cited
- **Structural gate:** `validate/structural.rs:110-111`, `:957-972` (`merge-into-itself`),
  `:1118-1145` (bare-shape `unsupported-operation`: guard `:1130-1135`, message `:1142`) — cited
- **Reference:** `validate/reference.rs:193-196`, `:425-430` — cited
- **Same-transaction refusal:** `validate/supersession.rs:80-132` (`named`; merge at `:112`), `:135` —
  cited
- **Endpoints:** `validate/types.rs:397`, `:468`, `:514-540` — cited; **cardinality:**
  `validate/cardinality.rs:103-138` — cited; **candidate:** `validate/candidate.rs:172-213`, `:263` —
  inferred
- **Apply:** `apply.rs:559-561`, `:326-396` (`supersede`), `:578`, `:108` — cited; **replay:**
  `replay.rs:158-191`, `:1202-1215` — cited
- **Record and lineage:** `crates/ekr-graph/src/supersession.rs:41-88` — cited; record ripple in
  `ekr-kernel/src/seed.rs:1052`, `ekr-store/src/snapshot.rs:421`, `ekr-sdk/src/read/kernel.rs:98-113`;
  `RetiredEdges` `supersession.rs:113-140`, `canonical.rs:481-499` — inferred
- **Views:** `crates/ekr-views/src/changes.rs:549-567`, `:273`, `:296`, `:773` — cited;
  `crates/ekr-views/tests/changes.rs` — inferred
- **SDK:** `crates/ekr-sdk/src/document/transaction.rs:199-200`, `:235`, `:290-291`, `:476-492` —
  cited; `src/read/views.rs:1020-1037`, `src/resolve.rs:191-217` — inferred
- **CLI:** `crates/ekr/src/cli/agent.rs:170-171`, `:404`, `:574-581` — cited; tests pinning "not
  applied": `crates/ekr/tests/agent_cli.rs:253`, `:1267`, `:2080-2162`, `:2329`;
  `crates/ekr/tests/docs_cli.rs:2343`, `:2569` — cited
- **Retained fixture:** `crates/ekr-kernel/tests/base_era_v1_replay.rs:33`, `:226-258`; must reopen
  unedited — cited
- **Struct-literal ripple:** `EntityMerge {…}` in 8 kernel tests and `ekr-sdk/tests/document_drift.rs`;
  `SupersededNode {…}` in 7 tests — inferred
- **Documents:** `docs/cli.md:2434`, `:2456`, `:3851-3852`; design § 46 (`:2030`), a dated
  amendment; `CHANGELOG.md`; `README.md:39` and `docs/overview.md:236` say "refused" — cited
- **Confidence:** high for kernel, spec, views, SDK, CLI; medium for the record ripple and the index.
- **Would collide with:** any unit in `ekr-kernel/src/validate/*`, `apply.rs`, `replay.rs`,
  `ekr-graph/src/supersession.rs`, the three spec files and their suites, `docs/cli.md`,
  `CHANGELOG.md`, `README.md`, `agent.rs`, `agent_cli.rs`, `docs_cli.rs`.
- **Safety fact:** the bare `{absorbed, into}` shape keeps its two-id encoding
  (`transaction.rs:898-905`) and its one structural issue (`structural.rs:1130-1142`); replay compares
  retained issues by count, code and message (`replay.rs:1202-1215`). The new fields stay out of serde
  and `Canonical` output when unset, and no validator adds an issue for the bare shape.

## Coordinator decisions

Taken by the coordinator of wave 2026-10-20b on 2026-10-20, inside ADR 0018, on the questions the
story's scoper could not settle:

1. **NodeRef values in assertion value objects.** A merge rewrites, in place, every `NodeRef` naming
   `absorbed` inside an assertion's `Object::Value` (at any depth), as it rewrites node and edge
   property values (section "NodeRef property values") and subjects and objects (ADR 0018 decision
   8); the lineage records each with its prior reference. ADR 0017's D4 clarification concerns a
   supersession, which rewrites nothing; a merge is the operation that rewrites every reference.
2. **Collapsed edges.** An edge collapsed onto one `into` already holds (ADR 0018 decision 7(b)) goes
   into the record's `edges`, as a retired edge does, and the lineage maps its id to the kept edge;
   `RetiredEdges` and `retired_with` keep answering for it.
3. **Changes.** A merge appears in `changes_since` as a new `ChangeKind::NodeMerged`, appended after
   `NodeSuperseded`, naming the absorbed node's type, name and evidence and `into` as its target.
4. **`ekr.integrate.Merge`** in `integrate.yaml` is left as it is.

Release: 0.0.41 on green CI (operator, 2026-10-20).
