---
format: aep.planning-md/3
id: review-result:schema-application-authority-independent-r2
kind: review-result
status: active
title: Schema-only application authority independent review, round 2
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 new finding, 0 coordinator/kernel implementor, 1 delegated store implementor. The 2 coordinator/kernel findings from authority-independent-r1 are closed for their reported paths; they are not counted again.

Bounded source/log review of the chronology, reserved-ID and fresh-preparation corrections in the current integrated schema-only slice. No tests, builds, source edits or independent execution were performed. The existing first-round report remains unchanged.

The first finding is closed: `crates/ekr-kernel/src/schema_application.rs:145` now refuses a write time before the approved review or existing election before retaining new application metadata. The chronology regression asserts the complete published-event stream is unchanged and a cold full replay succeeds. `<retained-evidence>/schema-application-kernel/application-review-red.log` records the original metadata-change assertion failure; `application-review-green-1.log` records all six application tests passing. The raw chronology failure contains private fixture details; none are copied here.

The second finding is closed for election-first ordering: `crates/ekr-kernel/src/application_auth.rs:147` checks election and step IDs when no attempt exists, and occurrence verification also recognizes all three reservation stages. The regression copies a closed native provider, retains exact election/step records from a real successful application, then submits that application's actual transaction document without an attempt. It checks refusal, no new occurrence/preparation, and successful reopened full replay for both providers and interruption stages. `application-reservation-red.log` records the original reserved-ID admission failure; the six-test green above includes the correction.

The physical fresh-preparation correction also covers a reservation already visible before preparing: `crates/ekr-store/src/applications.rs:1067` checks the captured complete audit's reservation metadata for NewCandidate. `preparation.rs:1482` selects that mode for a newly elected preparation, while `preparation.rs:1370` retains historical Candidate capture when reading immutable preparations. This reuses the existing audit and does not substitute unrelated future application objects into historical authorization. `application-preparation-red.log` records the original fresh-preparation acceptance failure; `application-preparation-green.log` records 14 physical application tests and 2 preparation-authorization tests passing. These are implementor executions. The pre-fix full-kernel result is not a final-source result.

**New physical-contract finding: a reservation can adopt an ID already held by an ordinary pending preparation.** `crates/ekr-store/src/applications.rs:561` rejects an election transaction ID only if it is already published; it does not reject an existing Propose preparation. `elect_application_step` likewise has no ordinary-preparation exclusion before retaining a new transaction reservation. The new NewCandidate check runs too early for preparation-first ordering. An already authorized ordinary request is preserved by read_preparation(false), and `preparation.rs:1566` restores and submits its original native request without a proposal marker. If an application adopts that same ID after preparation but before publication, the unguarded occurrence can therefore append and complete-history reads subsequently reject the inconsistent reservation/occurrence pair.

Bounded reproduction strategy using the existing physical fixture: construct its ordinary publication, remove the application guard and restore the ordinary payload format; prepare it before retaining the fixture's application election with the same transaction ID; then resume the exact preparation. Assert that the reservation is refused before retention, the original preparation remains recoverable, and complete history stays readable. Include an unrelated-ID reservation control and a cold reopen so cached authorization is not the only exercised path. The same identity exclusion belongs at each reservation boundary that accepts a fresh transaction ID; preserving old preparation authority should not mean permitting a conflicting later reservation. This trace uses callable physical store APIs with the fixture's injected authority. It is not a demonstrated Runtime collision: Runtime mints application transaction IDs internally. No concurrent direct-store exploit is claimed or executed here.

Limits: the review covers only these corrections and their immediate preparation-ordering consequences. It does not accept complete F, stale-attempt successor orchestration, source/mapping/correction application, new CLI/SDK surfaces, full conformance or the full gate. Marker closure remains the stated replay model; native group IDs are not available from the feed. No other new defect is asserted.

```findings
[
  {
    "file": "crates/ekr-store/src/applications.rs",
    "line": 561,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Physical application election rejects already published transaction IDs but can adopt an ID already held by an ordinary pending Propose preparation. Prepare the unguarded request first, elect the same ID second, then resume: immutable historical authorization can append the ordinary request without a marker, after which complete history refuses the elected unguarded occurrence. Reject conflicting pending preparation identities before retaining application/step reservations while preserving unrelated old preparation recovery. This is a source-derived callable physical-store contract path with injected fixture authority, not an independently executed test or demonstrated Runtime-generated-ID collision. The two original kernel findings are closed separately."
  }
]
```
