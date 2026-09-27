---
format: aep.planning-md/1
id: story:refuse-discarded-ontology-semantics
kind: story
status: implemented
title: Refuse ontology semantics unsupported by the decoder
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
- implements: executable-system-specification:ekr-v1
scope:
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
- confidence: cited
  path: crates/ekr-ontology/src/lifecycle.rs
- confidence: cited
  path: crates/ekr-ontology/src/schema.rs
- confidence: cited
  path: crates/ekr-ontology/src/types.rs
- confidence: cited
  path: crates/ekr-ontology/src/value.rs
- confidence: cited
  path: crates/ekr-ontology/tests/adversary_decoder.rs
- confidence: cited
  path: crates/ekr-ontology/tests/ontology_load.rs
revision: 11
---
## Context

The independent post-membrane checkpoint found that Ontology::from_yaml silently discards amendment 87's OperationDefinition.sets because the Rust input type omits it and serde allows unknown fields. The existing kernel refusal then cannot see an effect the decoder discarded. This pre-existing defect is recorded before routing; runtime reproduction is required.

## Acceptance

Serialized input containing unsupported sets is refused with a named decoding error before a valid operation can be admitted. The otherwise identical supported lifecycle document loads, and its invocation validates. Unknown semantic fields elsewhere in the ontology input cannot be silently dropped. Supported documents preserve their roundtrip and existing tests. Do not implement an invented ValueTemplate language.

## Scope

Cited: crates/ekr-ontology/src/lifecycle.rs, schema.rs, types.rs and value.rs for serde input declarations; crates/ekr-ontology/tests/ontology_load.rs for serialized boundary cases; crates/ekr-kernel/tests/validation.rs for the real invocation control. Coordinator owns planning and any ESS comment updates. Extend only fields where an unknown member could silently discard requested semantics.

## Verification

Write the serialized failing input first and retain the red output. Apply the minimum fail-closed decoding change. Run ontology and kernel tests plus package clippy and format, followed by integrated gate. The post-membrane independent reviewer checks the correction.

## Deferred capability

Amendment 87 assignment templates remain a required capability of full project completion. Their language and application must be settled in the later schema/runtime phase; the P1 parser refuses them until implemented. This refusal does not remove the predecessor capability.
