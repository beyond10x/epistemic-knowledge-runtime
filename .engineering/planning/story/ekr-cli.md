---
format: aep.planning-md/1
id: story:ekr-cli
kind: story
status: draft
title: 'The ekr binary: seed, propose, validate, commit, snapshot, explain'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:seed-and-explain
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr/src/cli
- confidence: inferred
  path: crates/ekr/src/exit.rs
- confidence: inferred
  path: crates/ekr/src/main.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures
- confidence: inferred
  path: crates/ekr/tests/retraction_example.rs
revision: 4
---
## Context

Design § 70 names the runtime's controlled interface: snapshot, ingest, propose, validate, commit,
maintain. P1 exposes the kernel half as the `ekr` binary so the epic's exit evidence — the § 65
retraction example — can run end to end from a shell and a conformance suite can drive it.

## Acceptance

The integration test `crates/ekr/tests/retraction_example.rs`, which drives the § 65 example
through the binary step by step, passes.

## Tests the story ships

- The steps of that test: `seed`, `propose` Alice `CEO_OF` Acme, `validate`, `commit`, `propose`
  Bob with Alice's `valid_to`, `validate`, `commit`, `snapshot --valid-at` before and after
  2026-03-12 — each step asserts exit 0, and the two snapshots assert Alice and Bob respectively.
- A refusal (an error outcome of `systems/ekr/domains/kernel.yaml`) exits 2 with the error's name
  on stderr; a fault exits 1.
- `ekr --help` lists exactly the six verbs with the wire names the ESS domain declares.

## Scope

- `crates/ekr/src/main.rs` — clap derive: `seed`, `propose`, `validate`, `commit`, `snapshot`,
  `explain`, each with `--store <path>` and JSON on stdin/stdout, wire names from
  `systems/ekr/domains/kernel.yaml`
- `crates/ekr/src/cli/{seed,propose,validate,commit,snapshot,explain}.rs`
- `crates/ekr/src/exit.rs` — exit codes: 0 outcome, 2 refusal by name, 1 fault
- `crates/ekr/tests/retraction_example.rs`
- `crates/ekr/tests/fixtures/` — the § 65 seed and transactions

## Notes

Depends on `story:seed-and-explain` and `story:eventlog-store`. `ingest` and `maintain` are P2
and P6. Uses only dependencies `story:workspace-crate-skeleton` declared for `ekr`.
