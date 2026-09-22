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
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 7
---
## Context

Design § 70 names the controlled runtime interface. P1 exposes the kernel half
as the `ekr` binary so the epic's exit evidence, the § 65 replacement example,
runs from a shell and conformance consumes the same durable handlers.

The authoritative command inputs and outcomes are in
`systems/ekr/domains/kernel.yaml`. DESIGN § 91 and the bounded transaction parser
require retaining original YAML bytes. The earlier generic JSON-on-stdin scope
was inconsistent with that contract and is superseded here. JSON output remains
a presentation of the actual result, never a replacement transaction parser.

## Acceptance

The integration test `crates/ekr/tests/retraction_example.rs` drives the § 65
example through fresh binary processes and the real kernel authority, persists
changed knowledge, reopens it, and distinguishes Alice and Bob at the requested
valid times. This acceptance is unexecuted until the writer and CLI land.

## Tests the story ships

- `seed`, `propose`, `validate`, `commit`, `snapshot`, and `explain` dispatch to
  the owning kernel handlers. No CLI-only application, substitute authority or
  persistence writer is allowed. Help exposes those P1 verbs using ESS wire names.
- Seed and proposal inputs are read as document bytes. Propose uses the shared
  bounded document parser, preserves exact accepted bytes and refuses malformed
  input before recording. `-` may select stdin; it does not change the parser.
- The fixture declares its own ontology, real trusted authority and retained
  evidence. Bootstrap/operator/validator identity and trusted time come from the
  host boundary, never a made-up registry or an allow-all test authority.
- The example seeds, proposes Alice's assertion, validates and commits, then
  proposes Bob with the explicit SupersedeAssertion operation required by
  the final lifecycle contract, validates and commits again. Snapshots at valid
  times before and after 2026-03-12 return Alice and Bob respectively. A snapshot
  at the earlier committed revision still returns its historical state.
- Exact Seed and Commit retries return their original retained results after
  restart and after later head movement. A different seed refuses. The example
  must not rely on a live in-process capability surviving between CLI commands.
- A declared ESS refusal exits 2 with its name on stderr; an operational fault
  exits 1; successful command outcomes exit 0. Validate rejection and Commit
  staleness remain their declared recorded outcomes rather than invented errors.
- Explain uses the complete kernel chain and distinguishes an unknown assertion.
  HumanStatement evidence terminates directly without an invented observation.

All cases above are planned and unexecuted here. The final fixture and operation
shape depend on the coupled durable activation unit; this story cannot close on
successful argument parsing or unchanged seed-only roots.

## Scope

- `crates/ekr/src/main.rs` — clap composition, store selection and host inputs.
- `crates/ekr/src/cli/{seed,propose,validate,commit,snapshot,explain}.rs` — thin
  handlers over the actual kernel interfaces, document input and result rendering.
- `crates/ekr/src/exit.rs` — the declared outcome/refusal/fault exit contract.
- `crates/ekr/tests/retraction_example.rs` and `crates/ekr/tests/fixtures/` — the
  fresh-process example, retained-result and named-refusal controls.

## Notes

Depends on `story:seed-and-explain` and `story:eventlog-store`, with the durable
writer dependency inherited through explain. `ingest` and `maintain` remain P2
and P6. Use the declared `ekr` dependencies; report a needed shared dependency or
specification change before implementing it. Exact host configuration transport
and concrete public handler signatures are resolved against the completed writer
before dispatch; this paragraph is not a claim that those interfaces exist.

## Command-result preparation

The cited/inferred scope report is
`.engineering/reviews/p1-cli-explain-scope.md`. The unapplied typed read-result,
valid-time selector and Commit outcome proposal is
`.engineering/waves/p1-cli-explain-contract-preparation.md` and its adjacent patch.
Released compiler qualification is recorded there; no runtime pass is inferred.
The coordinator owns activation of the shared kernel ESS, after reconciliation
with the durable unit's final facade. Host configuration/authentication transport
and CLI timestamp syntax remain explicit dispatch choices. Do not interpret the
proposed response wrappers as new persisted receipt versions.
