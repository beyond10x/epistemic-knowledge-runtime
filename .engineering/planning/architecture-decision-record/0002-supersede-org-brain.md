---
format: aep.planning-md/1
id: architecture-decision-record:0002-supersede-org-brain
kind: architecture-decision-record
status: accepted
title: ADR 0002 — Supersede the v2 engine and instance (D2)
relations:
- decides: initiative:epistemic-knowledge-runtime
revision: 2
---
## Status

Accepted 2026-09-21 with the roadmap. Reopening it is a new ADR.

## Context

v2 is an engine (`beyond10x/org-brain` 0.5.0, seven crates, ≈80k LOC on entity-runtime 0.17.7) and
an instance (`the org-brain instance tree`: 36 definitions, a 5,304-subject store, 26 GB of raw
evidence, a dashboard and an MCP facade). This runtime could wrap that engine as a new core it calls,
or replace it.

## Decision

This runtime supersedes both the v2 engine and the v2 instance. One runtime, one store. The engine's
crates are lift sources — adapter declarations, view templates, cost accounting, the request/receipt
protocol, the MCP facade, the adversary test cases (`docs/predecessors.md` § 10) — never dependencies.
The instance's definitions become the local schema of transient root `v2-store`; its store is
imported under the policy in `docs/predecessors.md` § 9, never wrapped.

## Why

Wrapping keeps two stores with two identity schemes, two meanings of `provenance`, `revision` and
`lifecycle_state` (predecessors § 6, conflicts 1–4) and no membrane between them. ADR 0001 already
removes the kernel the engine sits on. A second engine behind the runtime would be the layer nobody
can explain a fact through.

## Consequences

- P7 carries the cut-over: the v2 instance runs on this runtime before 0.1.0 is released.
- The v2 engine repository is retired after P7's exit evidence, through Atlas's catalog process, not
  by this repository.
- Nothing in `docs/predecessors.md` § 2 is dropped without an ADR.
