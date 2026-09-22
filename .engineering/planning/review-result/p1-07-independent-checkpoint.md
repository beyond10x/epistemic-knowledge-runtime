---
format: aep.planning-md/1
id: review-result:p1-07-independent-checkpoint
kind: review-result
status: active
title: Independent post-membrane checkpoint
relations:
- reviews: story:p1-transaction-membrane-repair
- reviews: story:ontology-types-and-values
revision: 1
---
# Independent post-membrane checkpoint

Reviewed integration: `fb68c28`, containing membrane implementation `6328031e3f2a546e64c72a8b94151b8684d311af`. Read on 2026-09-22. The coordinator was editing planning records and ontology comments concurrently; this report concerns the inspected implementation and names its exact source locations.

Verdict: the seven archived membrane witnesses have corresponding refusal paths and meaningful regression cases. One pre-existing input-decoding gap remains before seed and writer work should proceed.

Owners: 1 finding, 1 implementor (the pre-existing ontology decoding surface), 0 coordinator. The current membrane implementor did not introduce the missing field.

## Evidence and limits

This was a read-only source and acceptance review. No compilation, tests, remote CI queries, source mutation or independent runtime probe ran in this checkpoint. Behavioral conclusions below are static inferences, not newly executed facts.

Inspected the original design sections 13, 19–21 and amendment 87; the archived baseline probes; all transaction validators and candidate indexes; implementation and adversary regressions; the coordinator completion/wave records; and the correctness workflow.

The preserved adversary report records successful package tests, including the new assertion shape matrix, cancellation and surviving-reference cases, independent property/lifecycle operations, and inherited constraints. Those executions are attributed to that report, not to this reviewer. The coordinator's integrated full gate was still running.

## Finding

### Unsupported operation effects can disappear before validation

Amendment 87 explicitly declares `OperationDefinition.sets: BTreeMap<PropertyId, ValueTemplate>`. The Rust input type at `crates/ekr-ontology/src/lifecycle.rs:109` derives `Deserialize` but has no `sets` field and no `deny_unknown_fields`. `Ontology::from_yaml` at `crates/ekr-ontology/src/schema.rs:286` directly deserializes into `OntologyDocument`.

The repaired kernel checks nonempty `preconditions` and `emits` at `crates/ekr-kernel/src/validate/ontology.rs:130`. It cannot check a field decoding has already discarded.

Static reproduction recipe: take an otherwise valid YAML ontology fixture with an unconstrained declared lifecycle transition, add a nonempty `sets` mapping to that operation, load it through `Ontology::from_yaml`, then validate its ordinary `Invoke`. Serde's default unknown-field behavior discards `sets`, leaving the invocation indistinguishable from the valid control. Consequently the resulting validation omits an effect the source schema requested. A future writer would perform the transition without the requested property assignment.

This gap predates the membrane patch. No governed task or story inspected schedules `sets` or `ValueTemplate` admission handling. It is the same failure class the new refusal policy intends to close, at the preceding decoding boundary.

Minimum correction: reject unknown fields on operation definitions at ontology decoding, retaining the existing refusal of represented-but-unsupported preconditions and emissions. Do not invent template semantics in this repair. Review surrounding ontology input structs for the same silently discarded semantic-field class. Keep plain supported lifecycle operations loadable.

Required regression: serialized ontology input containing nonempty `sets` must fail with a named decode/refusal outcome, while the otherwise identical document without `sets` loads and its valid lifecycle invocation succeeds. Preserve the offending input bytes in the test; constructing an `OperationDefinition` directly cannot exercise this boundary. Runtime reproduction remains the next action.

```findings
- file: crates/ekr-ontology/src/lifecycle.rs
  line: 109
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Static inference, not runtime-reproduced in this checkpoint: OperationDefinition derives Deserialize without deny_unknown_fields and omits amendment 87's sets field. Ontology::from_yaml consequently drops nonempty sets before the kernel sees the operation; its preconditions/emits refusal cannot prevent Invoke from validating an operation whose requested assignment disappeared. Reject unsupported input fields before decoding loses them, and add a serialized-input negative case with an unchanged valid lifecycle positive control."
```

## Other observations

The seven original witnesses are addressed in source: deleted edge references use surviving identities; assertions enforce declared predicate/object and endpoint semantics; competing writes refuse; schema and merge operations refuse in P1; opaque preconditions refuse. Positive controls remain for ordinary node/edge changes, property assignments and declared lifecycle transitions. I found no additional supported-operation regression within this bounded static inspection.

The new correctness workflow is configured to run `task check` with read-only repository permission and pinned dependencies. This inspection establishes configuration presence, not a successful remote run or a required branch-protection status. Neither should be claimed until observed.

The coordinator records keep the seed and durable-writer gaps open, place an independent checkpoint before seeding, distinguish provider-only evidence from kernel durability, and assign the proposer-authentication task as a CLI blocker. Those known gaps are not repeated as new findings here. The new seed unit remains separate from explain. No project or P1 completion claim is warranted by this review.

