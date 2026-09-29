---
format: aep.planning-md/3
id: story:validation-without-whole-graph-scans
kind: story
status: active
title: Validation checks a new edge or alias without scanning the whole graph
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T07:27:54Z", actor: "human:timo", revision: 4}
---
## Context

Performance audit of 2026-09-29 (see `story:reads-share-verified-state`). Validation scans the whole
graph per new item:

- every new edge scans all edges (`crates/ekr-kernel/src/validate/candidate.rs:106`): 21% of
  validate at 1×, 47% at 3×; an edge-heavy 10× batch validated in 19.5 s against 0.235 s at batch 1;
- every new node scans all nodes for alias uniqueness (`crates/ekr-kernel/src/validate/structural.rs:173`):
  17–19% of validate.

## Build

Per-(source, edge type) counts and an alias index built once per validation from the basis graph;
the edge list is produced only when an error message needs it. Refusals and their messages are
unchanged.

## Acceptance

- At the 10× shape, validating the edge-heavy batch takes at most 1 s (measured).
- Validation cost for a fixed batch grows at most linearly from 1× to 3× (measured).
- Every refusal and message is unchanged: the kernel suite and the validation tests pass.

## Scope (cited from the audit)

`crates/ekr-kernel/src/validate/candidate.rs`, `crates/ekr-kernel/src/validate/structural.rs`.
