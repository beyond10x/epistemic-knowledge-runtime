---
format: aep.planning-md/3
id: task:seed-document-bounds-alias-expansion
kind: task
status: implemented
title: ekr seed bounds the size and alias expansion of its document
relations:
- serves: vision:o5
- decomposes: epic:p6-maintenance-observability
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-02T10:29:26Z", actor: "agent:codex-ekr-x7b", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Context

Found by the implementor of unit S in wave p2p3p4-03 while fixing
`review-result:adversary-p2p3p4-03-resolve-pass-1` F1 (a YAML alias bomb in `ekr resolve`). The
implementor reports, and the coordinator has not re-read:

- `ekr propose` walks aliases through the `observation` facade
  (`crates/ekr-kernel/src/document/shape.rs`) and charges each expansion to a budget, so its
  expansion is bounded.
- `ekr seed`: `SeedDocument::from_yaml` is a plain `serde_yaml_ng::from_str`, and
  `crates/ekr/src/cli/seed.rs:22-25` reads the document with no size limit, so a seed document can
  expand aliases without bound.

Who reaches it: whoever runs `ekr seed` on a document they did not write. A seed runs once per
store.

Wave extract-06 unit D (2026-10-01, `review-result:adversary-extract-06-d-pass-1`) added an opt-in
depth bound to the vendored YAML loader, `Documents::from_str_within_depth`
(`vendor/serde_yaml_ng`), which stops the parser at the first container past the limit, before the
full load; the extraction reader uses it at depth 32 (`DOCUMENT_V2_LIMITS`). The kernel's
transaction-document reader checks depth only after loading, so the quadratic load still runs
before its refusal. Both readers of this task use the same bounded loader.

## Acceptance

- A seed document that uses YAML aliases to decode past a stated limit, or that is itself over a
  stated size, is refused with a named error before anything is committed, and the limit is stated
  in `docs/cli.md`.
- A seed or transaction document nested past a stated depth is refused before it is loaded (the
  YAML loader is quadratic in flow nesting depth: 120 KB took 9.6 s,
  `review-result:adversary-p2p3p4-03-resolve-pass-2` F1; `ekr resolve` already refuses depth 64).

