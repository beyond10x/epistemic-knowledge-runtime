---
format: aep.planning-md/2
id: task:seed-document-bounds-alias-expansion
kind: task
status: draft
title: ekr seed bounds the size and alias expansion of its document
relations:
- serves: vision:o5
revision: 2
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

## Acceptance

A seed document that uses YAML aliases to decode past a stated limit, or that is itself over a
stated size, is refused with a named error before anything is committed, and the limit is stated
in `docs/cli.md`.


## Also: nesting depth

The YAML loader is quadratic in flow nesting depth inside a block mapping (`review-result:adversary-p2p3p4-03-resolve-pass-2` F1: 120 KB took 9.6 s, 400 KB 402 s). `ekr resolve` refuses a document nested deeper than 64 levels before loading it; the seed and propose readers do not yet. The acceptance extends to: a seed or transaction document nested past a stated depth is refused before it is loaded.
