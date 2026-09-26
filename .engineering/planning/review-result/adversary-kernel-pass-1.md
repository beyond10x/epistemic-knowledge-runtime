---
format: aep.planning-md/2
id: review-result:adversary-kernel-pass-1
kind: review-result
status: active
title: Adversary, story:transaction-and-validators, pass 1
relations:
- reviews: story:transaction-and-validators
revision: 1
---
## Pass

Adversary pass 1 over `story:transaction-and-validators`, wave p1-05 unit 2, against the uncommitted
unit work in worktree `ekr-impl-kernel` at base `f981a07`. Opus, 158k tokens, 43 tool uses, 4m11s.

261 cases before, 266 executed, 5 red in one new file. Seven findings, one blocker.

The blocker defeats `AGENTS.md` invariant 4 and the fifth of the story's own five acceptance
defects: a proposer launders its own evidence by naming a phantom id twice, once on the assertion
and once in the transaction's evidence list, and the two validators that should catch it each see
the other's half.

## What it could not break

Invariant 1 as a bound rather than as its two instances: no public constructor, no `Deserialize`, no
`Default`, no `From`, no public field holding one, a private pipeline field and one constructor, and
both `.stderr` files naming the error their case exists for. Each of the five acceptance fixtures
carries exactly one defect, walked by hand against all seven validators — no repeat of wave p1-03's
two-fault document. The admissibility pass reaches every value in all eleven operation variants
including nested containers. Every public field of all eleven encoded structs reaches an encoding,
checked against the declarations rather than against the guard. `validation_hash` determinism across
two runs and two processes. Authorization's thin surface: the actor is not optional, so an absent
one is unrepresentable. All 261 previously-passing cases.

## Findings

```findings
- file: crates/ekr-kernel/src/validate/reference.rs
  line: 152
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the resolvable evidence set chains the ids the proposer listed in the transaction, which carries no Evidence and which no operation creates, so the fifth acceptance defect passes whenever the proposer names a phantom id twice, contradicting the provenance validator's own statement of the division of labour"
- file: crates/ekr-kernel/src/validate/structural.rs
  line: 47
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "duplicate identity is refused only within one transaction, so a CreateNode naming a node canonical state already holds passes all seven validators and replaces a committed node through the create path"
- file: crates/ekr-kernel/src/validate/reference.rs
  line: 167
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "an AddAssertion carrying the id of an already Accepted assertion and a different object validates, leaving AGENTS.md invariant 5 to whatever the store does with a duplicate key"
- file: crates/ekr-kernel/src/validate/reference.rs
  line: 15
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the module doc enumerates the graph identities it resolves and excuses only TypeId, but GraphRootId is resolved nowhere, so a node validates into a graph root nothing holds"
- file: crates/ekr-kernel/src/validate/cardinality.rs
  line: 173
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the edge counter replays operations in vector order while the reference validator declares the transaction is read as a set, so one operation set is Ok as create-then-delete and refused as delete-then-create"
- file: crates/ekr-kernel/src/transaction.rs
  line: 478
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "no test constructs DefineEdgeType and the property generator omits it, so the nine-field edge-type encoding and its variant tag never execute and swapping two of its fields leaves the suite green"
- file: crates/ekr-kernel/src/validate/mod.rs
  line: 147
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the conversion's error arm is unreachable now that admissibility is a separate pass over the same five value sites the conversion walks, so an integrity boundary carries a dead guard whose issue code is credited to the scan by a string raised elsewhere"
```
