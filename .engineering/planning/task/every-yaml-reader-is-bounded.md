---
format: aep.planning-md/3
id: task:every-yaml-reader-is-bounded
kind: task
status: implemented
title: Every YAML reader that takes outside input is bounded
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:seed-document-bounds-alias-expansion
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T21:12:11Z", actor: "agent:codex-ekr-next-three-root", revision: 10, decided_on: {"recorded":{"test_result":2,"review_outcome":1,"verification":2}}}
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

Confirmed from the implementor report-public.md, recovered correction public-report.md and final
adversary public-report.md: crates/ekr-ontology/src/schema.rs adds the ontology preflight;
crates/ekr-ontology/tests/ontology_load.rs holds its bounds and compatibility cases;
crates/ekr-core/src/decode.rs and crates/ekr-core/src/decode/yaml.rs own the shared bounded
loader; crates/ekr-core/tests/decode.rs holds shared decoding regressions;
crates/ekr-kernel/src/yaml.rs delegates while retaining its depth-event test wrapper;
crates/ekr-ontology/tests/yaml_ingress.rs inventories production readers and guards new calls,
imports and aliases. This inventory is conservative lexical analysis, not Rust control-flow proof.

The earlier inferred shared decoder and inventory test are now implemented at those exact paths.
The old kernel alias-budget algorithm moved rather than being duplicated. The anticipated
core/lib.rs overlap did not occur. Extraction already used the bounded core facade and required
regression execution, not a new reader implementation. Embedded code-name domains remain trusted
compile-time inputs. systems/ekr/domains/ontology.yaml is coordinator-owned shared specification.
No retained format, domain noun or dependency version changed.

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

## Final combined verification

The earlier pending paragraphs record intermediate states; this section supersedes their
admission status. Final reviewed unit heads: YAML eb807fd7e70c73b388d805f733874300170a66f6 and identity 14e43ddd7c4949212aab25dd01e257844641e624.
They are merged in combined source candidate 5ab56194a7ef25965ffc5d4eb790080e235f1708.
All task-check steps applicable there exited zero, recorded individually under
<cache>/ekr-next-three/coordinator/input-08-gate. This includes workspace tests and bench-feature
compilation, format, clippy, rustdoc, vendor checks, ESS validation, conformance freshness and
planning validation. The existing ignored cases include measurements, subprocess helpers and
known defects; this wave changes no ignore annotation and claims none of those defects fixed.

Current-main documentation was subsequently merged at 401a56d43e4b617a078a0dda7b384c4f7a1ef908. Supplementary format,
workspace all-target clippy, cross-crate story_contract/public_surface/docs_cli/agent_cli/
temporal_reads, xtask tests, YAML ingress inventory, rustdoc and the newly added site-check all
exited zero under <cache>/ekr-next-three/coordinator/input-08-supplement. Required CI will run
the complete current task check on the published candidate. Source gate logs remain retained.

The focused baseline/treatment claim is VERIFIED by the previously cited red and green logs.
Final adversary records are review-result:adversary-input-08-yaml-pass-2 and
review-result:adversary-input-08-identity-pass-1. They report no remaining findings. The YAML
pass-one inventory finding is corrected and has its fixed review_outcome. No further attack ran.
Public integration and cleanup remain coordinator responsibilities before reporting the release
checkpoint ready. No version change or tag is part of these implementation moves.

Measured test-result totals below sum report lines (including subprocess executions); they are
executions, not unique test identities:
- Combined workspace log: 2138 passed, 0 failed, 14 ignored.
- Vendor log: 183 passed, 0 failed, 0 ignored.
- Supplementary test logs: 98 passed, 0 failed, 0 ignored.
