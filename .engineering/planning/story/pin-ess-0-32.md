---
format: aep.planning-md/1
id: story:pin-ess-0-32
kind: story
status: implemented
title: Pin ESS 0.32.0 for the specification and the conformance target
relations:
- serves: vision:o2
scope:
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr/src/conformance.rs
- confidence: inferred
  path: crates/ekr/tests/adversary_p1_14_conformance.rs
- confidence: inferred
  path: crates/ekr/tests/adversary_p1_14_conformance_r2.rs
- confidence: inferred
  path: crates/ekr/tests/fixtures/conformance/scenarios
- confidence: cited
  path: systems/ekr/conformance
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 16
---
## Context

ESS 0.32.0 is released (2026-09-25). EKR pins ESS at git rev `8bef63a` (0.29.0): the `ess` CLI in CI and `task check`, and the `ess-conformance` / `ess-primitives` crates the conformance target links.

## Acceptance

- CI, `task check` and the conformance crates pin ESS 0.32.0 by one exact rev, the same everywhere.
- `systems/ekr` validates under 0.32.0; the kernel suite is re-synthesized from the checked-in specification and `conform-check` passes on both providers with 0 failed, error, unsupported or skipped; the baseline records the new counts.
- Any contract change 0.32.0 requires is a measured correction to `systems/ekr/`, not a weakening.
- `task:current-revision-ranking-is-unsynthesizable` and `task:conformance-cannot-observe-command-responses` are re-checked against 0.32.0 and their bodies say what changed.

## Scope

Derived 2026-09-25 by `story-scoper` (wave p1-15). Confidence: medium; the pin sites are read from the tree, suite bytes under 0.32.0 are not known until it runs.

- `Cargo.toml:29-30` (`ess-conformance`, `ess-primitives` at rev `8bef63a2`), `Cargo.lock:400-456` — cited; at 0.32.0 `ess-primitives` takes `serde_yaml` as a normal dependency
- `.github/workflows/correctness.yml:51,55` (cache key `ess-0.29.0`, `cargo install --rev`) — cited
- `Taskfile.yml:63-73` (`spec-check`, `conform-check` run the `ess` on PATH; no version pinned) — cited
- `systems/ekr/conformance/{provenance,suite,baseline}.json`, `README.md:40` — cited
- `ess_conformance` target/scenario API: no diff between 0.29.0 and 0.32.0; suite format stays `ess-conformance/13` — cited
- `crates/ekr/src/conformance.rs:29`, two adversary doc comments naming 0.29.0 — inferred
- `systems/ekr/domains/kernel.yaml` only if 0.32.0 Timestamp ordering changes (commits `b24fcc6eec`, `423b36f42a`) force a correction — inferred
- task bodies `current-revision-ranking-is-unsynthesizable`, `conformance-cannot-observe-command-responses` re-checked through `aep plan artifact body` — cited
- 0.32.0 tag resolves to `f4c1bb84298e75650674c028f9995b2324f79c06` — cited
