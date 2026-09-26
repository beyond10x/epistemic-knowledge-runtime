---
format: aep.planning-md/2
id: review-result:independent-review-p1-core
kind: review-result
status: active
title: Independent review, the merged P1 core, 56abc2b..f293b4d
relations:
- reviews: epic:p1-kernel-ontology-core
revision: 1
---
## Pass

Independent review of the merged P1 core, `git diff 56abc2b..f293b4d` — 141 files, ~27.9k
insertions, the three waves that built the ontology, the graph, the store and the transaction
kernel. Run on Fable, 520k tokens, 83 tool uses, in its own worktree at `f293b4d`.

It was pointed at the gap nobody else covered: every implementor line in those waves had two
adversary passes, and **nothing the coordinator wrote had any review** — six decision records, every
`systems/` document, `AGENTS.md`, the acceptance statements restated mid-wave, and eighteen tasks.

337 cases before, 346 executed, **9 red** across six new test files it wrote. `fmt`, `clippy`, `doc`
and `spec` green with its files in the tree; it skipped `plan-check` because that step is an
`aep plan artifact` verb its charter forbids, and said so.

Owners: 13 findings, 11 coordinator, 2 implementor.

## Why it is recorded after the fact

It ran outside a wave, so no closing commit carried it and it sat only in a task notification. Wave
p1-06 opened on two of its blockers with this record still missing. It was found by the coordinator's
own check 6 in `.engineering/waves/COORDINATOR.md`, while counting what the store could attribute —
the answer was nothing, because this was not in it. A review nobody can list afterwards is
indistinguishable from one that never ran.

## What it could not fault

Invariant 1's first half: private fields, a crate-visible constructor, no `Deserialize`, both
expectations naming their errors. The sealed marker traits and all four of their expectation files.
The admissible-value type: ten kinds, no float at any depth, serde through the conversion, the
conversion total and path-naming. Every kernel validator's refusal, the fold's sequencing, the object
store's content identity and retention ladder. ADRs 0004, 0005 and 0006 against the manifests. Every
strict text form. All 337 pre-existing cases.

## Findings

Each message opens with the owner, because the findings schema has no key for it.

```findings
- file: crates/ekr-store/src/log.rs
  line: 142
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "coordinator — the public append takes a bare RevisionEvent and the fold marks a transaction validated because a TransactionValidated event was appended, so a commit lands in a process that cannot link ekr-kernel and holds no ValidatedTransaction; the second half of invariant 1 is carried by nothing while its first half is sound"
- file: AGENTS.md
  line: 55
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "coordinator — the sealed reference machinery is sound and no field of Node, Edge, Assertion or CanonicalGraph has that type, so canonical state references by bare ids and a canonical edge into a transient root compiles, constructs and is refused by the kernel rather than being unrepresentable"
- file: crates/ekr-store/src/snapshot.rs
  line: 51
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the crossing defers reference, type and ontology checks to a kernel that validates only transactions, so the seed path runs no validator and produces canonical state holding a dangling edge, an undeclared type or an assertion accepted by nobody"
- file: crates/ekr-kernel/src/transaction.rs
  line: 222
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "implementor — the validation hash is a payload address over canonical bytes, the composition ekr-core's contract forbids, so a payload with those bytes takes a validation's address through the public object store"
- file: crates/ekr-store/src/log.rs
  line: 409
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "implementor — the fold discards the revision a transaction was validated against, so a commit the design calls stale replays as valid"
- file: systems/ekr/domains/kernel.yaml
  line: 155
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the domain's Revision entity and the Rust Root describe one record and disagree on four fields; Root follows design section 34 and the domain does not, and no projection guard reads this document"
- file: AGENTS.md
  line: 85
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "coordinator — the claim that two trees do not clobber their artifacts is false, measured by a two-crate probe: one binary serves both checkouts and the second ran the first's in 0.01s, which is the mechanism behind the stale path"
- file: .engineering/planning/task/store-snapshot-and-its-id-are-declared-not-implemented.md
  line: 30
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the task says the public-surface guard would have caught an untested type, and that guard reads only function and constant declarations, as the coordinator's own other task records"
- file: .engineering/planning/task/graph-domain-carries-validation-state-payloads.md
  line: 12
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the task describes a document comment already rewritten to say the opposite, so its third option was taken without the decision record the task requires"
- file: .engineering/planning/architecture-decision-record/0004-timestamp-in-ekr-core.md
  line: 1
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the record's premise that no time crate is available is contradicted by ADR 0006's amendment adding one, and ADR 0004 was not amended"
- file: .engineering/planning/task/canonical-value-in-the-graph.md
  line: 21
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the implemented task's acceptance says a float cannot inhabit an assertion or a property, and after ADR 0005's amendment the transient instantiations of both hold floats by design"
- file: crates/ekr-store/src/log.rs
  line: 365
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "implementor — the seed root's transaction field is the payload hash of a serialised document and every later root chains to it, so the lineage's addresses rest on a serialisation, which the hash contract says an address never does"
- file: .engineering/planning/task/the-membrane-stops-at-the-store-boundary.md
  line: 1
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "coordinator — the task declares it blocks a story that is merged and whose unit closed the premise its own correction describes, and stays draft"
```
