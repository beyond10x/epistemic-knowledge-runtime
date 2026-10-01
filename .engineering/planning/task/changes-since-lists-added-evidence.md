---
format: aep.planning-md/3
id: task:changes-since-lists-added-evidence
kind: task
status: implemented
title: ChangesSince lists evidence added after the seed
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
- depends_on: story:add-evidence-operation
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T00:47:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T00:47:42Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:13:40Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

`AddEvidence` (wave ingest-02) lets a transaction add evidence after the seed. `ekr.views`
ChangesSince (0.0.15, `ekr.graph-changes/1`) lists nodes and edges created and assertions added,
superseded or retracted; it has no change kind for evidence added, so an agent asking what changed
does not see new evidence unless an assertion in the same range cites it.

## Build

A change kind `EvidenceAdded` in `ekr.views.ChangeKind` (spec first, `views.yaml`), carrying the
evidence id, its source identity and its payload hash; ordered with the other kinds by revision,
kind, id. The views suite gains the scenario; `GET /changes` and `changes_since` answer it.

## Acceptance

- A range containing an `AddEvidence` lists it once with its revision, on both providers.
- A range without one is byte-identical to its answer before this change.
