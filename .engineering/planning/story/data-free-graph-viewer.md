---
format: aep.planning-md/3
id: story:data-free-graph-viewer
kind: story
status: active
title: The embedded graph viewer names no type, edge type, property or entity
relations:
- serves: vision:o5
- depends_on: story:ekr-view-server
- decomposes: epic:p4-operator-surface
scope:
- confidence: inferred
  path: crates/ekr/src/cli
- confidence: inferred
  path: crates/ekr/tests
- confidence: inferred
  path: docs/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-27T18:34:03Z", actor: "human:timo", revision: 5, imported: true}
- {from: "proposed", to: "active", at: "2026-09-27T18:34:19Z", actor: "human:timo", revision: 6, imported: true}
---
## Context

Operator request (2026-09-26): the graph viewer is **data-free**. No node type, edge type, property or entity name is written into it. The roles a reader needs to lay the graph out (which types act as events, which as subjects, which as observation types) are derived from the store's shape, so the same viewer reads a store with any ontology. The prototype this replaces had such names written into its page.

Design sections served: § 82 (every fact shown carries the assertion id it came from, so a reader can reach § 62), § 11–12 and § 95 (the ontology is data and grows by transaction, so a viewer that knows type names breaks on the next schema version), invariant 8 in spirit (domain concepts are graph state, never code).

## What to build

- The full viewer page embedded by `ekr view` (replacing the minimal page of `story:ekr-view-server`): nodes and edges of `ekr.graph-projection/1`, each node and edge with its properties and assertions (id, assessment, lifecycle, valid and recorded time, evidence ids), relation assertions drawn as links distinct from Edges, the schema history (versions with what each added, revisions), and an evidence panel that fetches source bytes from the source endpoint and shows them as text (`textContent`, never `innerHTML`: record text is untrusted evidence, A14).
- **Role derivation in Rust**, in the `ekr` crate's view module, served to the page at its own endpoint and **not** part of `ekr.graph-projection/1`. Its rule is written down in `docs/cli.md` and in the module's docs, and uses only structure: the ontology's edge-type source and target types, which types carry valid-time assertions, degree and direction. It never compares a name or an id to a constant. A type the rule cannot place gets no role and is still shown.
- Every label the page shows comes from the projection (`canonical_name`, type, edge-type and property names in `ontology`). The page carries only generic words (for example "node", "edge", "assertion", "evidence", "revision").

## Tests

Two fixture stores in the runtime's own vocabulary (no customer or personal data, per `AGENTS.md`) with **different ontologies**: different type, edge-type and property names and a different shape. Then:

1. The embedded page asset contains no name from either store's ontology or graph state.
2. Role derivation gives each store the role assignment its fixture documents, by type id.
3. A copy of one store with every type, edge-type, property and entity name replaced gets the same role assignment, id for id.
4. `ekr view` serves both stores and each page load returns that store's projection and roles.

## Domain relations

- `EdgeType → NodeType` (allowed source and target types), which the role rule reads — `systems/ekr/domains/ontology.yaml` declares these as fields `source_types` and `target_types` of `ekr.ontology.EdgeType`, not as a `relations:` entry; inferable (**inferred** from the edge-type declaration read by `crates/ekr-kernel/src/validate/types.rs:234`, which checks a relation's endpoints against `declared.source_types` and `declared.target_types`).
- `Assertion → subject` and `Assertion → object Node` — inferable (**inferred** from `crates/ekr-graph/src/assertion.rs:39` and `:126`; no ess/1 relation declares them).
- The rest: `story:graph-projection-specification` § Domain relations.

Not assumed: `decision-blocker:relation-assertion-edge-correspondence`. The page does not merge a relation assertion into an Edge.

## Out of scope

Editing, answering attention items, any write, layouts beyond what the roles need.

## Acceptance

A test holds that the embedded viewer asset contains none of the names of two fixture stores with different ontologies, that role derivation assigns each store its documented roles by type id, and that renaming every name in one store leaves its role assignment unchanged.


## State, 2026-09-28 (held off-side while the store was stopped, written with AEP 0.62.0)

- Waves p2p3p4-04 (this story, units R and P) and -05 are merged on the integration branch `wave/p2p3p4-04`, local and unpushed; the operator holds landing.
- Gate at `cfe9dcd6`: fmt, clippy, doc, vendor, spec, conform-fresh and plan-check exit 0; the test step exited 201 with 3 failures in `adversary_p1_14_conformance`, which pass alone (7 of 7, 315 s). The operator stopped the rerun and all gates, so there is no green gate and no test_result.
- Operator-approved plan (`jazzy-cooking-sonnet`): the page is rebuilt on the operator's prototype viewer (commit `d50654e1`, earlier page at `/alt`); `ekr.views` gains ProjectOverview, ExpandNeighbourhood, DescribeNode and SearchNodes (`a63f474f`); layout roles follow the valid-time rule, and the structural rule of unit R is a tooltip badge. The engine, the streamed endpoints and the page data layer follow.
