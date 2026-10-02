---
format: aep.planning-md/3
id: task:every-yaml-reader-is-bounded
kind: task
status: draft
title: Every YAML reader that takes outside input is bounded
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- derived_from: task:seed-document-bounds-alias-expansion
revision: 1
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
