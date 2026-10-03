---
format: aep.planning-md/3
id: review-result:human-decision-identity-independent-r2
kind: review-result
status: active
title: Shared human decision identity checkpoint review
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
needs-revision

Owners: 0 coordinator/kernel implementor findings; 1 delegated store/graph implementor finding. Kernel integration, compatibility disposition and full PR acceptance remain coordinator-owned.

Bounded source and retained-log review of the ten-file store/graph correction over 89fc4337fce53b51072c62d066b3207093c32490. No tests, builds or mutations inside the repository were performed by this reviewer.

The shared singleton and atomic publication groups address the physical cross-operation identity race: new canonical signed publications and proposal reviews reserve the same tenant-local decision stream with Expected::NoStream. Preparation authorization checks the exact shared append, and full history loading verifies new signed occurrences against their binding. Legacy canonical envelopes, original review events and already-published preparation retries retain their old representation. The remaining defect is the checkpoint head shortcut.

1. Blocker — crates/ekr-store/src/eventlog.rs:699 returns a checkpoint-backed root without checking the new mandatory human-decision binding. checkpointed_head loads only the latest head record before calling the authority; head returns its result before the history path that calls require_human_bindings. KernelAuthority::head_by_binding rejects histories containing AuthorityUpgraded, but does not reject an earlier AttentionAnswered followed by an ordinary RevisionCommitted (crates/ekr-kernel/src/checkpoint.rs:184). The pointer binds revision occurrences, not the separately retained shared singleton. Thus a missing or corrupt new signed-answer binding can be rejected by history/full replay while a cold checkpoint-backed head read still succeeds.

   Concrete reproduction to add: retain a /5 AttentionAnswered followed by an ordinary commit, install a verified checkpoint pointer covering the complete occurrence stream, then remove or corrupt that answer's shared decision singleton through the fixture provider. Reopen and require both head and history to refuse the binding defect. This is a source-derived reproduction, not an independently executed result. A store test with an injected checkpoint-capable authority can isolate the bypass; kernel integration should cover the real authority. Decline the shortcut for histories containing requires_human_binding occurrences, or load and validate those bindings before returning the root. The current cold-replay regression calls history only and cannot detect this shortcut.

Evidence reviewed: <retained-evidence>/report.md, base.log, red-store.log, red-graph.log, green-focused-2.log, green-package.log, clippy.log, format.log, diff-check.log and their status files. The package summary totals are 299 passed, 0 failed and 3 ignored; baseline totals are 291 passed, 0 failed and 3 ignored. Focused store evidence records 30 passed, 0 failed and 1 ignored. Clippy, formatting and diff-check status files are zero. These are implementor executions inspected by this reviewer, not independent executions. The red race failed with [Err(Document("invalid-publication-envelope")), Ok(true)]; it did not demonstrate two successful duplicate writes. The report's description of that red as a successful duplicate race must not be used as behavioral evidence. Recovery and legacy-review compatibility cases were not individually observed red.

Limitations and compatibility: synthetic store authorities establish physical mechanics, not signature verification, audience authentication or semantic kernel admission. The original cross-domain admission finding is not closed end to end by this store-only review; the parent still must select the new envelope and integrate authenticated lookup/replay. Unpublished legacy signed preparations deliberately refuse human-decision-revalidation-required and retain their occupied immutable command slot. This is fail-closed behavior, not automatic migration or successful same-head re-election; its compatibility disposition remains explicit coordinator work. Mixed old/new writers are unsupported. No full workspace gate, ESS conformance or complete E/F acceptance is claimed.

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 699
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The checkpoint-backed head shortcut returns before require_human_bindings and loads only the latest head record. A /5 AttentionAnswered followed by an ordinary commit remains eligible for the real kernel shortcut, so a missing or corrupt shared decision singleton can make history/full replay fail while a cold head read succeeds. Decline the shortcut for histories containing requires_human_binding occurrences or validate their bindings before returning; add a cold checkpoint-head regression that removes or corrupts the index and requires refusal. This finding is source-derived; the reviewer did not execute the reproduction."
```
