---
format: aep.planning-md/2
id: review-result:p1-14-conformance-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-14 conformance unit
relations:
- reviews: story:ess-conformance-kernel
revision: 1
---
Adversary pass 2 on unit p1-14-conformance (wave p1-14), `aep-drive:adversary`, against `ddd83d1` (tree at `658c59f`).

Owners: 2 findings, 2 implementor (introduced).

Verdict: CONFIRMED (1 warning, 1 note). cases: 2 red of 3 in the new conformance file; the suite was not run (plan mode stopped the pass).

Pass-1 check: F1, F2, F6 resolved; F3 and F5 carried as decided (ESS 0.29.0 cannot observe responses); F4 resolved for nulled fields, wrong-value residue is finding B.

Cases: `crates/ekr/tests/adversary_p1_14_conformance_r2.rs` (forged store-event payloads pass all 39 scenarios: red; wrong canonical and basis hashes on a Transactions row pass: red; a snapshot ignoring `at` fails the past-revision scenario: green) and `crates/ekr-store/tests/adversary_p1_14_published_events_paging.rs` (1001 events across the 1000-event page boundary on both providers: green, about 350 s on the file provider).

Routing (coordinator): A back to the implementor for a last correction the coordinator verifies: an authored scenario asserting the written storage class and payload of the store events. B no-op: `canonical_transaction_hash` is published by no event, so the value cannot be tied in ESS 0.29.0; recorded on task:conformance-cannot-observe-command-responses. The paging case stays; its cost is recorded on the wave page.

```findings
[
  {"file": "crates/ekr/src/conformance.rs", "line": 1295, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Every ekr.store event payload can be forged (storage_class Ephemeral, byte_len 0, other hashes, attempt 99) and all 39 scenarios still pass, so the store-event projection and the storage class the kernel writes are held by no scenario."},
  {"file": "crates/ekr/tests/fixtures/conformance/scenarios/a-committed-transaction-row-carries-every-record-it-names.yaml", "line": 68, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "canonical_transaction_hash and validation_basis are held only by defined(), so a Transactions row naming another well-formed canonical transaction and basis passes every scenario."}
]
```
