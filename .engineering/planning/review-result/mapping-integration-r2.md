---
format: aep.planning-md/3
id: review-result:mapping-integration-r2
kind: review-result
status: active
title: Mapping report and residual material correction rereview
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/kernel implementor findings; 0 new delegated implementor findings. The prior report-prefix finding and residual-material finding are closed within this bounded correction review.

`application_progress.rs:137` extracts the committed boundary from the supplied receipt's verified schema and processing links. Both receipt verification and report generation use that calculation. `report` now validates the supplied receipt at line 239 and derives item outcomes only through the same boundary at lines 240-241. An intervening later mapping commit therefore cannot enter the items of an older partial receipt. No authority is granted by the receipt: verification still reconstructs its expected fields from admitted canonical commits.

The focused regression at `application_progress/tests.rs:71` performs actual schema and mapping application on File and SQLite, then cold-replays the retained history. It reconstructs a valid older receipt projection using actual committed revisions and retained processing identities, and requires the later item to remain pending without transaction/assertion IDs. It also checks the genuine complete receipt and rejects an altered remaining-items field. This exercises the report seam with real runtime history; it does not persist that reconstructed older receipt or schedule an actual concurrent pair of callers.

Observed implementor evidence:

- `<retained-evidence>/schema-application-kernel/application-report-prefix-red.log`: changing the report lookup back to current-head processing produces the intended assertion failure, integrated V1 versus pending V2; one failed test. The red run stops in the first provider branch, so it does not demonstrate separate red executions on both providers.
- `<retained-evidence>/schema-application-kernel/application-mapping-regression-1.log`: corrected library suite is terminal with 62 passed, zero failed, including the both-provider report-prefix test. The integration target is also terminal with 12 passed, zero failed, including the extended interrupted-mapping test.
- `<retained-evidence>/schema-application-kernel/application-mapping-residual-red.log`: removing completed-addition clearing produces the intended residual effects-digest mismatch. The restored source clears those additions (`application_material.rs:30-32`) while validating the original candidate from `original_proposal` (`schema_proposals.rs:315`). The extended interruption test covers unrelated change to completed ReviewVocabulary versus material change to the pending Project declaration, refusal without canonical publication under the old review, renewed residual approval, cold/full resume, omitted-receipt recovery and exact retry across both providers.

Limitations: source/log review only; all execution above belongs to the implementor. No Cargo, builds, tests, mutations, source edits, AEP operations or publication were performed by this reviewer. This approves only the two reviewed corrections and their stated evidence. It does not approve full F, correction application, all concurrency/recovery boundaries or the full gate; corrections remain refused in the current slice.

```findings
[]
```
