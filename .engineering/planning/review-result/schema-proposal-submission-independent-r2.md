---
format: aep.planning-md/3
id: review-result:schema-proposal-submission-independent-r2
kind: review-result
status: active
title: Proposal submission review corrections
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
approve

Owners: 2 coordinator/kernel implementor findings resolved; 0 delegated implementor findings; 0 new findings.

The schema-material closure includes inherited declarations and normalizes temporary IDs. The regression requires relevant dependency changes to alter the effects digest while unrelated schema additions leave it unchanged.

Retained reads and exact retries now preserve incompatible mappings as blockers. The corrected value-kind path retains the unresolved-subject diagnostic; fresh invalid submissions still refuse. Tests cover unchanged publication and reopen/full replay.

Reviewed logs show both original failures and the final correction run passing all 14 tests.

Limitations: source/log review only; no independent execution, edits, or builds. Approval covers these corrections to the partial submission/show slice, not review persistence, CLI, F, or full E acceptance.

```findings
[]
```
