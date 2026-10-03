---
format: aep.planning-md/3
id: review-result:schema-application-stale-recovery-r1
kind: review-result
status: active
title: Schema application stale recovery independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 coordinator/kernel implementor finding, 0 delegated implementor findings.

Bounded source/log review of the terminal-Stale recovery delta relative to 2886e477559c1c60e7563e94bece005a1a40892d. Reviewed schema_application.rs, application_auth.rs, the new recovery test, and the necessary existing ordinary preparation and attempt-election behavior. No tests, builds, source/AEP edits or publication were performed.

The new successor path preserves the frozen generated transaction by cloning the retained step and changing only its transaction ID (`crates/ekr-kernel/src/schema_application.rs:250`). It records both the predecessor transaction and hash of the actual retained Stale record. The existing store method requires the latest predecessor for a new election, compares all transaction fields except ID, invokes kernel authority and elects by CAS; the caller uses the returned winner. Kernel verification checks terminal Stale, exact record hash, frozen transaction equality and predecessor election time, with the new additional check that successor election does not predate the Stale decision (`application_auth.rs:540`). Unknown ordinary command errors propagate instead of authorizing a successor. Existing committed-progress detection still precedes new execution and recovers a missing completion receipt from the linked actual commit.

**Finding: a retained but unpublished Validate preparation is not recovered before selecting a new validation basis.** At `crates/ekr-kernel/src/schema_application.rs:310`, a transaction still observed as Proposed is always validated against the current head. However, `commands.rs:365` includes the requested revision in Validate's immutable input hash, and `commands.rs:154` rejects a retained preparation whose input hash differs. After an interruption or uncertain provider result leaves the original Validate preparation durably retained but its validation occurrence unpublished, unrelated canonical advancement changes that input. Apply then repeatedly fails with PublicationInputConflict instead of resuming the original validation, observing its terminal Commit/Stale outcome and only then electing a successor.

Bounded reproduction: retain the application's real Propose occurrence and its genuine Validate preparation against revision N, inject a failure before the canonical validation append, commit unrelated work at N+1, then cold-reopen and Apply. Assert recovery of the original validation request, terminal Stale for its original attempt, exactly one successor with unchanged operation/allocation fields, and final completion. Include the no-head-advancement control and ensure uncertainty does not by itself create a successor. Resolve the existing Validate preparation using its original requested basis before selecting a new basis for a never-prepared validation.

This validation call was carried from the checkpoint into the new helper; the finding is an existing orchestration omission exposed by examining the requested recovery boundary, not an asserted regression in the new transaction cloning. Formal origin is undecided because the reproduction has not been executed against either candidate or base. The coordinator has received the source trace.

Evidence: `<retained-evidence>/schema-application-kernel/application-stale-red.log` records the old driver refusing with `schema transaction has not committed`. `application-stale-green-1.log` records the new focused test passing: 1 passed, 0 failed, 0 ignored, 6 filtered. Its source loops both providers and covers validated-before-Stale, already-terminal-Stale, and committed-without-receipt boundaries. It checks one retained step, expected attempt/receipt counts, unchanged frozen transaction after normalizing only the successor ID, final revision advancement and no publication on exact repeat. Its earliest interruption point is after the validation occurrence; it does not cover an unpublished Validate preparation. These are implementor test executions, not independent execution evidence.

Limits: no broad F re-review, concurrent successor fault injection, all-command uncertainty campaign, full-package result or full-gate acceptance is claimed. Source/mapping/correction application and unrelated presentation/store changes are outside scope. No authorization bypass or duplicate canonical effect is claimed for this finding; the demonstrated source consequence is persistent recovery refusal.

```findings
[
  {
    "file": "crates/ekr-kernel/src/schema_application.rs",
    "line": 310,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "undecided",
    "message": "A still-Proposed application transaction is always validated against the current head without first resolving a retained unpublished Validate preparation. After unrelated advancement, Validate's revision-bound input hash differs from that immutable preparation and pending() returns PublicationInputConflict on every retry. Recover the original preparation/basis before selecting a new validation input, then observe terminal Stale before successor election. Reproduce with failure after validation preparation retention but before its canonical append, unrelated commit, and cold Apply retry. This call is carried from the base; origin remains undecided without execution. Source-derived recovery refusal, not an independently executed test or authorization bypass."
  }
]
```
