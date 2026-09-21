---
format: aep.planning-md/1
id: story:ess-conformance-kernel
kind: story
status: draft
title: ESS conformance suite over the kernel domain
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:ekr-cli
- verifies: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: crates/ekr/tests/conformance.rs
- confidence: inferred
  path: systems/ekr/conformance/suite.json
revision: 4
---
## Context

`systems/ekr/` is the contract the P1 crates implement. A specification nobody runs a suite
against is prose. ESS synthesises the scenarios the kernel domain obliges — every command outcome,
every refusal, every lifecycle move — and holds a target to them; the report is what moves the
specification artifact to `conforming` in the planning store.

## Acceptance

Every scenario in the report `ess verify conform run --path systems/ekr` writes against the `ekr`
conformance target has status `passed`.

## Tests the story ships

- `crates/ekr/tests/conformance.rs` synthesises the suite, runs it, and fails on any scenario
  whose status is not `passed` — a skipped scenario fails it as a failed one does.
- The committed `systems/ekr/conformance/suite.json` is byte-identical to a fresh synthesis.
- The report's scenario count equals the suite's.

## Scope

- `crates/ekr/src/conformance.rs` — the target: `ExecuteCommand` and `QueryView` over the kernel
  library for `ekr.kernel.*` commands and views
- `crates/ekr/tests/conformance.rs`
- `systems/ekr/conformance/suite.json` — the synthesised suite, committed so drift is visible
- `Taskfile.yml` — a `conform-check` task added to `check`

## Notes

Depends on `story:ekr-cli`. Closing step, after the acceptance holds: record the report with
`aep plan artifact evidence executable-system-specification:ekr-v1 --from <report>` and move the
specification to `conforming`; that move is the store's decision, not this story's acceptance.
The `ess-specify:coverage` skill is the reference for raising executed scenarios. Adds no
dependency beyond the skeleton's.
