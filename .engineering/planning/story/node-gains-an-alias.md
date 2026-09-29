---
format: aep.planning-md/3
id: story:node-gains-an-alias
kind: story
status: active
title: A transaction adds an alias to an existing node
relations:
- serves: vision:o5
- decomposes: epic:p3-incubation-integration
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: inferred
  path: systems/ekr/domains/graph.yaml
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:14:19Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-29T15:14:19Z", actor: "human:timo", revision: 7}
---
## Context

A consumer instance (2026-09-29) keys imported nodes by an alias (for example
`gitlab-repos:<project id>`) so later imports find them with `ekr resolve`. A node created earlier by
another path is found and linked, but it can never gain that alias: aliases are set only in
`CreateNode` (`crates/ekr-kernel/src/transaction.rs:86`), and no operation adds one afterwards
(0.0.17). Every later import then has to find the node by a property instead of by its alias.

## Build

Spec first, in `systems/ekr/domains/graph.yaml` and `kernel.yaml`: an `AddAlias {node, alias}`
operation in `ekr.transaction-document/2`. Validation holds it to the rules a `CreateNode`'s aliases
are held to: the node exists, the alias is well formed, and it is unique among nodes of the node's
type (`alias-already-exists`, `duplicate-alias`). Replay applies it; the per-head alias index
(`ekr_graph::AliasIndex`) and `resolve` see it at the next head. Removing an alias is out of scope.

## Acceptance

- A node created without an alias gains one in a later transaction, and `ekr resolve` on that alias
  then answers the node, on both providers.
- An alias another node of the type holds, the same alias twice in one transaction, and an unknown
  node are refused by name.
- Replay reproduces the roots; the kernel conformance suite gains the scenarios.
