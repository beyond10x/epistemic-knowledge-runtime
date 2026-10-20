---
format: aep.planning-md/3
id: story:explain-names-the-merge
kind: story
status: draft
title: An explanation of an assertion a merge rewrote names the merge and its evidence
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:merge-entity-applies
revision: 1
---
## Context

`review-result:adversary-w20261020b-u1-pass-1` (J1): explaining an assertion a `MergeEntity`
rewrote (`story:merge-entity-applies`) gives the assertion naming `into`, the origin proposal's
`AddAssertion` naming `absorbed`, and no link to the merge; the merge's evidence is not in the
explanation's support (`crates/ekr-kernel/src/explain.rs:302` at `05bd70503`). The coordinator of
wave 2026-10-20b left it out of that wave to keep the wave to its story.

## Build

1. `kernel.yaml` first: an explanation link for the merge that rewrote an assertion (its transaction,
   revision, `absorbed`, `into`, the field rewritten), read from the record's merge lineage.
2. `ekr-kernel` `explain` adds the link and the merge's evidence to the support; `ekr explain` and
   MCP `explain` render it.

## Acceptance

- `an-explanation-of-a-merged-assertion-names-the-merge`: on both providers, the explanation of an
  assertion a merge rewrote holds a link to the merge with its evidence; an assertion no merge
  touched explains as before, byte for byte.

## Out of scope

Explaining a supersession (it rewrites nothing); splits.
