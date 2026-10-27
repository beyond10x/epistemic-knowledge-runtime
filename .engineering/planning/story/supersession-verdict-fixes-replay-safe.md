---
format: aep.planning-md/3
id: story:supersession-verdict-fixes-replay-safe
kind: story
status: draft
title: Four SupersedeNode verdict fixes land without breaking replay of stores that recorded the 0.0.39 verdicts
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node-review-fixes
revision: 1
---
## Context

Wave 2026-10-20a (`story:supersede-node-review-fixes`) fixed four findings of
`review-result:haiku-panel-0-0-39-supersede-node` that change a kernel validation verdict, and the
coordinator took them back out before release 0.0.40:

- **D7** `crates/ekr-kernel/src/validate/supersession.rs:218`, `:140`: a supersession Reference
  refuses still enters the same-transaction retiring set, so other operations get a second, false
  refusal; `X by X` on an absent X is refused twice for absence.
- **D8** `reference.rs:425`, `:283`, `:458`; `structural.rs:246`, `:294`: refusal messages name a
  `by` the same transaction retires, an edge of `by` that does not exist, or a superseded alias
  holder with "use that node".
- **D10** `structural.rs:912`: under profiles v1 and v2 a `CreateNode` may take the id of an edge
  retired with a superseded node (v3 refuses it; `kernel.yaml:741` says every profile).
- **Q7** `transaction.rs:248` and the SDK mirror: duplicate evidence ids in `!SupersedeNode` are
  dropped silently instead of refused.

Why they were held back: replay compares every retained rejection's issues (count, code, message)
with what the current rules answer, byte for byte (`crates/ekr-kernel/src/replay.rs:1203-1213`), and
revalidates committed transactions. 0.0.39 was released on 2026-10-19 to a consumer; a store that
recorded one of these verdicts under 0.0.39 would stop reopening (or stop full replay) under a
release that answers differently.

The implementation exists: `<home>/.cache/ekr-w20a-u1-scratch/verdict-changes.patch` (code, tests,
spec, docs), written by the wave 2026-10-20a implementor against `01830917d0`.

## Build

1. Decide and record how a rule change reaches stores that recorded verdicts under the old rule: a
   rules era (or validation-profile revision) recorded with each verdict and replayed under the era
   it was recorded in, or another mechanism the design already has for this. This is an
   architecture decision; it needs an ADR and the operator's acceptance.
2. Apply the four fixes behind it; a 0.0.39 store holding each of the old verdicts reopens, with and
   without `--full-replay`, on both providers.

## Acceptance

- A fixture store written by 0.0.39 holding a rejection of each kind (D7, D8, D10, Q7) reopens and
  fully replays under the new release, and a new proposal of each kind gets the new verdict.
- The four tests in `verdict-changes.patch` pass.

## Out of scope

Any other rule change.
