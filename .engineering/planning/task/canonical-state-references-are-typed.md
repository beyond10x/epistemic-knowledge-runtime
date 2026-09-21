---
format: aep.planning-md/1
id: task:canonical-state-references-are-typed
kind: task
status: active
title: Canonical state references by bare ids, so the membrane is refused rather than unrepresentable
relations:
- serves: vision:o2
revision: 3
---
## What this implements

`architecture-decision-record:0008-canonical-state-references-are-typed`. Carried as a task for the
same reason as its sibling: it repairs an invariant the merged core claims and does not hold.

## Acceptance

**A canonical edge whose target is a candidate's id does not compile.**

The independent reviewer's `trybuild` case
`crates/ekr-graph/tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs`
fails today *because the code compiles*. It goes green — meaning the case compile-fails as intended.

## Scope

- `crates/ekr-graph/src/edge.rs` — `source` and `target`
- `crates/ekr-graph/src/assertion.rs` — `Subject::Node`, `Object::Node`
- `crates/ekr-graph/src/value.rs` — `CanonicalValue::NodeRef`
- `crates/ekr-graph/src/canonical.rs` — the sealed machinery gains the callers it was built for
- every fixture that builds canonical state by hand, principally
  `crates/ekr-store/tests/fixture/mod.rs`
- `AGENTS.md` — invariant 2 stays as written, because after this it is true

## Tests

- the reviewer's `trybuild` case, compile-failing
- the four existing membrane cases still name the errors they exist for
- the serde boundary stated rather than hidden: a document deserialises into whatever type the
  caller names, so the kernel's reference validator keeps its refusal for ids arriving from outside
  Rust

## Notes

`crates/ekr-store/tests/fixture/mod.rs` currently seeds canonical state with a **dangling edge
target** — the edge's `target` is not in the `nodes` map. That fixture stops compiling under this
change, which is the point: the reviewer found the defect by reading and the type finds it by
building.

The transient instantiation is unaffected. `Node<Value>`, `Edge<Value>` and `Assertion<Value>` keep
whatever they keep, per ADR 0005's amendment.
