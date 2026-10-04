---
format: aep.planning-md/3
id: review-result:correction-integration-r2
kind: review-result
status: active
title: Frozen correction provenance and retained object integrity rereview
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/kernel implementor findings; 0 new delegated implementor findings. Both findings in correction-review-r1 are closed within this bounded source/log rereview.

The frozen correction now records `correction_review_id` (`application_correction.rs:125`). `application_auth::correction_origin` resolves that reference to a real Approved decision from the authenticated review chain. Structural retained-step verification checks origin membership and chronology without re-deriving effects against already resolved current disputes. At publication, the latest effective review and current residual material are still checked separately (`application_auth.rs:1135-1160`), then exact frozen operations are checked using the original decision and statement (`application_auth.rs:1162-1173`). This permits renewed approval without changing allocations or original provenance and does not reinstate a rejected approval as current authority.

Explain captures both the original review and the effective commit review, verifies their object bytes and the actual guarded commit, and resolves canonical statement evidence from the original review. The generated optional origin field has `serde(default, skip_serializing_if = "EssPresence::is_absent")`; schema/mapping steps retain absent-field serialization. The new field does not force historical non-correction records to acquire another encoded field.

`application_auth.rs:494-511` now requires every review's embedded proof, policy and statement to match both its declared content hash and the exact independently retained object in the supplied history before producing a verified decision capability. An embedded signed proof no longer substitutes for a missing retained proof object. Current and original reviews use the same checked chain.

Observed implementor evidence, all under `<retained-evidence>/schema-application-kernel/`:

- `correction-renewal-red.log`: the original implementation returns Partial with "correction step has no prior exact approval"; one failed test. This is the concrete reproduction of R1's renewal finding.
- `correction-integration-4.log`: terminal four passed, zero failed, covering Choose, Unresolved, renewed approval and temporal/multi-dispute correction with the missing-proof negative control. Source loops execute both File and SQLite. That executed renewal test checked exact preservation of the frozen step, returned both typed reviews in Explain, and checked full-replay reopen equality. The explicit assertion that `original_review.review.review_id == approved.review_id` was added afterward; it was inspected in the current source but is not claimed as executed by this earlier run.
- `correction-recovery-2.log`: terminal one passed, zero failed in 98.66 seconds. Its both-provider fixture covers missing final receipt, terminal-stale successor, rejection after validation followed by renewed approval, and refusal of a copied unsigned correction transaction. Frozen step/replacement identities remain retained. The earlier recovery run's assertion compared all provider events; the corrected assertion checks unchanged canonical revision events while permitting receipt metadata recovery. This is an honest expectation correction, not a production red/green claim.
- `correction-after-mappings.log`: terminal one passed, zero failed in 64.65 seconds. The both-provider source checks the actual committed order schema, mapping, mapping, correction; exactly four new revisions; completed effects; no-write retry; and full-replay Explain equality.

The expanded removal/alteration controls for all three review objects, on warm and fresh authorities, are present in `schema_application_corrections.rs:155-176`. The current `correction-regression-1.log` rerun covering those controls, detached-provider coverage and the explicit original-review-ID assertion was still running when this chronology correction was supplied; no green execution is claimed for those additions. The earlier executed missing-proof case and the exact source checks support closure of R1's specific retained-input finding. This paragraph records the coordinator's chronology correction without further review or execution.

Inspected source SHA-256 fingerprints:

- `crates/ekr-kernel/src/application_correction.rs`: `e550489103451ee0241c53952d2c78861b599edbed3b455dd2b5669a3a837eb9`
- `crates/ekr-kernel/src/application_auth.rs`: `1bc42af468bb054376bddcc444b25d7f2046e342268b1de6786c0edca92bb7c5`
- `crates/ekr-kernel/src/application_inputs.rs`: `846761dae733ced13ae5800aadd3e98e4ad3884d8951f7673427cda0ee00071c`
- `crates/ekr-kernel/src/explain/schema_corrections.rs`: `9845f26e25dba9e868b16aba758b381cc745b9b1499f70a1466ece99c0318349`

Limitations: source/log review only; all executions above belong to the implementor. This reviewer ran no Cargo, builds, tests or mutations and made no source/AEP/publication changes. No new concrete defect was established in the two corrected paths. Approval is limited to these corrections and their stated evidence, not full F, every failure boundary, the pending expanded controls or the full gate.

```findings
[]
```
