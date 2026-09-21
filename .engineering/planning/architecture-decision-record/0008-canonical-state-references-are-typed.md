---
format: aep.planning-md/1
id: architecture-decision-record:0008-canonical-state-references-are-typed
kind: architecture-decision-record
status: accepted
title: ADR 0008 — Canonical state references by a typed reference, not a bare id
relations:
- decides: task:canonical-state-references-are-typed
revision: 3
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

The serde boundary is the honest limit. **The sentence that first stood here was false and is
corrected**: it said the kernel's reference validator keeps the refusal for ids arriving from outside
Rust. The only path ids arrive by is the seed fold — `EventlogStore::fold_from` →
`GraphDocument::into_canonical` — and that sits **below** `ekr-kernel` and can reach no validator in
any process. The adversary of this wave put a document whose edge targets a node it does not carry
through it and got canonical state holding a reference to nothing.

What is true: the guarantee holds **inside Rust**, and on the **transaction** path the kernel's
reference validator refuses a dangling id. **The seed path refuses nothing today.** The independent
review's finding C — recorded in `review-result:independent-review-p1-core` — is that the seed path
runs no validator at all, and closing it is the debt wave's, with an ADR deciding who validates a
seed. `task:the-membrane-stops-at-the-store-boundary` records the same limit for the value direction.

A case pins this rather than a sentence:
`crates/ekr-store/tests/adversary_p1_06_reference_from_bytes.rs` asserts the crossing produces a
reference to a node the document does not carry with nothing refusing it, and goes red the day
finding C closes.

## The exit criterion

The reviewer's `crates/ekr-graph/tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs`
is a `trybuild` case that **fails today because the code compiles**. The wave is done when it
compiles-fails as intended.
