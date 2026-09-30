---
format: aep.planning-md/3
id: review-result:adversary-reads-04-l-pass-1
kind: review-result
status: active
title: Adversary, wave reads-04 unit L, pass 1
relations:
- reviews: task:lineage-shows-widened-ends-and-modified-properties
revision: 1
---
## Verdict

CONFIRMED on `2d015b1a`: one version carrying every schema operation lists each change once against
an independent ontology diff, on both providers (the adversary's case in `41bc2848`). Two findings:
the unit's own cases never exercised ValueType, Required, Constraints or edge-type properties (now
covered), and a version whose new id reuses a UUID another id kind holds lists nothing, filed as
`task:lineage-keys-members-by-kind` since nothing found reaches it. Names render as text nodes;
byte identity holds for stores without such versions.

```findings
- file: crates/ekr-views/src/index.rs
  line: 743
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: "a version adding a property or node type whose UUID another id kind uses lists nothing, and the page calls a changed version empty"
- file: crates/ekr-views/tests/lineage.rs
  line: 40
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the unit's cases never exercised ValueType, Required, Constraints or edge-type-owned property changes"
```
