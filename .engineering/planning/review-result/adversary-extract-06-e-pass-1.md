---
format: aep.planning-md/3
id: review-result:adversary-extract-06-e-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit E, pass 1
relations:
- reviews: task:one-event-type-rule
revision: 1
---
CONFIRMED

Adversary pass 1 on unit E (`impl/one-event-type-rule` at `ba2a4123`; cases committed as `8fcac9d4`, one red and ignored). The acceptance holds: `/roles`, the timeline and `ekr ocel` agree on event types on every fixture. `/roles` now checks `event` before `observation`, so the overview's `observation_type`, which is always an event type, is served as `event` and never as `observation`. The fixture seed edits hide a visible change: the base seeds lose every event and observation role, since a type with one dated fact per node is no longer an event type. Roles can now move from event to subject across revisions. The Build step's "counts on real stores" has no evidence in the repository; the rule was chosen by the fixture-preservation fallback.

Held: no other event-type computation (viewer JS, MCP, session, the overview summary's count); OCEL `--events`; the 60% threshold evaluated once; abstract types; subtypes; retracted dated facts; `/roles` bytes on readings.

```findings
- file: crates/ekr-views/src/roles.rs
  line: 141
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "/roles checks event before observation, so the overview's observation_type (always an event type) is served as event and never as observation"
- file: crates/ekr/tests/fixtures/view/readings/seed.yaml
  line: 1
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the readings and sessions seeds as they stood at the base lose every event and observation role under the one rule, and the CHANGELOG does not say that single-dated-fact event types stop being events
- file: docs/cli.md
  line: 816
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: a revision adding a dated fact can also demote an event type to subject and drop its observation, which the docs do not state
```
