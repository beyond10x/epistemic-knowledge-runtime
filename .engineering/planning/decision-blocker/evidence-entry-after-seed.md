---
format: aep.planning-md/3
id: decision-blocker:evidence-entry-after-seed
kind: decision-blocker
status: open
title: Nobody has decided how evidence enters canonical state after the seed
relations:
- blocks: epic:p2-observation-layer
- informed_by: task:agent-cannot-add-evidence
- blocks: story:add-evidence-operation
revision: 2
---
## Question

After the seed, how does a new `ekr.graph.Evidence` of kind `Observation` come to exist in canonical state so that an assertion can cite one message instead of a whole seed file? Three sub-questions:

- Is it introduced by an operation inside a validated transaction?
- Is it minted by the ingest path when an observation is recorded?
- Or does the proposer cite an `ObservationId` directly, with the evidence record derived at validation?

Relation: `Transaction → Evidence` (may a transaction introduce evidence, and does the proposer or the ingest path own it?).

## Why nobody can read the answer

- `crates/ekr-kernel/src/validate/reference.rs:35`: "P1 has no operation that introduces evidence" (inferred, code). `GraphOperation` (`crates/ekr-kernel/src/transaction.rs:366`) has no evidence-adding variant, and design § 19 lists none either.
- `systems/ekr/domains/graph.yaml:557` declares `ekr.graph.Evidence` with lifecycle `Retained`. It has no command or `relations:` entry saying what creates one. The graph domain states it has no commands at all (`graph.yaml:3-8`).
- `task:agent-cannot-add-evidence` names "observations as evidence through the P2 observation layer" as the proper route, without saying how.
- The provenance profile `retained-admissible-evidence/1` (design § 91.1) verifies the retained payload bytes that each evidence entry cites. Nobody has said what the payload of observation-kind evidence is: the observation record, or the source bytes.

## Options

1. **`AddEvidence` operation.** A transaction carries the new `Evidence` beside the assertion that cites it. The validators check that its observation is retained and that its hash matches.
2. **Minted at ingest.** Recording an observation also records one `Evidence` for it. Proposers cite existing evidence only, and the P1 rule stays unchanged.
3. **Cite the observation.** `Assertion.evidence` may name an `ObservationId`, and the kernel derives the evidence record. This is a model change to `ekr.graph`.

## What it stops

"Assertions cite that observation, not a file" and "evidence can enter after the seed". That is the proper route for `task:agent-cannot-add-evidence`. No story was drafted for either. The task is related here and not absorbed.
