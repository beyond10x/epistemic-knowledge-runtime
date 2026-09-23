---
format: aep.planning-md/1
id: review-result:p1-14-conformance-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-14 conformance unit
relations:
- reviews: story:ess-conformance-kernel
revision: 1
---
Adversary pass 1 on unit p1-14-conformance (wave p1-14), `aep-drive:adversary`, against `0ac4eb7` (tree at `b7e12b4`).

Owners: 6 findings, 5 implementor (introduced), 1 pre-existing.

Verdict: NEEDS-CHANGE (1 blocker), CONFIRMED (3 warnings, 2 notes). cases: executed 119→126, red 4.

Cases in `crates/ekr/tests/adversary_p1_14_conformance.rs`: a real Propose appends 1 `ekr.store.ObjectStored` and 1 `ekr.store.PublicationPrepared` to the provider log while the suite expects none (red); a forged SnapshotTaken passes `Snapshot/outcome/taken` (red); dropping Propose/Validate responses passes every scenario (red); nulling six Optional Transactions fields passes every scenario (red); resampled retained-seed time is caught (green); proposal responses carry exact document bytes (green); a retained seed retry leaves the provider log byte-identical (green).

Also measured: `story_contract::external_dependencies_match_the_story` red after the integration merge `b7e12b4` (serde_yaml_ng dev-dependencies from the bindings unit); not this unit's finding, routed to the same correction.

Routing (coordinator): F1 back to the implementor to report real provider-log occurrences and measure the contract correction, which the coordinator applies; F2-F4 back as authored scenarios; F5 back (project SnapshotResult and ExplanationResult); F6: the I/O-error-to-invalid-seed mapping back, the unbounded seed read is pre-existing and matches the CLI (no-op).

```findings
[
  {"file": "crates/ekr/src/conformance.rs", "line": 1078, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The real Propose appends ekr.store.ObjectStored and PublicationPrepared to the provider log, but the target reports only kernel records, so the suite's expect_no_event steps for ekr.store events pass by construction."},
  {"file": "systems/ekr/conformance/suite.json", "line": 9653, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Snapshot/outcome/taken checks only the shape of SnapshotTaken, so a snapshot reporting revision 424242 and a forged root for at=1 passes."},
  {"file": "crates/ekr/src/conformance.rs", "line": 611, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "No scenario observes the Propose or Validate response; setting both to None leaves all 36 scenarios passing, so the declared lossless record projection is not held by the suite."},
  {"file": "crates/ekr/src/conformance.rs", "line": 1227, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Nulling validation_basis, validation_hash, validation_record_hash, terminal_record_hash and both canonical hashes on every Transactions row leaves every scenario passing."},
  {"file": "crates/ekr/src/conformance.rs", "line": 722, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Snapshot and Explain return no response although kernel.yaml declares result SnapshotResult and ExplanationResult."},
  {"file": "crates/ekr/src/conformance.rs", "line": 294, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The seed document is read unbounded like the CLI seed handler, and the target maps a read I/O error to the declared invalid-seed refusal where the CLI reports a fault."}
]
```
