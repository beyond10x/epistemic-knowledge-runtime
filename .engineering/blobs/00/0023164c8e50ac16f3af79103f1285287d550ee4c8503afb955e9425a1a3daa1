---
format: aep.planning-md/1
id: review-result:p1-14-exit-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-14 exit unit
relations:
- reviews: story:p1-exit-properties
revision: 1
---
Adversary pass 1 on unit p1-14-exit (wave p1-14), `aep-drive:adversary`, against `ba0f59d`.

Owners: 3 findings, 3 implementor (introduced).

Verdict: CONFIRMED (1 warning, 2 notes). cases: executed 559→563, red 1.

Cases added: `crates/ekr-graph/tests/adversary_p1_14_exit_typed_assertion_refs.rs` with three
trybuild cases under `crates/ekr-graph/tests/adversary_p1_14_exit_compile_fail/` (red: a
`LocalRef<Assertion>` id compiles into `AssertionLifecycle::Superseded.by`,
`Assessment::Disputed.competing_assertions` and `EvidenceSource::GraphAssertion`), and
`crates/ekr-kernel/tests/adversary_p1_14_exit_wire_shape.rs` (3 cases, green: canonical bytes, JSON
and decoding of the typed edge subject and evidence set equal the legacy bare-id forms).

What reaches finding 1: nothing found at runtime (`apply.rs:140` fills `Superseded.by` only from a
checked `Supersession`; provenance admits only `Proposed`/`Active`; seed admission refuses sources
other than a human statement). It is a type-level gap against AGENTS.md invariant 2.

Routing (coordinator): finding 1 back to the implementor to type the three fields (invariant 2
is the P1 exit clause), keeping the adversary's cases; findings 2 and 3 back with it.

```findings
[
  {"file": "crates/ekr-graph/src/lib.rs", "line": 35, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The doc says every canonical reference kind is typed against transient identities, yet Superseded.by, Disputed.competing_assertions and EvidenceSource::GraphAssertion accept a LocalRef<Assertion> id and compile (adversary_p1_14_exit_typed_assertion_refs.rs red); at runtime the validators refuse a dangling one, so the fix is the sentence or the three fields."},
  {"file": "verification-report.md", "line": 1, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The report names a non-existent Assessment::Contested (the variant is Disputed) and credits the reference/lifecycle validators with refusals that the provenance validator and seed admission actually make."},
  {"file": "crates/ekr-kernel/tests/p1_exit_properties.rs", "line": 388, "category": "property", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "32 unseeded, unpersisted cases leave each node-reference shape out of a run with probability about 0.31, and the generator never produces a NodeRef inside a value, an absent merge target or an absent graph root."}
]
```
