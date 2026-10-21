---
format: aep.planning-md/3
id: story:merge-entity-review-follow-ups
kind: story
status: draft
title: 'The deferred findings of the PR #101 review: merge cost, duplicated helpers, two verdicts and the repointed records'' lineage'
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:merge-entity-applies
revision: 2
---
## Context

`review-result:haiku-panel-pr-101-merge-entity` (10 read-only reviewers on PR #101, before its
merge) found 26 findings. Five rows (H1 to H5: 2 blockers, the guide text, a module doc) were fixed
on wave/20261020b before release 0.0.41. This story holds the seven rows deferred:

- **H6** `crates/ekr-kernel/src/validate/cardinality.rs:112`: a `CreateEdge` beside a merge is
  counted against the pre-merge edges, so it is refused `edge-cardinality` when the merge retires
  the edge joining the two nodes or collapses a duplicate onto the created edge. Over-refusal only.
- **H7** `crates/ekr-kernel/src/validate/structural.rs:1146`: `reason: null` and `evidence: null`
  decode to the bare shape and are refused with the P1 text ("MergeEntity is not supported in
  P1"), not `merge-without-reason` / `merge-without-evidence`. Nothing commits.
- **H8** `crates/ekr-kernel/src/merge.rs:381`: records repointed from the absorbed node to `into`
  keep no prior `by` in `MergeLineage`; a split (ADR 0018 decision 4) has to read it from the log.
- **H9** `crates/ekr-views/src/changes.rs:425`: `subject_at` scans every supersession record for
  each superseded or retracted assertion change (A·S per derived revision list, cached per
  revision), against the module's cost statement at `changes.rs:22-25`.
- **H10** cost of one merge: passes over the head without an index in validation
  (`validate/merge.rs:227`, `:317`) repeated at commit (`merge.rs:250`, `:299`); `properties_of`
  resolved per holder (`validate/merge.rs:689`, `:785`); `fates` d·D over parallel duplicates
  (`merge.rs:212`).
- **H11** duplication: `values_admitted`, `cardinality()` and `allows` in `validate/merge.rs`
  (`:845`, `:914`, `:837`) against `types.rs:578`, `:602`, `:184` and `cardinality.rs:161`; the
  MergeEntity evidence loop against the SupersedeNode one (`reference.rs:220`, `:344`);
  `narrow_lineage` / `widen_lineage` against the node and edge property loops (`seed.rs:1076`,
  `crates/ekr-store/src/snapshot.rs:441`).
- **H12** `crates/ekr-sdk/src/resolve.rs:223`: one cache `retain` per committed merge.
- **H13** (wave 2026-10-21b, `review-result:adversary-w20261021b-u1-pass-1`):
  `MergeEntity.evidence` (`crates/ekr-kernel/src/transaction.rs:161`) collapses duplicate ids under
  every profile, v4 included; ADR 0102 d3 names three fixes, so a refusal waits for a later profile.
- **H14** (wave 2026-10-21b, `review-result:adversary-w20261021b-u1-pass-2` finding 5): the SDK's
  `Assertion.evidence` (`crates/ekr-sdk/src/document/graph.rs:239`) is a plain `BTreeSet`, so
  `from_yaml` collapses an `!AddAssertion` citing one evidence id twice that `ekr propose` refuses
  under every profile; profile-independent and pre-existing. The adversary's case
  `from_yaml_refuses_an_assertion_citing_one_evidence_id_twice_as_propose_does`
  (`crates/ekr-sdk/tests/adversary_w20261021b_supersession_v4_pass2.rs`) is kept `#[ignore]` for it.

H6 and H7 change a kernel verdict once 0.0.41 has shipped (replay compares retained rejections byte
for byte, `crates/ekr-kernel/src/replay.rs:1202-1215`), so they wait on the rules-era decision of
`story:supersession-verdict-fixes-replay-safe`. H8 to H12 change no verdict.

## Build

1. H9, H10, H12: an index or one precomputed list where the reviewers name a scan; the cost
   counters in `crates/ekr-kernel/tests/adversary_perf_01_validation_scans.rs` and the views'
   change-list counters cover a merge.
2. H11: the merge checks call the existing helpers; same verdicts, byte for byte.
3. H8: decide with the split story whether the lineage records the repointed records' prior `by`.
4. H6, H7: behind the rules era of `story:supersession-verdict-fixes-replay-safe`.

## Acceptance

- Every `merge_entity` and adversary merge case passes unchanged after H9 to H12.
- A store with S supersession records and no merge derives its change list without reading a
  record per assertion change (a counter shows it).
- H6 and H7: a 0.0.41 store holding each old verdict reopens and fully replays; a new proposal
  gets the new verdict.

## Out of scope

Any other rule change.
