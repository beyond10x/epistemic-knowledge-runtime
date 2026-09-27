---
format: aep.planning-md/1
id: architecture-decision-record:0003-crate-prefix-ekr
kind: architecture-decision-record
status: accepted
title: ADR 0003 — Crate prefix ekr- (D3)
relations:
- decides: initiative:epistemic-knowledge-runtime
revision: 2
---
## Status

Accepted 2026-09-21 with the roadmap.

## Decision

Crates are prefixed `ekr-` (`ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store`, `ekr-observe`,
`ekr-adapters`, `ekr-import`, `ekr-incubate`, `ekr-interpret`, `ekr-integrate`, `ekr-views`,
`ekr-mcp`, `ekr-frontier`, `ekr-schema`, `ekr-runtime`, `ekr-maintain`, `ekr-metrics`). The binary is
`ekr`. ESS domains live under `systems/ekr/`. The repository keeps the name
`epistemic-knowledge-runtime`.

## Why

v2 used `brain-*`, entity-runtime uses `entity-*`; a short prefix keeps crate names readable in a
workspace of seventeen. `ekr` is the design document's own abbreviation of the system's name.

## Open

`atlas/AGENTS.md:443` prefers a plain lowercase noun for a repository name. This is flagged for the
operator and not applied here; a rename would be an Atlas catalog action, not a change in this
repository.
