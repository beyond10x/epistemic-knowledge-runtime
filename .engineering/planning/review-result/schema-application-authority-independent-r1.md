---
format: aep.planning-md/3
id: review-result:schema-application-authority-independent-r1
kind: review-result
status: active
title: Schema-only application authority independent review, round 1
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 2 findings, 2 coordinator/kernel implementor, 0 delegated implementor. The reserved-identity finding crosses the integrated kernel/store preparation boundary; its missing command-side check is coordinator-owned.

Bounded source/log review of the uncommitted schema-only application authority slice over d0ff800fe. No tests, builds, mutations, or independent execution were performed. Both findings below are source-derived reachable paths, accepted by the coordinator for red-first reproduction; that acknowledgement is not execution evidence.

1. **Earlier apply time can persist an election that subsequently makes the store unreadable.** `crates/ekr-kernel/src/schema_application.rs:144` converts the caller's time and retains the election at lines 148–177 without checking it against the approved review's recorded time. Store election admission checks retained identity/material links but does not make that temporal check. Later, `application_auth.rs:353` rejects an election predating its initial review. The attempt-election history read therefore discovers the failure only after the election and step have been retained; subsequent ordinary reads encounter the same invalid election. Reproduce on both providers with the existing signed schema-only fixture: approve at millisecond 3, apply at millisecond 2, require refusal with the complete published-event stream unchanged, then require a cold/full-replay read to succeed. Validate the election's temporal prerequisite before its first retention write.

2. **An elected transaction can escape the guard before its attempt exists.** `crates/ekr-kernel/src/application_auth.rs:141` searches only retained attempts; the no-attempt branch returns an ordinary unguarded publication at line 150 even when the ID is already frozen in an application election or step. This state occurs legitimately between the separate election/step/attempt writes, including after interruption. The application read exposes that transaction ID. For an unguarded candidate, `crates/ekr-store/src/applications.rs:1120` captures election rows only through the last canonical occurrence, since there is no guard to select the candidate's newer metadata. Thus a fresh election made after that canonical occurrence is absent from preparation authorization, and the otherwise defensive reservation checks in `stage_application_marker` cannot see it. `preparation.rs:1561` can then submit the authorized ordinary native request; complete-history reads subsequently reject its unguarded elected occurrence. Reproduce with a deterministic failure or pause after a real valid election/step and before attempt retention, then invoke generic Propose using the elected schema transaction. Require refusal before any canonical occurrence/preparation publication and preserve readable complete history on reopen. Recognize reserved IDs at election and step stages in the command path, and retain a store-level protection compatible with historical preparation capture. Do not substitute unrelated current material for an old preparation's authorization basis.

The inspected supported path does verify signed historical review material, initial template and statement equality, effective review prefixes, exact attempt bytes, and per-occurrence marker links against the pre-occurrence replay state. Application histories disable the ordinary occurrence-only replay cache and decline checkpoint shortcuts. The captured-history implementation explicitly uses marker closure, not caller-controlled request IDs as evidence of native atomic grouping. These observations do not negate the two gaps above.

Evidence inspected: `<retained-evidence>/schema-application-kernel/application-implementation-4.log` records four application cases and eight proposal-review cases passing; `application-clippy-2.log` records completed strict kernel clippy. `<retained-evidence>/schema-application-store/capture-focused-final.log` records 13 application-retention and 19 provider tests passing, and its status plus `capture-clippy-final.status` are zero. These are implementor executions. The four kernel application tests cover normal completion/retry, absent/wrong/rejected approval, and removed/rebound guards after a warm read; they do not exercise the two paths above. The full kernel package was still progressing when inspected, so this review makes no whole-package or full-gate green claim.

Limits: source/observation/mapping/correction application is explicitly refused and is outside this slice. The current schema-only driver also reuses the latest attempt without electing a successor after terminal Stale (`schema_application.rs:215`); complete stale-resumption behavior is not established by these four tests. This is not acceptance of full F, its generated conformance adapters, concurrent/uncertain-outcome coverage, CLI/SDK additions, or PR64 A–F completion. No cryptographic or provider atomicity bypass beyond the concrete reserved-identity path is asserted.

```findings
[
  {
    "file": "crates/ekr-kernel/src/schema_application.rs",
    "line": 144,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Apply can retain an election whose elected_at predates its approved review; application_auth later rejects that same election, causing the attempt operation and subsequent complete reads to fail after persistence. Check the temporal prerequisite before election. Reproduce with approval at millisecond 3 and apply at millisecond 2; assert no publication and successful cold/full-replay read. Source-derived path, not independently executed."
  },
  {
    "file": "crates/ekr-kernel/src/application_auth.rs",
    "line": 141,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Ordinary command guard attachment recognizes only attempt IDs, allowing an ID already reserved by a fresh application election/step to remain unguarded before attempt retention. Guard-free historical preparation capture excludes that newer election, so generic Propose can append the elected transaction without a marker and make subsequent complete history refuse. Reproduce by interrupting valid apply after election/step and before attempt retention, then invoking generic Propose with the exposed elected transaction; require refusal before publication and readable reopened history. Source-derived path, not independently executed."
  }
]
```
