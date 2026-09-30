---
format: aep.planning-md/3
id: task:candidate-built-once-per-validation
kind: task
status: active
title: Validation builds its candidate view once, and the bench targets compile
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T23:07:50Z", actor: "human:timo", revision: 3}
---
## Context

Wave perf-01's adversary pass on unit W (2026-09-29): `Candidate::of`
(`crates/ekr-kernel/src/validate/candidate.rs:31`) copies every node and edge, and it runs five times
per validation (Structural through `node_types`, Reference, Types, Cardinality, OntologyConstraint);
`reference.rs:180` loops over every stored assertion. Linear, within perf-01's acceptance, but at 10×
a session `validate` still costs 857–1279 ms, of which 734–1017 ms is paid by a one-node transaction
too (the read path, `story:reads-share-verified-state`, is the larger part).

Also found: `crates/ekr-kernel/tests/command_bench.rs:111` does not compile under `--features bench`
(`missing field aliases`); the gate never builds that target.

## Build

Build the candidate view once per validation and hand it to every validator; index assertions by
subject once. Fix `command_bench.rs` so the bench target builds, and add the bench targets to a gate
lane that compiles them.

## Acceptance

- One `Candidate` construction per validation (a counting test).
- `cargo test -p ekr-kernel --features bench --no-run` compiles in the gate.
- Refusals byte-identical (the differential in `adversary_perf_01_validation_scans.rs`).
