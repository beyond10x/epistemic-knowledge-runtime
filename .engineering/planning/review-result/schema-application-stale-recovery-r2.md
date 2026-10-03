---
format: aep.planning-md/3
id: review-result:schema-application-stale-recovery-r2
kind: review-result
status: active
title: Schema application prepared validation recovery rereview
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new findings, 0 coordinator/kernel implementor, 0 delegated implementor. The prior coordinator/kernel pending-validation recovery finding is closed for its reported path.

Bounded source/log rereview of the pending-validation correction. No tests, builds, source/AEP edits or publication were performed. This report does not replace the first-round finding record.

`crates/ekr-kernel/src/commands.rs:54` extracts the existing validation key without changing its transaction, proposal/authority-transition predecessor, or command-kind fields. The ordinary Validate handler and the new crate-private application helper share that function. `commands.rs:365` reads the checked retained preparation, decodes its actual validation or rejection record through the existing record reader, and selects that record's original requested revision. It uses the current head only when no preparation exists. `commands.rs:374` then invokes ordinary Validate, which still checks lifecycle state, recomputes the revision-bound input hash, compares it with the retained preparation, and uses the existing publication/recovery path. No new validation token or publication shortcut is introduced. `schema_application.rs:311` now calls this helper before deciding whether ordinary Commit has produced terminal Stale and permits a successor.

The reported failure is therefore addressed: a pending validation against revision N is not silently changed into a request against N+1. The original preparation remains the authority input; unrelated head advancement is handled by ordinary publication conflict recovery and subsequent terminal-Stale processing. Source inspection found no weakening of the frozen transaction, original preparation bytes, or successor prerequisites in this fix.

Evidence inspected: `<retained-evidence>/schema-application-kernel/application-pending-validation-red.log` records the original `a publication preparation exists for different input` failure. `application-pending-validation-green.log` records 1 passed, 0 failed, 0 ignored, 6 filtered. That implementor execution covers four recovery boundaries on both providers, including a genuine pending Validate preparation elected without resume, unrelated advancement and cold Apply recovery. The helper test obtains the decision from a real closed validated snapshot, checks exact preparation equality when electing it in the original prefix, and asserts the target transaction remains Proposed before recovery; it does not fabricate an admitted outcome.

The source now additionally includes a fifth boundary without head advancement: the same pending preparation is recovered, the original transaction ID is retained, and the attempt count remains one. Its source assertions were inspected, but the four-boundary green log above is not evidence that this subsequent extension ran. The original completion, frozen-successor comparison and exact-repeat publication assertions remain present.

Limits: this is approval of the bounded correction, not complete F or a full gate. No independent execution, complete uncertainty/concurrency campaign, new rejected-preparation runtime case, or final-source package result is claimed. A preparation lookup racing a concurrent writer can still return a normal command conflict; this review does not assert wait-free progress. Source/mapping/correction application and unrelated changes remain outside scope.

```findings
[]
```
