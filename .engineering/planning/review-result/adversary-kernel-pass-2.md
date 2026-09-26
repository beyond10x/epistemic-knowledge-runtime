---
format: aep.planning-md/2
id: review-result:adversary-kernel-pass-2
kind: review-result
status: active
title: Adversary, story:transaction-and-validators, pass 2
relations:
- reviews: story:transaction-and-validators
revision: 1
---
## Pass

Adversary pass 2 over `story:transaction-and-validators`, wave p1-05 unit 2, against correction round
1, uncommitted at base `f981a07`. Opus, 164k tokens, 50 tool uses, 4m48s.

272 cases before, 276 executed, 4 red in one new file. Three blockers. All seven pass-1 findings
verified resolved rather than taken on trust.

**Two of them resolved and each left a sibling standing.** The widened identity class stopped at
three where it is five, and the same enumeration that gained `GraphRootId` still excuses `TypeId` to
a validator that does not check it.

## What it could not break

The encoding's field order across thirty-four fields of seven types, by a probe it wrote itself. The
evidence manifest against three legitimate shapes. The ungated conversion: it cannot panic, it is
linear, and the two walks are pinned equal by a property rather than by a branch. The order-free edge
count in five orderings. The encoder's framing against an ambiguous boundary. All 272.

## The claim in the correction brief that it disproved

The correction brief accepted the implementor's statement that pinning a whole field layout needs a
stored byte vector over fixed identifiers, and a dependency the story forbids. That is false. Hold
every other field constant, vary one, and the offset of the first differing byte is where that field
is written; the offsets must ascend in declaration order. The reviewer wrote it, ran it over seven
types and thirty-four fields including every one the stated bound named as unreachable, and it
passes. About sixty lines, no dependency.

This is the second claim in that brief the reviewers have measured and refused. The first was that a
hash-separation property covers field order, which the implementor disproved.

## Findings

```findings
- file: crates/ekr-kernel/src/validate/structural.rs
  line: 112
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the widened class of operations that bring an identity into existence stops at three where it is five, so a DefineNodeType or DefineEdgeType over a TypeId the ontology already declares passes all seven validators and redeclares a committed type through the declare path"
- file: crates/ekr-kernel/src/transaction.rs
  line: 610
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "nothing reads the ValidationState an AddAssertion arrives carrying, so a proposer seals an assertion already marked Accepted by validators that never saw it, which is the state ekr-graph documents as having crossed the integrity boundary this kernel is"
- file: crates/ekr-kernel/src/validate/types.rs
  line: 206
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the reference validator excuses TypeId by naming the type validator as the one that refuses it, and the type validator leaves the assertion arm unless the object is a Value, so an assertion naming an undeclared TypeId is refused by nobody and one undeclared property is refused against a value object and accepted against a node object"
- file: crates/ekr-kernel/src/validate/reference.rs
  line: 139
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a MergeEntity whose absorbed and into are one node validates, naming one id as both the record that stops being its own entity and the record that survives, and the property generator draws the two independently so it produces this shape about one proposal in three"
- file: crates/ekr-kernel/tests/validation.rs
  line: 1238
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the field-order guard states that pinning the whole layout needs a stored byte vector and a forbidden dependency, which is false: varying one field at a time and asserting the first-differing-byte offsets ascend pins all thirty-four fields of all seven encoded types with no dependency, and it passes"
- file: crates/ekr-kernel/src/validate/authorization.rs
  line: 55
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "the separation of proposer and validator is checked between an actor the pipeline supplies and a proposer the proposal supplies, so an agent naming a different proposer validates its own transaction; the kernel receives no submitter identity and cannot fix this, and the doc does not say the obligation passes to the submission story"
- file: crates/ekr-kernel/src/transaction.rs
  line: 276
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the encoder places the obligation to pin a variant numbering on the implementor and names the graph crate's case as the precedent, and nothing pins the eleven operation numbers against the domain's operation-kind list"
```
