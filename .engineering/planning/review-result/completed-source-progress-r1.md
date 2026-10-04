---
format: aep.planning-md/3
id: review-result:completed-source-progress-r1
kind: review-result
status: active
title: Completed source progress independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/kernel implementor findings; 0 delegated implementor findings. The coordinator's reproduced completed-source projection defect is corrected within this bounded review.

Reviewed the `read.rs`, `schema_gaps.rs` and `attention.rs` source delta over `5e5e814457888529aa57aea3e9c12b3f1d94c6c9`, the focused completed-progress regression and the supplied learning-demonstration source/log context. Viewer work is excluded.

`VerifiedRead::integrated_source_item` uses only private captured application prefixes and only mapping-kind commits. It compares the complete generated InterpretationVersion plus item path, so interpretation identity, version and document digest must all match. The prefix builder takes transactions already admitted as committed by replay and filters their canonical revisions against the captured read boundary. A prepared/elected step, an uncommitted attempt, a later commit outside the capture, or a receipt claiming completion does not supply this new progress signal.

Discovery and blocker attention use that signal only to omit the completed source item's current question. Discovery continues verifying the blocker's document digest before filtering. Neither path changes stored interpretation documents, original blockers or import-time receipts. The underlying interpretation read still returns those immutable historical receipts; this patch does not claim a new current-processing receipt projection on the public interpretation API.

The helper answers whether this exact source item has an admitted mapping, not whether every mapping in every proposal has completed. It does not change application remaining-item calculation, election/attempt state, or mapping resumption. The focused test proves a never-integrated sibling item and a later-version item stay visible. It does not add a dedicated control for multiple selected mapping digests over the same source item; no new defect in application-qualified pending work was established from this source delta. The separate existing attention question for a fully completed approved proposal is not fixed or accepted by this review.

Evidence inspected under `<retained-evidence>/knowledge-learning-demonstrations/`:

- `runtime-3.log`: the ownership journey passes; the health journey reaches its completed application but fails the expectation that discovery groups are empty. This is one passing and one failing test, not a green full demonstration suite.
- `completed-progress-red.log`: the tighter regression fails after cold/full replay because both discovery and blocker attention still include the completed version-1 facts[0] alongside the expected sibling/version-2 items. The failing run stops in the File branch; it is not separate red evidence from both providers.
- `completed-progress-green.log`: terminal one passed, zero failed in 16.37 seconds. The source loops over File and SQLite and requires exactly the two unaffected blockers, unchanged original blocker history, one canonical mapped assertion, and no events appended by discovery/attention/history reads.

Inspected source SHA-256 fingerprints:

- `crates/ekr-kernel/src/read.rs`: `3f97d16833e401301b95eebd3d11e9f9d8ce240b69e20eff219178009d6b5a0c`
- `crates/ekr-kernel/src/schema_gaps.rs`: `1bcb3a5e8f09f0b777e279fad74c4219d2610688f17fe6e7aa37bb91f9e8b13e`
- `crates/ekr-kernel/src/attention.rs`: `00987c3367725001d784ff39e69d9bf18ea7ab1e9ab94f1ca0a440460b84621f`
- `crates/ekr-kernel/tests/completed_incubation_progress.rs`: `cbcc9441d63a7ca92105bf82e2654068c0676869412fc2e9e2b4bcac6f1686bd`

Limitations: source/log review only; the executions are the implementors', not this reviewer's. No builds, tests, mutations, source edits, new agents, AEP changes or publication were performed. No corrected full learning-demonstration rerun, viewer acceptance, full F or full-gate result is claimed.

```findings
[]
```
