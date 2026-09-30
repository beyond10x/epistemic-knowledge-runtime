---
format: aep.planning-md/3
id: task:commit-applies-once
kind: task
status: implemented
title: A commit applies the transaction to the graph once
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T00:47:44Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Wave perf-02's adversary pass on unit C (2026-09-29): the story asked the verify step to reuse the
kernel's `apply` result "instead of applying and hashing it again"; unit C reuses the root only.
Both the decision (`crates/ekr-kernel/src/commands.rs:574`) and the verify step
(`crates/ekr-kernel/src/apply.rs:113`) still clone and apply the whole graph, and the decision
discards its graph. The base did the same; the cost at a consumer's 1× shape is not measured
separately.

## Build

`commit_decision` hands the graph it applied to the publish check, which verifies against it instead
of cloning and applying again, keyed exactly as unit C's remembered root is (prior graph allocation,
prior root and time, transaction hash, validators, commit time). The kernel still produces the graph
and the root (invariant 1).

## Acceptance

- One graph clone and one apply per commit (a counter), on both providers.
- Roots unchanged against a history written before the change; replay equality.
- Commit CPU at batch 57 of the 1× build, measured before and after on one machine.
