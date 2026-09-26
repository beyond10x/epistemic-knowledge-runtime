---
format: aep.planning-md/2
id: review-result:adversary-canonical-value-pass-2
kind: review-result
status: active
title: Adversary, task:canonical-value-in-the-graph, pass 2
relations:
- reviews: task:canonical-value-in-the-graph
revision: 1
---
## Pass

Adversary pass 2 over `task:canonical-value-in-the-graph`, wave p1-05 unit 0, against the unit work
plus correction round 1, uncommitted at base `967d279`. Opus, 279k tokens, 30 tool uses, 5m05s.

223 cases before, 225 executed, 2 red in one new file,
`crates/ekr-graph/tests/adversary2_membrane_and_addresses.rs`. `task check` exit 201 at `test`.

Five of pass 1's seven resolved. Finding 2 resolved for graph state and reappears for evidence
state. Finding 7 carried unchanged and is the coordinator's.

## The judgements the brief asked for

**The two rewritten cases were redirected and are strictly stronger.** The claim is the same claim,
now carried by the generic, and each case gained the negation of what it used to assert. Under the
amendment the original body asserted something the decision forbids, so a rewrite was unavoidable,
and it was declared.

**`Canonical for Node` and `Canonical for Edge` with no caller are defensible.** They are not the
class wave p1-04 deleted twice: that was a method kept alive so a guard would not warn. These have
three cases exercising their behaviour, one pinning their field lists to the declaration, and a
named consumer in unit 1.

## What it could not break

The `Canonical` bound is load-bearing: no blanket impl, one address constructor, no derived `Hash`,
and the orphan rule stops any crate above implementing `Canonical` for `Value`. The fourth
`trybuild` case covers all three types and its `.stderr` names its own error; the other three still
name theirs. The generic gives no route around the membrane — the marker macro expands at the
default parameter, so a reference to a transient-valued node is not a type. No conversion between
the two instantiations in Rust. The scanner helpers on lifetimes, const generics, nested angle
brackets and defaulted parameters. The numbering guard under the generic, which panics loudly
rather than skipping. Both mutation tables against every single-field drop, including three id
newtypes mutated to one value, which rule 5 makes encode identically and position separates.

**One thing it found and could not turn into a case.** `CanonicalValue` serialises through `Value`,
so the two instantiations have byte-identical documents and nothing on a stored node records which
space it came from. A candidate's document deserialises into a canonical node whenever it carries no
float. The guarantee holds inside Rust and stops at the store boundary. The reviewer declined to
write the case because asserting the refusal would invent a requirement nobody has stated.

## Findings

```findings
- file: crates/ekr-graph/tests/canonical_value_and_assertion.rs
  line: 261
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "dropping the payloads of Subject::Edge, Subject::Type, Predicate::Relation, Object::Node, Object::Type and ValidationState Validating.completed leaves all 223 cases green at exit 0, because a variant's payload is pinned only where that variant appears twice with different payloads"
- file: crates/ekr-graph/src/root.rs
  line: 59
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Evidence, Observation and Support have no Canonical implementation, so Root.evidence_root is not computable from the fourth map CanonicalGraph holds, and the amendment's claim that the address exists exactly where canonical state does is false in the direction no case holds"
- file: crates/ekr-graph/src/transient.rs
  line: 67
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "LocalRef carries the seal on CanonicalDependency without implementing the trait, so the pinned stderr lists it to a reader as an implementor and nothing inside this crate stops the impl being written"
- file: crates/ekr-graph/src/transient.rs
  line: 186
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Resolved::id erases which side of the membrane a node came from, which is the one distinction TransientRef's PartialEq exists to preserve, and nothing asserts the variant because the public-surface guard counts the accessor as used by any field access named id"
- file: crates/ekr-graph/tests/domain_projection.rs
  line: 373
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a where clause makes the item scanner miss the item, which is a third bound beside the two its doc states, and the two byte-identical copies of the helper have nothing asserting they stay identical"
- file: crates/ekr-graph/tests/canonical_value_and_assertion.rs
  line: 428
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the pairwise case panics at the first colliding pair, so a mutation measurement made with it reports one red however many fields broke"
```
