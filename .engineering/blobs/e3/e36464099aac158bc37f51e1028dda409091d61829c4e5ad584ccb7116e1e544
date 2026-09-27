---
format: aep.planning-md/1
id: review-result:adversary-canonical-value-pass-1
kind: review-result
status: active
title: Adversary, task:canonical-value-in-the-graph, pass 1
relations:
- reviews: task:canonical-value-in-the-graph
revision: 1
---
## Pass

Adversary pass 1 over `task:canonical-value-in-the-graph`, wave p1-05 unit 0, against the
uncommitted unit work in worktree `ekr-impl-canonical-value` at base `967d279`. Opus, 166k tokens,
51 tool uses, 6m19s.

216 cases before, 219 executed, 3 red in one new file,
`crates/ekr-graph/tests/adversary_canonical_value_reach.rs`. `task check` exit 201 at the `test`
step; `fmt-check` and `clippy` pass.

Two blockers. One is a design finding about the membrane and one is a mutation the whole suite
misses.

## What it could not break

`TryFrom<Value>` exhaustiveness and recursion, including empty containers, an empty field name and
ten-deep alternating nesting. `From<CanonicalValue> for Value` as an exact inverse. Every route into
the type: public variants, serde, no `Default`, no payload `From` — none can hold a float at any
depth, because the shape is the invariant. The serde path carries `try_from`, which is the hole wave
p1-04 closed on `TransactionTime`. `ValuePath`'s `Display` is injective. Prefix-freeness across the
six sum types that gained an encoding. The widened variant-order guard against doc comments with
braces, attributes, all four variant forms and a prefix-named variant. Every remaining float route
into canonical state: one `f64` exists in the whole workspace and it is the one the decision keeps.
All 216 pre-existing cases.

Two latent bounds of the variant guard that no case covers and neither is reachable today: an
`impl Canonical for X` whose `enum X` is declared in another module is read as a struct and required
to write no tag, and an unbalanced brace inside a doc comment breaks its block scanner.

## Findings

```findings
- file: crates/ekr-graph/src/transient.rs
  line: 124
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "TransientGraph is composed of Node, Edge and Assertion, so moving those three to CanonicalValue makes a Float unholdable in transient state, which ADR 0005's decision says it stays legal in"
- file: crates/ekr-graph/tests/canonical_value_and_assertion.rs
  line: 270
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "removing all eight payload-encoding lines from ValidationState::encode leaves the whole 216-case suite green at exit 0, so two Accepted, Rejected, Superseded or Retracted assertions differing only in their payload would share a content address unnoticed"
- file: crates/ekr-graph/src/root.rs
  line: 57
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "ADR 0005 decides that this unit makes Root.knowledge_root computable, and neither Node nor Edge has a Canonical implementation, so the address of graph state cannot be computed from graph state"
- file: crates/ekr-graph/tests/canonical_value_and_assertion.rs
  line: 275
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "dropping TemporalRange.to and TransactionTime.recorded_from from the new Canonical impls leaves the suite green at exit 0, because the valid_time row varies only from and the transaction_time row varies only recorded_to"
- file: crates/ekr-ontology/src/value.rs
  line: 324
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the doc names ekr_graph::CanonicalValue as one of two callers of inadmissible_in_canonical_state while crates/ekr-graph/src/value.rs:113 says there is deliberately no such call, leaving a new public item with zero callers that the public-surface guard cannot see"
- file: crates/ekr-graph/src/value.rs
  line: 19
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the module doc says the one way in is TryFrom of Value while every variant is a public constructor that the unit's own tests use at crates/ekr/tests/graph_assertion_serde.rs:79"
- file: systems/ekr/domains/graph.yaml
  line: 116
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "TypedValue.kind is ekr.ontology.ValueKind's eleven members while a canonical property carries ten, which is a documentation gap only because no Rust type binds TypedValue, and the fix is the coordinator's since systems is not the unit's to edit"
```
