---
format: aep.planning-md/3
id: story:supersession-verdict-fixes-replay-safe
kind: story
status: active
title: Profile v4 carries the D7, D8 and Q7 SupersedeNode verdict fixes; v1-v3 verdicts stay as released (ADR 0102)
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node-review-fixes
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-kernel/src/authority.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commands.rs
- confidence: inferred
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/supersession.rs
- confidence: inferred
  path: crates/ekr-kernel/tests
- confidence: inferred
  path: crates/ekr-sdk/src/document/ontology.rs
- confidence: cited
  path: crates/ekr-sdk/src/document/transaction.rs
- confidence: inferred
  path: crates/ekr-sdk/tests
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/host.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures/conformance
- confidence: inferred
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: inferred
  path: systems/ekr/conformance
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:42:12Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-21T00:42:12Z", actor: "human:timo", revision: 7}
---
## Context

Wave 2026-10-20a (`story:supersede-node-review-fixes`) fixed four findings of
`review-result:haiku-panel-0-0-39-supersede-node` that change a kernel validation verdict, and the
coordinator took them back out before release 0.0.40, because replay compares every retained
rejection's issues (count, code, message) with what the current rules answer, byte for byte
(`crates/ekr-kernel/src/replay.rs:1201-1216`), and 0.0.39 went to a consumer on 2026-10-19.

Atlas ADR 0102 ("A kernel verdict change is a new validation profile; a retained verdict replays
under the profile that decided it", accepted 2026-10-21, atlas `d5d1dcd6`,
`architecture/adr/0102-a-kernel-verdict-change-is-a-new-validation-profile.md`) answers this
story's former Build 1:

- d1: a verdict change is made only as a new validation profile; `ekr.p1-deterministic/1`,
  `ekr.p2-deterministic/1` and `ekr.p3-deterministic/1` are frozen as released in 0.0.39, messages
  included.
- d2: a retained verdict replays under the profile that decided it.
- d3: **D7, D8 and Q7 form profile v4**, ruleset `ekr.p4-deterministic/1`: v3's rules plus the three
  fixes; application stays `ekr.p2-apply/1`.
- d4: **D10 is closed as a misread**: `systems/ekr/domains/kernel.yaml:739-742` refuses cross-kind
  id reuse under v3 only, and the code agrees. It is not built.
- d5: **Q7 is gated at parse**: `NodeSupersession.evidence` (`crates/ekr-kernel/src/transaction.rs`)
  refuses a duplicate only when the profile in force is v4; under v1-v3 the parser keeps collapsing
  duplicates; the SDK mirror follows the same profile.
- d6 (a recorded move of an existing store to v4) and d7 (a 0.0.39 fixture) are
  `story:validation-profile-move`.

The fixes:

- **D7** `crates/ekr-kernel/src/validate/supersession.rs`: a supersession that Reference refuses
  (node absent or already superseded, or `by` already superseded) no longer enters the
  same-transaction retiring set, so other operations get no second, false refusal; `X by X` on an
  absent X is refused once for absence and once as `supersede-into-itself`.
- **D8** `validate/reference.rs`, `validate/structural.rs`: the five refusal messages name the `by`
  the head will hold, say a node's edges are retired rather than moved, and name a superseded alias
  holder as superseded.
- **Q7** `transaction.rs` and `crates/ekr-sdk/src/document/transaction.rs`: duplicate evidence ids in
  `!SupersedeNode` are refused instead of collapsed.

A first implementation exists, unconditional and against `01830917d0`:
`<home>/.cache/ekr-w20a-u1-scratch/verdict-changes.patch` (12 files). Wave 2026-10-20b changed the same
validators since, so it is reworked behind v4, not applied.

## Build

1. Profile v4 `ekr.p4-deterministic/1` + `ekr.p2-apply/1` beside v1-v3 in
   `crates/ekr-kernel/src/authority.rs`, selectable at seed as v3 is; `kernel.yaml`, the design, the
   CLI help (`crates/ekr/src/cli/agent.rs`, `crates/ekr/src/host.rs`) and the SDK ontology document
   name four profiles.
2. D7 and D8 apply only when the profile in force is v4; under v1-v3 every verdict and message stays
   byte for byte as 0.0.41 answers it.
3. Q7 at parse, profile-aware in the kernel and the SDK, at every call site that parses a
   transaction document (commit and replay).

## Acceptance

- A v4 store gives each of D7, D8 and Q7 its new verdict; a v1, v2 and v3 store gives the 0.0.41
  verdict for the same proposal, issue count, codes and messages included.
- Every existing replay and base-era fixture replays unedited.
- The kernel specification and its conformance suite hold v4; `task spec-check` and
  `task conform-fresh` exit 0.

## Out of scope

Moving an existing store to v4 (`story:validation-profile-move`); D10; any verdict not named here
(the merge follow-ups H6 and H7 wait for a later profile).
