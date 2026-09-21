---
format: aep.planning-md/1
id: architecture-decision-record:0008-canonical-state-references-are-typed
kind: architecture-decision-record
status: accepted
title: ADR 0008 — Canonical state references by a typed reference, not a bare id
relations:
- decides: task:canonical-state-references-are-typed
revision: 2
---
## Status

Proposed 2026-09-21, after an independent review of the merged P1 core found the invariant carried
by nothing in canonical state. Accepted the same day.

## What the review found

`AGENTS.md` invariant 2: *"A `Canonical → Transient` reference is unrepresentable at the type level,
not merely refused."*

The sealed `CanonicalRef` / `CanonicalTarget` machinery in `crates/ekr-graph/src/canonical.rs:139-164`
is sound. The reviewer attacked the sealing, the orphan rule and all four `.stderr` files and could
not fault them.

**It is held by nothing in canonical state.** `CanonicalGraph` references by bare ids:
`Edge.source` and `Edge.target` are `NodeId` (`edge.rs:31-33`), `Subject::Node` and `Object::Node`
are `NodeId` (`assertion.rs:19,88`), `CanonicalValue::NodeRef` is a `NodeId` (`value.rs:57`).

So a canonical edge whose target is a candidate's id **compiles, constructs, and is refused by the
kernel** with `unresolved-node` — which is precisely what the sentence says it is not. The four
compile-fail cases prove a property of types that canonical state does not use.

## Decision

**Canonical state references by a typed reference, not by a bare id.**

`Edge.source`, `Edge.target`, `Subject::Node`, `Object::Node` and `CanonicalValue::NodeRef` take
`CanonicalRef<Node>` where the value parameter is canonical, and the transient instantiation keeps
whatever it keeps. The machinery already exists and is already sealed; this decision gives it the
callers it was built for.

## Why not the cheaper option

The alternative is to amend the invariant to say a `Canonical → Transient` reference is *refused at
validation*, and keep the sealed machinery for the day something uses it.

Rejected. The runtime's first phase has three exit criteria and this invariant is one of them
(`docs/roadmap.md:144-146`). Meeting it with a run-time check while the operating document claims a
type is the shape of defect this phase has now found four times in its own documents: a sentence
asserting agreement between a document and code that nothing executes. Taking the retreat would add
a fifth, in the document that states the rules.

It is also cheaper now than at any later point. Nothing is persisted, so no stored reference has to
migrate; `story:commit-and-revision-lineage` writes the first lineage anybody keeps and it is the
next wave but one.

## What this costs

`crates/ekr-graph/src/{edge,assertion,value,canonical}.rs` and every fixture that builds canonical
state by hand — principally `crates/ekr-store/tests/fixture/mod.rs`, which the same review found
seeding a dangling edge target. `ekr-kernel`'s reference validator keeps its refusal for the ids that
arrive from outside Rust, through serde, where no type can help.

The serde boundary is the honest limit and it is stated rather than hidden: a document deserialises
into whatever type the caller names, so the guarantee holds inside Rust and the kernel's refusal is
what holds at the edge. `task:the-membrane-stops-at-the-store-boundary` already records that for the
value direction.

## The exit criterion

The reviewer's `crates/ekr-graph/tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs`
is a `trybuild` case that **fails today because the code compiles**. The wave is done when it
compiles-fails as intended.
