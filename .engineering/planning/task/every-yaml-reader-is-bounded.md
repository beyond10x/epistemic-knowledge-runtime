---
format: aep.planning-md/3
id: task:every-yaml-reader-is-bounded
kind: task
status: active
title: Every YAML reader that takes outside input is bounded
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:seed-document-bounds-alias-expansion
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 4}
---
## What is wrong

Wave correct-07 unit B (2026-10-02) gave the seed and transaction-document readers one bounded YAML
loader (`crates/ekr-kernel/src/yaml.rs`: size cap, depth bound before the full load, alias-expansion
budget). Other YAML readers in the repository still call a plain `serde_yaml_ng::from_str`:
`Ontology::from_yaml` (`crates/ekr-ontology`) and `crates/ekr-views/src/code_names.rs`. The
extraction reader (`crates/ekr-integrate/src/extraction.rs`) calls the vendored
`from_str_within_depth` directly, because the kernel's module is crate-private. Each reader that
takes a document from outside the process is a place the loader's bounds do not hold.

Also from unit B: `vendor/serde_yaml_ng/tests/test_observation.rs:173–183` asserts
`started.elapsed() < 5s`, a wall-clock assertion the repository forbids (AGENTS.md); it belongs with
`task:rendering-cost-test-passes-under-load`.

## Build

One bounded loader every outside-facing YAML reader uses: move it where `ekr-ontology`,
`ekr-views` and `ekr-integrate` can depend on it (or the vendored crate's facade), and route each
reader through it with limits stated per format.

## Acceptance

- No crate outside the vendored one calls `serde_yaml_ng::from_str` or `from_slice` on input from
  outside the process (a source guard names the allowed sites).
- Each routed reader refuses a document past its depth before the full load, counted as unit B's
  test counts it.

## Refined baseline and scope

Source inspection supersedes the broad mechanism claim above. Ontology::from_yaml in
crates/ekr-ontology/src/schema.rs still directly decodes caller input. ExtractionDocument::from_yaml
already runs ekr_core::decode::observe_yaml before decoding. CodeNameIndex parses only
EMBEDDED_DOMAINS in crates/ekr-views/src/code_names.rs; those are compile-time repository documents,
not caller input. The vendored depth test already uses event counts.

Cited surfaces: crates/ekr-ontology/src/schema.rs, crates/ekr-core/src/decode.rs,
crates/ekr-kernel/src/yaml.rs, crates/ekr-integrate/src/extraction.rs.
Inferred edits: ontology reader and its existing tests; a source inventory guard under
crates/ekr/tests or ekr-ontology/tests; shared bounded decoder only if the existing facade
cannot express the established reader policy. No change to embedded domain matching is needed.
Coordinator owns planning and shared docs. No new entity or retained format is introduced.

## Revised acceptance

Inventory every production YAML ingress and classify caller input, bounded decode, or trusted
retained/embedded input with its caller. Ontology input has stated byte, nesting and alias-work
limits enforced before full materialization. Reuse existing shared primitives; do not duplicate
the loader. Preserve successful supported ontology inputs, including previously supported
bounded aliases unless an existing format contract forbids them. A case exceeds each bound
and proves refusal; a deterministic event/materialization counter proves excessive depth is
refused before full loading. The source guard detects a newly added unclassified outside-input
decode, and explicitly permits embedded and already-bounded decodes. Existing extraction,
seed and transaction-document limits and refusals stay covered. These regression cases
assert surviving behavior after the fix, rather than constructing a now-unrepresentable value.
