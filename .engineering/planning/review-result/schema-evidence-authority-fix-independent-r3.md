---
format: aep.planning-md/3
id: review-result:schema-evidence-authority-fix-independent-r3
kind: review-result
status: active
title: Independent verification of historical authority explanation fix
relations:
- reviews: story:schema-transaction-cites-evidence
revision: 1
---
approve

Owners: 0 new findings; 0 coordinator findings; 0 delegated implementor findings. The prior coordinator/kernel second-transition finding is resolved.

The fix retains every verified authority boundary and selects the latest boundary at or before the validation revision. `VerifiedRead` captures that map, preserving knowledge/1 attribution after the knowledge/2 transition.

Seed authority remains unchanged. Initializing checkpoint-restored history with an empty map is consistent with existing safeguards: upgraded states do not produce checkpoints, and checkpoint restoration rejects prefixes containing authority transitions. Subsequent replay reconstructs each verified boundary.

The regression checks the explicit knowledge/1 validation profile before upgrading and on historical and current reads afterward, across both providers and normal/full replay. Retained logs show the original `validation-profile-disagrees` failure, followed by three passing cases after the fix.

No additional findings in this bounded verification. I performed source and log inspection only—no builds, tests, mutations or filesystem writes. This closes the specific finding; it does not establish the full gate or remaining A–F acceptance.

```findings
[]
```
