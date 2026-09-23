---
format: aep.planning-md/1
id: task:ess-retained-command-results
kind: task
status: active
title: Adopt executable exact-result retry contracts from ESS
relations:
- derived_from: story:ess-conformance-kernel
- serves: vision:o2
revision: 5
---
## Measured upstream boundary

The independent activation review found that the declared Commit wrong-state
case contradicted exact retry, and document-only Seed had no truthful retained
identity selector. Released ESS0.28.0 refused the four concrete declarations
recorded in the retained writer-contract correction report. Generating more
scenarios from the partial declaration does not close either mismatch.

## Decided correction

Upstream ESS story:retained-command-result-replay owns the bounded capability
at its docs/design/retained-command-results.md. Source proposal cc2609e6
records its independent design review and two corrections. It admits an
effect-free default state refusal and a command-local replays relation to
the original successful result. The suite observes original identity and
typed response before retry; no expected result is supplied to the target.
New replay responses recursively refuse Decimal/Binary64 and use exact
non-floating comparisons. Immediate replay does not prove later-head behavior.

## What closes this

Implement, independently review and publish the upstream source and verified
release. Adopt the actual compiler/library revision and all corresponding
repository CI pins together. Reconcile EKR's Seed/Commit declaration, add
complete retained-revision observation, regenerate exact suites and execute
them through shared real kernel handlers. Preserve the frozen old formats.
No missing scenario or compiler refusal is reclassified as outside scope.

The existing initial-design and durable-record acceptance still require
restart, later head, changed input/context/anchor and no new occurrence/write.
Those are actual EKR tests beyond the generated immediate witness.

## Coordination

This task blocks specification activation and kernel conformance while the
upstream implementation runs in its own managed checkout. It does not mean
the runtime writer or project is complete. The user approved finishing the
project; this is the necessary measured dependency, not a new permission stop.

## Compiler prerequisite discharged

The verified ESS release and EKR compiler pin discharge the upstream compiler
prerequisite. The exact release and independent adopter preflight are recorded
in .engineering/reviews/p1-writer-released-compiler-preflight.md and on
story:version-persisted-contracts. The generated contract includes Seed/Commit
own-result retry and complete wrong-state observations.

Remove this task's blocks edge on specification activation: that prerequisite
is now supplied. Keep the task active and its conformance block because its
acceptance also requires execution through actual EKR handlers, restart and
later-head controls, which remain unfinished. Requiring those runtime results
before implementing the contract would create a sequencing cycle. No runtime
requirement is dropped or counted as passed.

## Wave p1-14 note

Wave p1-14 (2026-09-23): the compiler prerequisite is discharged and `kernel.yaml` :971-974 and
:1112-1115 declare the retained Seed and Commit replays. The remaining obligation, executing them
through the real EKR handlers, is `story:ess-conformance-kernel`'s acceptance; the blocks edge was
a cycle and is removed. This task closes on the same test_result as that story.
