---
format: aep.planning-md/3
id: review-result:adversary-sdk-02-v-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-02 unit V, pass 1
relations:
- reviews: story:sdk-read-helpers
revision: 1
---
## Verdict

NEEDS-CHANGE on `6bc397dc`, five findings; four fixed in `ec17d2b4`, the revision-pin mutant covered by
the adversary's green case in `63048bd4`. Held under attack: the main-then-views parser order (no
shadowing, no swallowed usage error), refusals equal to the routes' for limits, since, a revision
past the head and a missing node, the index cache across foreign commits and an in-place store
replacement on both providers, and the SDK enums against every variant the engine emits.

```findings
- file: docs/sdk.md
  line: 193
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Snapshot and Ontology read nested records through deny_unknown_fields document types, so a field a newer ekr adds failed the read the doc promised to tolerate"
- file: crates/ekr/src/cli/session/views.rs
  line: 79
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "views verbs accepted a +-signed limit or revision that the ekr view route refuses as invalid-query"
- file: crates/ekr/src/cli/mod.rs
  line: 272
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "ekr session --help and the session-verb-unknown message named none of the six views verbs"
- file: crates/ekr-sdk/src/read/mod.rs
  line: 429
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "dropping the expand iterator's revision pin kept the suite green because no case committed between pages"
- file: crates/ekr/src/cli/session/views.rs
  line: 195
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a session started before its store existed reopened and re-indexed the store on every views request"
```
