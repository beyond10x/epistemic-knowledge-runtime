---
format: aep.planning-md/3
id: review-result:schema-evidence-presentation-independent-r2
kind: review-result
status: active
title: Independent schema evidence review finds historical authority regression
relations:
- reviews: story:schema-transaction-cites-evidence
revision: 1
---
needs-revision

Owners: 1 new coordinator/kernel implementor finding; 0 delegated implementor findings. The prior coordinator-owned mixed-data test finding is resolved.

Reviewed SDK commit `24002eeca1ca487671dde6c4146d33600c514a69` and the supplied presentation working diff. Kernel implementation matched `dded81ab` during inspection.

| Location | Finding | Verdict / origin |
|---|---|---|
| `crates/ekr-kernel/src/read.rs:270` | Retaining only the latest authority transition makes current-read explanations of knowledge/1 assertions fail after upgrading to knowledge/2. | NEEDS-CHANGE / introduced |

`VerifiedRead` stores only the latest `authority_change`. Its `authority_at` method falls back to the seed authority for any earlier revision. After the second upgrade, `explain.rs:717` therefore selects the seed profile for an assertion committed under knowledge/1 and rejects its actual validation hash with `validation-profile-disagrees`.

Concrete reproduction:

1. Open a native knowledge/1 fixture.
2. Commit an ordinary `AddAssertion` and verify its explanation.
3. Complete the reviewed knowledge/2 upgrade.
4. Explain that assertion from `runtime.read(None)`, including after reopening/full replay.

Preserve all verified authority boundaries and select the boundary appropriate to the retained validation. Add a regression asserting the old assertion’s explanation still identifies knowledge/1. This is a source-derived finding; I did not execute the reproduction. The second-transition path is newly enabled by `dded81ab`, although the affected read code predates it.

The previous mixed-data finding is closed: the corrected case clears its unrelated manifest and requires `mixed-schema-transaction`. Retained logs show three passing cases before and after restoration, and both provider cases fail with “mixed schema/data transaction was admitted” under the targeted guard mutation. I also verified `structural.rs` had no difference from `dded81ab`.

No additional findings in the presentation/SDK slice. The inspected code preserves historical filtering, omits empty support fields, uses the shared generated entry type, escapes evidence excerpts, and forms source links from parsed evidence IDs. The end-to-end presentation test checks both providers, normal/full replay, past revisions, SDK decoding, projection, overview and inbox output.

Evidence limits: the retained presentation library log runs one actual case; the earlier binary log runs zero and provides no coverage. The retained regression log totals 365 passed, zero failed, four ignored; the later CLI library log reports 65 passed. These are coordinator-produced executions, not runs by this review.

No builds, mutations or filesystem writes performed. This verdict covers partial D only; authored ESS conformance, remaining viewer acceptance and the full integrated gate remain coordinator-owned. It does not approve PR64 A–F.

```findings
- file: crates/ekr-kernel/src/read.rs
  line: 270
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Retaining only the latest authority transition makes current-read explanations of knowledge/1 assertions fail after upgrading to knowledge/2.
```
