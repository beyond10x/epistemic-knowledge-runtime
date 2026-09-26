---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-01-observe-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit O (observe domain)
relations:
- reviews: story:observe-domain-model
revision: 1
---
## Adversary pass 1 — unit O (`story:observe-domain-model`), HEAD `cf56e094`

Verdict: NEEDS-CHANGE. Cases executed 19→32 (scratch `adversary-observe.sh` plus the implementor's
`check-observe.sh`), red 6, one of which the adversary withdrew (single-state lifecycle is the repo convention).
Origin: introduced 4 (plus 1 infeasible note), pre-existing 1.

Coordinator routing (2026-09-27): findings at `observe.yaml:96`, `:100`, `:34` and `:123` go back to the
implementor; `:117` (struct identity of `ObservedRecord`) is resolved together with `:123`; the
`identity_serde.rs:162` scan list is pre-existing and becomes its own task.

```findings
- file: systems/ekr/domains/observe.yaml
  line: 96
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the required SourceCheckpoint.source_unit_id field rules out option 2 of source-unit-granularity, a checkpoint-to-unit link the blocker holds open, written as a field to avoid a relation
- file: systems/ekr/domains/observe.yaml
  line: 100
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: putting PollHealth and its checked_through window on SourceCheckpoint decides where poll health lives, which options 1 and 3 of checkpoint-unit-cardinality put elsewhere
- file: systems/ekr/domains/observe.yaml
  line: 34
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: checked_through is documented as both the window this poll proves and a per-unit value carried from the last Complete poll, leaving it undefined after a Failed poll
- file: systems/ekr/domains/observe.yaml
  line: 123
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: ObservedRecord models a persisted deduplication index over retained observations, which observation-retention-path says it blocks, and the story's scope does not list it
- file: systems/ekr/domains/observe.yaml
  line: 117
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: ObservedRecord is the only entity in systems/ekr whose identity is a struct rather than an id newtype, and nothing consumes it yet
- file: crates/ekr-core/tests/identity_serde.rs
  line: 162
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the id-type scan lists its domain files by hand and leaves out observe.yaml, so the ekr.observe id types go unchecked
```
