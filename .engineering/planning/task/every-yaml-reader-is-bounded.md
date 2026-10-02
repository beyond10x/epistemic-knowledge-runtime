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
revision: 7
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

## Specified named cases

The ontology ESS domain now declares input, depth, expanded-node and expanded-text bounds matching
the existing seed reader's limits. Named acceptance cases are
ontology_yaml_refuses_input_bytes_before_decoding,
ontology_yaml_refuses_excessive_container_depth,
ontology_yaml_bounds_expanded_alias_nodes_and_text,
ontology_yaml_preserves_bounded_aliases_and_existing_refusals, and
ontology_yaml_stops_loading_at_the_depth_cut. The ingress inventory and its counterexamples live
in crates/ekr-ontology/tests/yaml_ingress.rs. These are planned, unexecuted cases at this specification
commit; later test-result evidence determines implementation. The specification was propagated
identically to both input-wave unit trees before implementation.

## Candidate verification and remaining checks

Candidate 9b65ea399ad0b8ee920f6d923741ad6e0bc63c7f is bot-authored. The coordinator read the
red-all-bounds.log and green-focused-rebuild.log outputs in <cache>/ekr-next-three/input-08/yaml,
and inspected the shared loader move and ontology preflight. VERIFIED for the focused claim:
input past the stated byte, depth and alias-work bounds now receives the named Syntax refusal
before typed materialization. The implementor's interim report-public.md reports three failing
cases becoming passing in the same four-case regression run, plus a passing early-depth event
counter. Its complete package run reports 809 passed and six existing ignored cases, exit zero;
this is quoted from that report, not an inferred clean-baseline count.

The candidate is not yet an admitted unit. Clippy, downstream extraction checks, the independent
adversary and combined wave gate remain pending. Remaining compilation paused when free storage
fell below the wave's build floor. The existing full baseline was interrupted and is not a green
baseline. The immutable focused red output remains the before evidence.

## Scope

Confirmed against the candidate: ontology/schema.rs preflight and ontology_load.rs bounds cases;
core/decode.rs and new core/decode/yaml.rs shared observation/expansion primitives and their tests;
kernel/yaml.rs retains its event counter wrapper; ontology/tests/yaml_ingress.rs inventories all
crates/*/src and xtask/src YAML references with explicit caller classifications. The old kernel
alias-budget algorithm moved rather than being duplicated. Core identity exports are untouched,
so the anticipated core/lib.rs overlap with unit I did not occur. Extraction already used the
bounded core facade and needs regression execution, not an extra reader implementation.
Embedded code-name domains remain trusted compile-time inputs. The ESS comment is coordinator-owned.

## Corrected shared-loader inventory

Adversary pass one is recorded verbatim as review-result:adversary-input-08-yaml-pass-1. Root
verified its reach: a newly added ordinary shared-loader call escaped the source inventory;
no existing unsafe production caller was demonstrated. Tests were retained and committed before
correction. The implementor's saved correction was recovered after a host usage limit, with its
stopped state inspected and a separate root lease. No other lease was cleared.

Correction candidate bc41079b2956acc206d83efe9acf23afa62be8e5 changes only the inventory test file
relative to the committed adversary cases. Shared imports and reader uses now have explicit
caller classifications; extra calls using an already classified import are counted. The root
added a leading-underscore alias counterexample, observed it red and corrected that scanner edge.
This remains a conservative lexical inventory, not Rust control-flow proof. Original adversary
assertions and production source are unchanged.

Verification source: <cache>/ekr-next-three/input-08/yaml/correction-1/public-report.md and its
raw logs. The report records the recovered package run, then final changed-target, clippy and
format checks. VERIFIED for the pass-one finding: the same original counterexample now passes.
The final allowed adversary pass and the combined gate still determine unit admission.
