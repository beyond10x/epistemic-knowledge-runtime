---
format: aep.planning-md/1
id: task:current-revision-ranking-is-unsynthesizable
kind: task
status: draft
title: CurrentRevision ranking cannot be synthesized while Seed alone creates a Revision
relations:
- serves: vision:o2
- derived_from: story:ess-conformance-kernel
revision: 1
---
## What is wrong

`systems/ekr/domains/kernel.yaml` `ekr.kernel.CurrentRevision` carried `order_by: [number desc]`
until wave p1-14 (`e128e4e`). `ess conform synthesize` (ESS 0.29.0) builds the ranked scenario by
creating a second `ekr.kernel.Revision` through the only command the model declares as creating
one, `Seed`, and expects `seeded` with at least 2 rows. The kernel answers the second Seed with
`already-seeded`, so `ekr.kernel.Seed/outcome/seeded` failed on both providers (unit
p1-14-conformance: 35 of 36 passed). No correct implementation can satisfy it: later revisions come
from `Commit`, which the model announces through `ekr.kernel.RevisionCommitted` because one
outcome moves one subject.

The ranking obligation was removed; the view's ordering is now unasserted by conformance.

## What still holds head ordering

`crates/ekr/tests/retraction_example.rs::the_retraction_example_runs_through_fresh_processes_on_both_providers`
and `crates/ekr-kernel/tests/durable_commands.rs` read the head after commits (cited by the
conformance unit's scope; not re-run for this task).

## What closes this

Either ESS lets an outcome declare a second created subject (or synthesis sets up extra instances
through any command whose event names the entity), and `order_by` returns with a generated
scenario that passes; or the decision that `CurrentRevision` is unordered is recorded in the
domain. Upstream question for the ESS repository; not a P1 exit obligation.
