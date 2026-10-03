---
format: aep.planning-md/3
id: review-result:schema-evidence-kernel-independent-r1
kind: review-result
status: active
title: Independent review of schema evidence kernel checkpoint
relations:
- reviews: story:schema-transaction-cites-evidence
revision: 1
---
needs-revision

Owners: 1 coordinator/kernel implementor finding; 0 delegated implementor findings.

Reviewed `dded81ab56351b2b705d62dd2d6b34732856780a` against `0333f526ea7b607c1477fddcd4758d26de0d1694`. One actionable test finding; no implementation defect identified within this bounded source review.

| Location | Finding | Verdict / origin |
|---|---|---|
| `crates/ekr-kernel/tests/schema_evidence.rs:122` | The mixed-data case asserts only rejection, so `evidence-set-mismatch` masks removal of the mixed-schema guard. | NEEDS-CHANGE / introduced |

The case starts with `schema(support)`, retains its nonempty evidence manifest, and adds a `CreateNode` operation. It contains no assertion or attachment citing that evidence. Consequently, `validate/structural.rs:991` rejects it independently of `mixed-schema-transaction`.

Concrete regression probe: disable the mixed-schema refusal only when `schema_evidence` is true at `validate/structural.rs:520`. This case would still observe rejection through the manifest mismatch. **This is a static control-flow finding; I did not execute that mutation.** Clear the mixed case's manifest and require the explicit `mixed-schema-transaction` issue. Root has acknowledged that correction and owns its execution.

Other inspected invariants showed no additional findings:

- Knowledge/1 keeps its original validation path; knowledge/2 activates through a reviewed transition.
- Preview recomputation binds the active predecessor, destination, stream, head and pending validations; replay verifies the retained proof and exact preview.
- Evidence references resolve against canonical records or inline additions; inline payload/source checks remain active.
- Schema history reads immutable committed manifests without changing ontology encodings.
- Native knowledge/1 fixtures preserve original roots and the historical rejection across the second transition and full replay in the retained test evidence.

Limitations: no compilation, tests, mutations, publications or filesystem writes performed. I inspected the checkpoint's retained evidence, including its reported 42 focused passing cases; those are author-produced results, not execution by this review. I do not claim a green full gate or completion of D or PR64 A–F.

Owner split: kernel test correction belongs to root/kernel implementation. Presentation, SDK, generated contracts, named ESS conformance and integrated acceptance remain root-owned and outside this checkpoint verdict.

```findings
- file: crates/ekr-kernel/tests/schema_evidence.rs
  line: 122
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The mixed-data case asserts only rejection, so evidence-set-mismatch masks removal of the mixed-schema guard.
```
