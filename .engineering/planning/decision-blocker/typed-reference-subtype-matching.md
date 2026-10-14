---
format: aep.planning-md/3
id: decision-blocker:typed-reference-subtype-matching
kind: decision-blocker
status: cleared
title: Nobody has decided whether a typed reference to a type matches nodes of its subtypes
relations:
- blocks: epic:p3-incubation-integration
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T07:29:18Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## Question

A typed reference names one node type. When that type has declared descendants, or is abstract, does the reference match nodes whose type specialises it, and what does "propose a new node" mean for an abstract type?

## The relation

`TypedReference → NodeType → descendant NodeType`, many-to-many through `parents`; no ownership. **requires-stakeholder-input**: `systems/ekr/domains/ontology.yaml` `ekr.ontology.NodeType` declares `parents: List<TypeId>` and `abstract_type: Boolean`, and says nothing about identity across the hierarchy. The code conforms subtypes to ancestors for type checks (inferred from `crates/ekr-ontology/src/schema.rs:345`, `Ontology::conforms_to`); that is one implementation answer for type checking, not a decision about identity.

## Why it matters

Exact-type matching proposes a duplicate when the same entity already exists under a subtype, which is the defect the resolver exists to stop. Subtype matching widens the candidate set, and with it the risk of a wrong merge (design § 74.4).

## What it stops

Resolution for a reference whose type is abstract or has descendants. `story:typed-reference-resolver` refuses such a reference with a named refusal instead of choosing.

## Answer

Decided 2026-10-14: a typed reference to a type matches nodes of that type and of every descendant (`Ontology::conforms_to`). Several candidates is a named ambiguity, never a choice. A reference to an abstract type never proposes a new node; it refuses by name. This is option B.
