---
format: aep.planning-md/3
id: review-result:schema-proposal-viewer-independent-r1
kind: review-result
status: active
title: Independent proposal presentation review
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
needs-revision

Owners: 1 coordinator/kernel implementor finding; 0 delegated implementor findings.

The proposal page can hide canonical evidence bound by its review basis. Reproduce by retaining canonical evidence ID `E` with payload A, importing an interpretation using `E` with different payload B, then submitting a proposal selecting that source and citing `E`. Admission currently permits this cross-source collision; the basis includes both records, but the page renders B alone.

Reject conflicting retained/canonical evidence identities during admission and reads, or render both records explicitly. Add a regression with distinguishable payloads; the existing page test exercises only retained evidence.

Escaping and the shared GET-only guard appear sound. Review-state filtering remains explicitly deferred.

Limitations: read-only source review; no independent execution, builds, or edits. Scope is this presentation increment, excluding signed decisions, application, and full E acceptance.

```findings
- file: crates/ekr/src/cli/inbox.rs
  line: 270
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The retained-evidence branch suppresses canonical evidence sharing its ID even when the records differ and both contribute to the review basis; reject conflicting identities during admission/read or render both explicitly, and cover the collision with distinguishable payloads.
```
