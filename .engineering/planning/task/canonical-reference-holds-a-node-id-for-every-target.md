---
format: aep.planning-md/1
id: task:canonical-reference-holds-a-node-id-for-every-target
kind: task
status: implemented
title: A canonical reference to evidence resolves to a node, because the marker is decoration
relations:
- serves: vision:o2
- decomposes: story:p1-exit-properties
revision: 5
---
## What is wrong

`crates/ekr-graph/src/canonical.rs` declares `CanonicalRef<T>` over a `CanonicalTarget` marker, and
the doc at `:68-71` says the marker's job is "to keep a reference to one kind of thing out of a slot
that wants another".

**It stores a `NodeId` for all seven admitted types.** So `CanonicalGraph::resolve` on a
`CanonicalRef<Evidence>` looks in the `nodes` map and answers a `Node`. Six of the seven markers are
wrong if anybody uses them; exactly one, `Node`, has a caller.

Measured by the adversary of wave p1-06: a reference to evidence resolved to a node with a named id.

## Why it is newly worth closing

Wave p1-06 made `CanonicalTarget` the bound on five new implementations — `Serialize`,
`Deserialize`, `Canonical`, `Ord` and `Hash` — so the marker now carries far more than it did when
it had one caller and a doc comment.

`Subject::Edge(EdgeId)` sits beside `Subject::Node(CanonicalRef<Node>)` in one enum, with
`CanonicalRef<Edge>` available and unusable. The next reader who reaches for it gets a node.

## What closes this

Either:

- an associated `Id` type on `CanonicalTarget`, so `CanonicalRef<Evidence>` holds an `EvidenceId`
  and `resolve` is implemented per target — the real fix, and it makes the remaining bare-id
  references in `Subject::Edge` and `Assertion.evidence` typeable; or
- narrow `canonical_target!` to the types a **node** reference can name, which is one, and rename
  the type to say so.

The first is the one that lets `AGENTS.md` invariant 2 reach the references wave p1-06 did not
convert. The second is honest and small.

## Why it is not wave p1-06's

It reproduces at that wave's base (`canonical.rs:92` is `node: NodeId`, `:174` reads
`self.nodes.get(…)`), and nothing in the tree reaches it — the adversary built the state. Closing it
is a design change to the marker rather than a repair of the two invariants that wave exists for.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Coordinator decision: the parent story requires `Subject::Edge`, `Assertion.evidence` and `CanonicalRef<Evidence>` typed, so the fix retypes them and reaches `ekr-kernel` and `ekr-store`.

- `crates/ekr-graph/src/canonical.rs:98-117` (`CanonicalTarget`), `:203-301` (`CanonicalRef<T>` stores `node: NodeId`; Serialize/Deserialize/Canonical/Ord/Hash keyed on it), `:338` (`resolve` reads `nodes`) — cited
- `crates/ekr-graph/src/transient.rs:81-97,166-180` — cited
- `crates/ekr-graph/tests/adversary_p1_06_reference_markers.rs:113-123` pins the defect and must be inverted — cited
- `crates/ekr-graph/tests/compile_fail/{canonical_ref_cannot_target_a_transient_type,canonical_graph_rejects_a_transient_ref,canonical_dependency_is_sealed}.stderr` — cited
- `Observation`, `Support`, `GraphRoot` markers have no map in `CanonicalGraph` — cited; remove or add maps
- if `Subject::Edge` is retyped: `crates/ekr-graph/src/assertion.rs:33-60`, `crates/ekr-graph/tests/domain_projection.rs:533,539`, `crates/ekr-kernel/src/{transaction.rs,seed.rs,validate/reference.rs,validate/ontology.rs,validate/types.rs}`, `crates/ekr-store/src/snapshot.rs:304-327` and their tests — inferred
- `systems/ekr/domains/graph.yaml` unchanged while references encode the bare id; `crates/ekr-graph/tests/current_vectors.rs` holds the bytes — inferred
- Confidence: high for the primary surface, medium for the kernel/store spread
