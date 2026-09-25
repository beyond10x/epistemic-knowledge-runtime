---
format: aep.planning-md/1
id: story:ekr-cli
kind: story
status: implemented
title: 'The ekr binary: seed, propose, validate, commit, snapshot, explain'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:seed-and-explain
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: inferred
  path: crates/ekr/src/cli
- confidence: inferred
  path: crates/ekr/src/exit.rs
- confidence: inferred
  path: crates/ekr/src/host.rs
- confidence: inferred
  path: crates/ekr/src/lib.rs
- confidence: inferred
  path: crates/ekr/src/main.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures
- confidence: inferred
  path: crates/ekr/tests/retraction_example.rs
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 16
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

DESIGN 93 and active kernel ESS bind actual Propose, Validate and Commit records.
The reconciled Snapshot/Explain and host declarations from
`.engineering/waves/p1-cli-explain-contract-r2.md` are now active in every
participating tree. The original adjacent patch is historical evidence, not a
pending patch to apply. Released ESS validation/compilation/synthesis pass; no
command-runtime success follows from those specification checks.

The exact JSON host envelope and valid-time parser are implemented and independently
reviewed in `.engineering/reviews/p1-cli-host-input-review.md`. Reuse that actual
library. The bounded review found no defects and reports its executed cases;
the full command story remains unexecuted and open.

The trusted operator selects tenant, BootstrapContext and AuthorityStateV1.
Kernel Runtime opens the actual provider and derives ontology from the retained
seed. Preserve Seed's typed anchor mismatch, lazy time, exact bounded proposal
reader and outcome/refusal/fault distinctions. No ekr-store dependency is allowed.

After the coherent writer checkpoint, the same source owner implements the
kernel read projections first and thin CLI dispatch second. Its new binary tests
and fixtures are exclusive; all existing legacy/current serialization fixture
migration remains with the writer. Kernel src/explain.rs plus only its lib.rs
export lines are the explicit shared-file partition recorded on the Explain
story. The writer remains incomplete pending fault controls and the coupled gate;
this source overlap neither closes a dependency nor claims an independently
ready wave. Final integration uses the actual shared runtime and no replacement.

The coordinator owns shared specifications/manifests. CLI already declares
workspace serde/time; no package pin changes or new dependency are authorized.

## Completion

Executed in wave p1-13 (2026-09-23): `crates/ekr/tests/retraction_example.rs` (11 cases) runs the
design § 65 example through fresh binary processes on both providers; adversary files
`adversary_p1_13_cli_*.rs` add 17. Gate on `wave/wave-p1-13` @ `8fe1b04`: 730 passed. Earlier
sentences in this body that call the acceptance unexecuted describe the state before that wave.
