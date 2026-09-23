---
format: aep.planning-md/1
id: task:graph-domain-carries-validation-state-payloads
kind: task
status: draft
title: The graph domain has a carrier for one of the four ValidationState payloads and claims four
relations:
- serves: vision:o2
revision: 4
---
## What is wrong

`systems/ekr/domains/graph.yaml:50-51` says the payload of each `ValidationState` — "validators,
issues, competing assertions, the superseding assertion" — "lives in the fields beside it".
`ekr.graph.Assertion` at `graph.yaml:225-269` declares `superseded_by` and declares **no** field for
the validators of an `Accepted`, the issues of a `Rejected`, the competing assertions of a
`Disputed`, or the reason and revision of a `Retracted`. One of the four has a carrier.

Found by the adversary of wave p1-04, in the same shape as the `ekr-ontology` pass-2 finding of
wave p1-03: a document sentence claiming a projection wider than the projection.

## Why it is not just a comment

`ess/1` has no sum type with per-variant payloads, so the payload cannot go on the variant where
design § 17 puts it. The alternatives the document could take are four nullable field groups on
`Assertion` that contradict each other by construction, or a separate entity per state, or leaving
the payload out of the domain and saying so. Nothing has decided between them.

`crates/ekr-graph/src/assertion.rs` carries the payloads in the Rust variant and states the
departure in its doc comment, quoting the document's sentence verbatim.
`crates/ekr-graph/tests/domain_projection.rs::the_domain_carries_only_the_supersession_payload_of_a_validation_state`
pins both halves.

## What closes this

Either a domain representation for the three missing payloads, or a decision recorded as an ADR
that the domain carries the state name only and the payload lives in the implementing crate. Either
way the comment at `graph.yaml:50-51` is corrected and the quote in `assertion.rs` and the case
above move with it — a patch for the comment half is in the wave p1-04 scratch and was not applied,
because the comment and the quote cannot move separately without turning the case red.

## Reconciliation, 2026-09-22

The inaccurate comment has already been corrected in `systems/ekr/domains/graph.yaml`; the Rust projection guard explicitly documents the remaining missing payload carriers. The task is not wholly stale: representing accepted validators, refusal issues, competing assertions and retraction evidence remains required by the approved persisted-contract work. Keep this residual open and bind the final representation to behavioral projection tests. Do not close the task merely because the comment changed.

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): the plan review of 72779f9 found this task's premise already
resolved in `systems/` at b2b64f8. The only remaining obligation is a binding case in the owning
crate's `tests/domain_projection.rs` that fails if the declaration and the Rust type drift. If that
case already exists, the wave cites it and archives this task.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Verdict: already-held.

- `systems/ekr/domains/graph.yaml:70-97` declares `ekr.graph.AssessmentProjection` (`validators`, `issues`, `competing_assertions`) and `ekr.graph.AssertionLifecycleProjection` (`at_revision`, `reason`, `by`, `effective_from`) — cited
- carried by `Assessment` and `AssertionLifecycle`, `crates/ekr-graph/src/assertion.rs:394,472` — cited
- held by `crates/ekr-graph/tests/domain_projection.rs::assessment_and_lifecycle_retain_independent_payloads` and `::every_declaration_of_the_domain_is_carried_field_for_field` — cited
- not covered: a new Rust variant without a YAML declaration (the enumeration check runs YAML to Rust only) — cited
- the case this body cites, `the_domain_carries_only_the_supersession_payload_of_a_validation_state`, no longer exists — cited
- Confidence: high
