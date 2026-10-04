---
format: aep.planning-md/3
id: review-result:observation-admission-runtime-r2
kind: review-result
status: active
title: Observation application allocation correction rereview
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
# Knowledge/3 R1 rereview

**Verdict: APPROVED for closure of R1.** The inspected correction addresses the allocation-basis mismatch, and the retained mutation/green evidence exercises the reported failure. This is not approval of full F, the complete Explain implementation, publication, or all pending regression gates.

Read-only review of the current `ekr-schema-proposals-20261003` working source and coordinator-retained logs. No Cargo or source/AEP writes by this reviewer. Repository-relative citations describe the inspected source, not a new immutable commit.

## R1 disposition

- Planning explicitly captures the approved observed revision (`crates/ekr-kernel/src/schema_application.rs:178`–188), passes that capture into `schema_template` (`:214`–220), and therefore uses the same historical basis as `application_auth::election` (`crates/ekr-kernel/src/application_auth.rs:582`–600). An identical original source record appearing later cannot alter reuse versus wrapper allocation.
- Current canonical/source collisions remain checked before election (`schema_application.rs:198`–200). `application_support::check_current` gathers explicit and selected-fact Evidence IDs and compares complete canonical/source records (`crates/ekr-kernel/src/application_support.rs:13`–48). This does not adopt a later canonical record into the frozen wrapper plan.
- The generated template is decoded and passed through the same exact `application_auth::template` comparison before it can be physically retained (`schema_application.rs:78`–81). The authority helper is now crate-visible; it still reconstructs and compares exact supporting additions and manifest, not a weaker preflight predicate (`application_auth.rs:452` onward).
- Every guarded ordinary occurrence rechecks the current source-ID collision boundary using independently captured support (`application_auth.rs:793`–802), retaining protection against changes after election.
- The normative amendment now states the initial-approval allocation basis, identical-later-source behavior, and current collision preflight (`docs/epistemic-knowledge-runtime-design.md:6088`–6092).

## Observed evidence

`application-observation-basis-red.log` records a compiled execution of `application_wrappers_preserve_source_and_attribution` failing at the exact expected message: `schema-proposal-refused: schema support wrapper is missing`. Result: **0 passed, 1 failed, 0 ignored, 8 filtered out**. The coordinator identifies this as the one-variable mutation that changed the template argument back from the reviewed capture to the current read; the source now inspected passes `&reviewed`. The log itself proves the behavioral failure, while the mutation operation is coordinator-provided provenance, not independently executed by this reviewer.

`application-observation-green-4.log` records **9 passed, 0 failed, 0 ignored** for the schema_applications target, including the wrapper regression and observation-admission case. This is coordinator-run test evidence inspected by the reviewer, not a separately executed suite.

The actual regression loops over File and SQLite and three modes (`crates/ekr-kernel/tests/schema_applications.rs:1006`–1023): Observation source with a different extractor; exact HumanStatement source added canonically after approval; and changed-confidence collision under the source identity. The intervening operation is real ordinary Propose/Validate/Commit (`:1054`–1096). The conflict control requires refusal, identical published events, no application and a valid full read (`:1105`–1119). Success checks exactly selected support, a fresh wrapper, application attribution, original timestamp and unchanged interpretation (`:1122`–1148), then reopen/full replay and Explain (`:1150`–1158).

The green-4 run precedes the additional shared-template pre-retention call and occurrence collision check described above. Those latest changes were inspected in source. The broader `application-observation-regression.log` was still incomplete at inspection: five completed target summaries showed 11, 7, 13, 11 and 5 passes (47 total, zero failures/ignored), followed by an unfinished schema_applications target. No final broader-suite success is asserted here; the coordinator must retain its terminal result before claiming that gate.

Immutable inspected log hashes:

- `application-observation-basis-red.log`: `a82bc5e9a60b0bafabf24cbf91c83f55197f6312af0fd0b82c99b373827f63c5`
- `application-observation-green-4.log`: `a098b552de1668794dfbe1db1468c5cda10bd237d6f18155346b8ac808d99d76`

No remaining concrete blocker to R1 closure found in this bounded rereview. General concurrency, mapping/correction authority, complete conformance and full F acceptance remain outside this disposition.
