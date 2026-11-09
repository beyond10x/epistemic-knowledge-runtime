---
format: aep.planning-md/3
id: story:explain-names-the-merge
kind: story
status: active
title: An explanation of an assertion a merge rewrote names the merge and its evidence
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:merge-entity-applies
- decomposes: epic:zero-open-issues-2026-11
scope:
- confidence: cited
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/adversary_p1_13_explain_r2_bound.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/adversary_x6_x.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/explain.rs
- confidence: inferred
  path: crates/ekr-sdk/src/read/kernel.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/explain.rs
- confidence: cited
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/provenance.json
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
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

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-kernel` explain projection — cited (story Context and Build 2)
- **Files:** `<tree>/crates/ekr-kernel/src/explain.rs:302` (`about`, which matches only Add/Retract/Supersede), `:182` (`ExplanationLink`), `:374` (`VerifiedRead::explain`) — cited
- **Spec first:** `<tree>/systems/ekr/domains/kernel.yaml:1144-1174` (the `ekr.kernel.ExplanationLink` variants) — cited (Build 1)
- **Symbols:** `ExplanationLink`, `MergeLineage.assertions` / `RewrittenAssertion` (`crates/ekr-graph/src/supersession.rs:197`), `MergeEntity` — cited
- **Renderers:** `crates/ekr/src/cli/explain.rs:312` (matches link kinds by string for the /3 evidence scope), `crates/ekr/src/cli/mcp.rs` — cited (Build 2)
- **Also likely:** `crates/ekr-sdk/src/read/kernel.rs:404` (the SDK's own `ExplanationLink` mirror), `crates/ekr-kernel/src/lib.rs:155` (re-exports) — inferred, a new variant needs both
- **Also likely:** exhaustive `match` on link kinds in `crates/ekr-kernel/tests/adversary_p1_13_explain_r2_bound.rs:182` and `adversary_x6_x.rs:194` will stop compiling on a new variant — inferred
- **Conformance:** new scenario `crates/ekr/tests/fixtures/conformance/scenarios/an-explanation-of-a-merged-assertion-names-the-merge.yaml`, then regenerated `systems/ekr/conformance/{suite,baseline,provenance}.json` — inferred, from how the other merge scenarios are registered
- **Documents:** `docs/cli.md`, `crates/ekr/src/cli/agent.rs:283` (agent help on `ekr.explanation/2`), `CHANGELOG.md` — inferred
- **Confidence:** high — the story names the defect site and the spec file, and the tree matches both
- **Would collide with:** any unit touching `crates/ekr-kernel/src/explain.rs`, the `ExplanationLink` block of `kernel.yaml`, `crates/ekr/src/cli/explain.rs`, or the generated `systems/ekr/conformance/*.json` (every new scenario rewrites these) — cited
- **Safety fact:** `explain` is a projection over `VerifiedRead` and writes nothing; `git grep -i explain` over `crates/ekr-kernel/src/replay.rs`, `commit.rs` and `validate/` finds only doc comments, so no validation verdict or stored byte that replay compares changes — step 2, unproven — cited
