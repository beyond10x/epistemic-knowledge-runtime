---
format: aep.planning-md/3
id: decision-blocker:constraint-language
kind: decision-blocker
status: cleared
title: Nobody has decided which constraint language a property's constraints hold
relations:
- blocks: story:property-constraints-are-enforced
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T07:29:19Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## Question

Which constraint language does `PropertyDefinition::constraints` hold, so validator 5 can evaluate a
declared constraint instead of refusing every write that touches it?

## Why nobody can read the answer

Design § 11.2 names `constraints: Vec<Constraint>` without defining `Constraint`;
`systems/ekr/domains/ontology.yaml:65,369` marks the language `UNMAPPED`; the kernel refuses every
write touching a declared constraint (`crates/ekr-kernel/src/validate/ontology.rs:88-110`,
`unsupported-constraint`). The consumer request the story cites (2026-09-28) is not in the store.

## Options seen

1. A closed set of typed constraints (range, length, pattern, enumeration, uniqueness within a type),
   parsed from the existing text at evaluation time so the canonical schema encoding is unchanged.
2. A small expression language over one value (and its node), evaluated deterministically without a
   model (invariant 7).
3. Constraints stay opaque; the story is rejected and the refusal documented as permanent.

## What it stops

`story:property-constraints-are-enforced`.

## Clears when

The operator records the answer and it lands in `ontology.yaml` before the story is scheduled.

## Answer

Decided 2026-10-14: a closed set of typed constraints (range, length, pattern, enumeration, uniqueness within a type), parsed from the existing constraint strings at evaluation time, so the canonical schema encoding is unchanged. An expression language may come later as a new constraint kind. This is option 1 (A).
