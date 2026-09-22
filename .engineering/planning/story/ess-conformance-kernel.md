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
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: crates/ekr/tests/conformance.rs
- confidence: cited
  path: systems/ekr/components.yaml
- confidence: cited
  path: systems/ekr/conformance
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 8
---
## Context

The executable kernel contract is systems/ekr. The original prerequisite report
correctly identified missing payload inputs and incomplete retained transaction
views. DESIGN 91–94 and the active kernel ESS now provide document-path inputs,
complete retained decisions, all-state Transactions and command response records.
The original findings remain in the planning journal; they are no longer missing
model prerequisites. Actual target execution is still outstanding.

## Acceptance

A Rust ConformanceTarget runs the complete admitted kernel suite through the
same real Runtime and typed read/command handlers as the CLI, on both native
providers. Every selected scenario passes, with no failed, error, unsupported or
skipped terminal result. The declared-coverage inventory is complete and has no
unresolved refusal. The report is produced from the actual ExecutedRun and paired
with the exact admitted suite bytes; a zero process status alone is insufficient.

## Tests the story ships

- Admit the original synthesized suite bytes, then call Runner::run_admitted.
  Use the released ESS library's explicit report/2 and detailed run/2 production;
  the legacy Runner::run path is not valid for the current suite version.
- Compare the committed suite byte-for-byte with fresh synthesis from checked-in
  ESS using the declared coverage option. Report actual selected/answered/terminal
  counts and scenario names, not only a test-target pass or a compiler inventory.
- Run the full generated obligation set. Authored scenarios extend it; they never
  remove generated scenarios or turn a failed obligation into outside coverage.
- The target opens the real kernel authority. SeedDocumentPath and
  TransactionDocumentPath resolve exact scenario-owned fixture files, and the
  bounded shared proposal reader consumes their bytes. No hash fabrication or
  JSON transaction round trip substitutes for those declared inputs.
- Establish real preconditions for external controls: changed retained seed,
  invalid seed, malformed proposal, semantically invalid proposal, intervening
  canonical commit, absent transaction/revision/assertion. Return actual observed
  outcomes and state, never the outcome selected by a control flag.
- Generated Validate input against=1 remains revision 1. Scenario setup must
  create that actual revision through legitimate handlers rather than rewrite
  the requested basis to revision 0. Fresh isolated stores prevent state leakage.
- Observe persisted/read-back transaction rows through every retained state and
  command publications from real records. Preserve actual occurrence identities;
  a retained retry emits no new event. Read-only result events use actual query
  results and never invent durable store events.
- Project returned records losslessly into ESS Node values. Native Integer
  values use exact integer constructors; no intermediate f64. Preserve Bytes,
  timestamps, optional fields, union tags, nested values and ordered collections.
- Mutation controls make named scenarios fail when kernel state transitions,
  retained responses or refusal behavior are broken. Restore the exact source
  before rerunning. Adapter bookkeeping alone cannot satisfy those assertions.
- Commit a count baseline and gate the structured report: answered floor,
  unavailable ceiling, complete total and no failures. P1 admits no quarantine.
  Record the coverage skill's repeated-run and CI-job verification before
  claiming stable counts.

## Scope

- crates/ekr/src/conformance.rs: target over actual public kernel handlers/views.
- crates/ekr/tests/conformance.rs: the real runner, drift and mutation controls.
- systems/ekr/conformance/suite.json: complete synthesized suite and provenance.
- systems/ekr/conformance/: authored scenarios and fixture manifest if needed.
- Taskfile.yml and the existing correctness workflow: conform-check in task check.
- Cargo.toml, Cargo.lock and crates/ekr/Cargo.toml: explicitly pinned ESS library
  dependencies for the target/runner, coordinator-owned and not yet added.
- systems/ekr/domains/kernel.yaml and components.yaml: only measured contract
  corrections, synchronized before dependent compilation; no weakening to pass.

## Measured readiness

The latest activated read-contract qualification ran the released ESS binary:
`ess specify validate --path systems/ekr`, `ess specify compile --path systems/ekr`,
and `ess conform synthesize --path systems/ekr --component ekr-kernel
--suite-format 5 --target ir`. This is compiler evidence, not executed scenarios.
The following inventory is extracted from those exact suite bytes when this
body is written:

- suite format: ess-conformance/13
- selected scenarios: 35
- generated: 35; authored: 0; refused: 0; outside: 0

## Delivery

Depends on story:ekr-cli and stays draft until its real target is ready. The
ESS release source used for the retained-result compiler was independently
qualified in the release/adoption evidence; pin that actual source before adding
library dependencies. There is no EKR-specific target built into the ESS binary,
so the former ess verify conform run acceptance command was invalid and is
replaced by the Rust runner above.

Record the exact passed report as evidence on executable-system-specification:ekr-v1
and let its lifecycle decide the conforming move. Do not claim conformance from
synthesis, an empty suite, or a substitute target. The ess-specify:coverage skill
supplies the executed-coverage and mutation requirements.
