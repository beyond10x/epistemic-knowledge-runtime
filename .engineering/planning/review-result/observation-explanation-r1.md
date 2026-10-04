---
format: aep.planning-md/3
id: review-result:observation-explanation-r1
kind: review-result
status: active
title: Observation explanation and authority history coverage review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 2 coordinator/kernel implementor findings, 0 delegated implementor findings. Both are bounded regression-coverage gaps; no runtime authorization bypass is asserted.

Read-only source/log audit of Observation Explain/capture and knowledge/3 historical-profile behavior in the current working tree. No Cargo, test execution, source/AEP edits, agents or publication. This does not approve complete F.

The inspected implementation has a sound local verification structure. `crates/ekr-kernel/src/read.rs:288` captures a private observation map through application_inputs::observed, which validates independently retained imports and rejects duplicate IDs. `explain.rs:366` first binds the caller-visible graph/root to retained records. Its Observation branch at `explain.rs:592` requires the private captured record, actual observation ID, matching content hash and identical payload bytes; the ordinary content-address check still follows. These functions do not query a live incubation root. Changing a caller-owned graph alone cannot bypass the graph/root binding. Source inspection therefore found no concrete Explain support-forgery path in these changes.

1. **Observation explanation tests do not hold the new verification boundary or source-root independence.** `crates/ekr-kernel/tests/schema_applications.rs:1158` only requires Explain to succeed after reopening the same intact provider. The test does check wrapper allocation/attribution and the original interpretation, but it never removes live incubation metadata, asserts the returned Observation Evidence link and exact retained bytes, or damages independently retained observation support. It would still pass if the new Observation identity/record comparison were removed, and it would not detect an implementation that accidentally requires the live source root. Add the named application_observation_explanation_survives_source_root_removal obligation: assert the exact source/wrapper link, remove only live incubation metadata while preserving independent pinned records, then require current/historical Explain and full replay to succeed. Separately remove or alter required observation/source bytes or records and require cold and warm refusal; exercise caller-capture tampering too. Use a planted verification-removal or live-root-dependency mutation to establish which negative/control each assertion detects. This is a missing test boundary, not a claim that root removal or tampering currently succeeds incorrectly.

2. **Knowledge/2 historical compatibility is no longer exercised by latest-upgrade tests.** `crates/ekr-kernel/src/upgrade.rs:110` now selects knowledge/3. The schema_evidence test at `crates/ekr-kernel/tests/schema_evidence.rs:53` uses that latest preview, and its old knowledge/1 fixture now explicitly expects a /1-to-/3 transition at line 381. Thus those passing tests do not demonstrate preserved /2 acceptance, rejection, hashes, pending decisions or Explain. Inspection did not find an explicit retained knowledge/2 fixture/test path in the kernel tests; the constructor assertion at schema_evidence.rs:21 only checks a predicate. Preserve a real /2 history with accepted schema-support and refused Observation decisions, assert exact historical records/roots/profile/Explain before and after a genuine /2-to-/3 transition, and repeat with cold/full replay. This directly tests upgrade.rs's new three-version dispatch and the promised older-profile boundary. attention_explanations.rs:133 still expects knowledge_evidence (/2) through a latest-upgrade fixture; updating that latest-profile expectation alone would not supply historical /2 coverage.

Source inspection of authority.rs and upgrade.rs found that profile support is checked by complete tuple equality, seed-supported profiles remain the original three, historical transition replay accepts exact /1 and /2 destinations, and the new evidence capability returns None before /3. These are positive source observations, not substitutes for the historical witness above.

Evidence inspected: `<retained-evidence>/schema-application-kernel/application-observation-green-4.log` records 9 application tests passing; application-observation-regression.log records the reported focused regression targets passing. These are implementor executions, not independent evidence produced by this review. They do not contain the missing source-root/tamper or explicit /2 cases described above. No whole-package or full-gate claim is made.

Limits: this audit did not re-review the separately assigned support planner/admission implementation, mappings/corrections, conformance adapters or full F delivery. The earlier support-allocation finding in observation-runtime-review.md is outside this report's new finding count and is not rediscovered here. Both findings concern obligations explicitly introduced or affected by the reviewed /3 slice; neither asserts an unexecuted exploit or a proven old-profile regression.

```findings
[
  {
    "file": "crates/ekr-kernel/tests/schema_applications.rs",
    "line": 1158,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Observation Explain coverage only unwraps success after reopening an intact provider. It neither asserts the actual Observation Evidence link/payload nor removes live incubation metadata or corrupts independently retained observation support; dropping the new identity/record comparison would remain undetected. Add the named source-root-removal positive control and cold/warm missing/altered/forged-support negatives, with a measured relevant mutation. Testing gap only; no runtime bypass independently executed."
  },
  {
    "file": "crates/ekr-kernel/tests/schema_evidence.rs",
    "line": 53,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "The schema-evidence tests follow latest preview and now run knowledge/3; the existing legacy fixture tests /1-to-/3. They no longer establish knowledge/2 historical compatibility. Add a real retained /2 witness with accepted/rejected decisions and exact roots/record hashes/profile/Explain, Observation refusal, and /2-to-/3 cold/full replay. Constructor predicates or updating stale latest-profile expectations do not hold this invariant. Testing gap, not a demonstrated old-profile failure."
  }
]
```
