---
format: aep.planning-md/3
id: story:merge-entity-applies
kind: story
status: draft
title: MergeEntity joins duplicate nodes of one type or of related types, and rewrites every reference to the survivor
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node
revision: 4
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
