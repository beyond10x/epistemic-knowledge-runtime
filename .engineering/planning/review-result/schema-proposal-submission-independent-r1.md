---
format: aep.planning-md/3
id: review-result:schema-proposal-submission-independent-r1
kind: review-result
status: active
title: Independent proposal submission review
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
needs-revision

Owners: 2 coordinator/kernel implementor findings; 0 delegated implementor findings.

Reviewed the current uncommitted kernel submission/show slice. Both findings are source-derived; I did not execute reproductions.

1. **Schema effects are missing from the material digest.** Submit a supported proposal defining `Child` with parent `Parent`, no mappings, then change an unused `Parent.p` from Integer to String through `ModifyProperty`. The proposed child now inherits a different declaration, but all three material digests remain unchanged; only `observed_revision` advances. Include the relevant effective schema declarations in deterministic review material, without temporary minted IDs. Add a regression requiring the effects digest to change for this dependency change.

2. **Schema drift can make retained proposals unreadable.** Submit a proposal with an unrelated additive declaration and a String mapping to an existing `Target.p: String`. Change the unused canonical property to Integer. Candidate construction still succeeds, but preview validation propagates `WrongKind`, so both Show and an exact submission retry fail. Preserve the immutable proposal and expose the current incompatibility as a stale preview/blocker with changed review material. Fresh invalid submissions should still refuse.

Limitations: read-only source review, no edits/builds/tests. Review persistence, CLI, application/F, and full E acceptance remain outside this partial slice.

```findings
- file: crates/ekr-kernel/src/schema_proposals.rs
  line: 286
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The effects digest omits effective schema additions and their canonical dependencies, so changing an inherited parent property can change a proposed child's schema without changing any material digest; bind deterministic relevant declaration material and add a schema-dependency regression.
- file: crates/ekr-kernel/src/schema_proposals.rs
  line: 245
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Show and exact submission retries propagate current mapping incompatibilities as errors, making an admitted proposal unreadable after an unused mapped property changes value kind; preserve the retained proposal with an explicit stale mapping blocker and changed review material while retaining strict validation for fresh submissions.
```
