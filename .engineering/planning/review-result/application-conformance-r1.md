---
format: aep.planning-md/3
id: review-result:application-conformance-r1
kind: review-result
status: active
title: Application conformance report integrity review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 coordinator/adapter implementor finding; 0 delegated implementor findings.

Bounded source/log review against committed `9030797f`, covering the integrate/knowledge adapter, application fixture inputs, conformance tests and integrate baseline. ESS conformance review guidance was applied read-only. No builds, tests, mutations, source edits, AEP operations, agents or publication were performed by this reviewer.

The application success observer incompletely checks the actual returned report. `crates/ekr/src/conformance/knowledge.rs:554-561` compares only receipt ID, schema transaction/revision, progress and remaining items with a retained receipt. It does not check the returned application identity, corrections-pending flag, stop reason, already-complete flag or per-item receipts. A real successful application followed by changing only its returned pending item's disposition to Integrated still satisfies this observer, despite replay retaining that item pending and the graph containing no assertions. That incorrect response is then emitted as the declared result event. Existing inert-call and substituted-digest controls do not exercise this gap.

Minimal correction: decode the generated ApplicationReport, compare every shared field with the matched retained receipt and election, and verify this fixture's exact qualified pending item against replay: source/item/mapping digest, pending disposition, absent transaction/assertions, and consistent reason. Assert this fresh invocation is not already complete. Add a report-tampering negative control at the helper boundary after a real native application, changing one returned item outcome or application identity while retaining the same actual provider state. This is a source-derived conformance-observation gap, not an independently executed mutation or a finding that the current Runtime actually returns false reports.

The remaining inspected changes are faithful to their bounded purpose. The adapter passes the three authored application inputs into Runtime without replacing them at dispatch; fixture setup independently signs and retains approval through the public runtime. Refusal uses an actually mismatched requested digest, and the observer requires both unchanged canonical head/occurrences and absence of application election/receipts. Positive observation reopens with full replay and checks actual schema advancement, a committed ordinary transaction, retained progress and absence of guessed assertions. The new fixture declarations add resolve-fixtures floors for both application scenarios. The total inventory remains 19, answered floor 19, unavailable ceiling zero and quarantine empty. The selected application test does not replace either full-suite gate.

Evidence inspected under `<retained-evidence>/schema-application-kernel/`:

- `application-conformance-baseline.log` records the prior 16 passed, one failed and two unsupported result per provider. The unchanged finite SubmitSchemaProposal input remains invalid for its answered scenario.
- `conformance-run4/full-File.json` and `full-Sqlite.json` each record 18 passed, one failed, zero error/unsupported/skipped, total 19. The sole failed scenario is `ekr.integrate.SubmitSchemaProposal/outcome/answered`; both ApplySchemaProposal branches pass. This is measured execution evidence, not full conformance attainment.
- `application-conformance-4.log` is terminal with seven test functions passing and the two full-provider suite tests failing. It records the application subset and negative-control tests passing, and includes application in result-event suppression accounting. The inspected inert and changed-digest count reports name exactly the application answered scenario as failed, with no error or unsupported substitution.
- The runner report's `completed_at` is the deterministic fixture clock (`1700000003900` in the full File report). It is not current wall-clock attainment time. No current conformance claim is inferred from it, and no full gate is claimed.

The upstream ESS416 diagnosis is coordinator context, not independently investigated here. The local finite input and actual Submit failure are visible in the inspected evidence; neither is hidden or quarantined by this patch. No CI-equivalent run or three-run stability claim was independently verified.

Inspected source SHA-256 fingerprints:

- `crates/ekr/src/conformance/knowledge.rs`: `2cfba20554ec74d8db8162eec5f11664b7f5450935f8b6650e08d4c645c9d2cc`
- `crates/ekr/src/conformance/integrate.rs`: `9bc5aa4f264bc5864a6c8a203ecc58554341660b5b89acc9e8f7dea27f8a3e7e`
- `crates/ekr/tests/conformance_integrate.rs`: `f0feabe5144504448515bee4bb72500a6fc489df792aaa2a99faffd9754eefbe`
- `systems/ekr/conformance/integrate-baseline.json`: `3eb6b0ee8aa0a918e11dab526a092d089a625e56a43acb68997f0d94cc48293f`

```findings
[
  {
    "file": "crates/ekr/src/conformance/knowledge.rs",
    "line": 554,
    "category": "testing",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "verify_application corroborates only a subset of returned ApplicationReport fields and never checks per-item outcomes. After a real application leaves the selected fact pending, changing only returned items[0].disposition to Integrated still passes this observer and is emitted as a conforming result, although replay contains no corresponding assertion. Use typed report/receipt/election comparisons and exact qualified pending-item checks; add a returned-report tamper control after a real native application. Source-derived observer gap, not an independently executed mutation or a claim that current Runtime produces the incorrect report."
  }
]
```
