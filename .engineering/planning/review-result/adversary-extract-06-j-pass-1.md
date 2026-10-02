---
format: aep.planning-md/3
id: review-result:adversary-extract-06-j-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit J, pass 1
relations:
- reviews: task:projection-carries-per-type-property-definitions
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit J (`impl/per-type-property-definitions` at `0e4143e0`; cases committed as `9e8b6cd7`, three red and ignored). `owners` lists only the types that declare a property and the projection carries no parents, so for a subtype that inherits a redeclared property the `views.yaml` reader rule names no definition and the viewer labels the value with another type's definition (Properties, Stated with evidence, timeline). The schema history names a version's added property by the shown revision's definition. No shipped seed has this shape; base refused the whole revision.

Held: the diamond, name-only, kind-only and required-only changes on an abstract parent, ordering with an edge type mixed in, redeclare-then-unify with overview equal to projection; no SDK view type denies unknown fields at base or 0.0.24.

```findings
- file: crates/ekr/src/cli/viewer/index.html
  line: 456
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a node of a subtype that inherits a redeclared property is labelled with the id's first entry, another type's definition, in the Properties, Stated with evidence and timeline panels
- file: crates/ekr-views/src/document.rs
  line: 632
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: owners lists only declaring types and the projection carries no parents, so the views.yaml reader rule names no definition for an inheriting type
- file: crates/ekr/src/cli/viewer/index.html
  line: 2672
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the schema history names a version's added property by the shown revision's definition instead of the name that version gave it
```
