---
format: aep.planning-md/3
id: task:sdk-consumer-yaml-dependency
kind: task
status: active
title: The released SDK builds in an external consumer without a workspace patch
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
- decomposes: epic:zero-open-issues-2026-11
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:55:00Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
---
# The released SDK builds in an external consumer

## Observed failure

An external Cargo workspace depending on `ekr-sdk` at release tag 0.0.26 (commit 4832d892e7586f6cc7980fa6f99634e3c9b71356) fails to compile:

```text
error[E0432]: unresolved import `serde_yaml_ng::observation`
crates/ekr-core/src/decode.rs:106:24
could not find `observation` in `serde_yaml_ng`
```

The engine workspace patches serde_yaml_ng to its bundled implementation. Cargo does not inherit a dependency workspace's `[patch.crates-io]`, so external consumers resolve the registry package, which lacks the observation module. Installing the CLI from the release works because its root workspace supplies the patch.

An external consumer builds after selecting serde_yaml_ng from the same release tag in its own `[patch.crates-io]`. That is a temporary consumer workaround, not the desired public SDK contract.

## Scope

Dependency packaging in Cargo.toml, crates/ekr-core/Cargo.toml, crates/ekr-sdk/Cargo.toml and the bundled YAML package; an external-workspace build test. No application data or store-format changes are required.

## Done when

- A fresh minimal external Cargo workspace depending on the released SDK can run cargo check without copying private workspace patches.
- The bounded YAML observation implementation remains in use and its existing size, depth and alias-expansion tests remain green.
- A regression check builds the SDK from outside this workspace, so workspace-only dependency overrides cannot hide the failure.

This task records a generic integration requirement only. No engine implementation is supplied by this report.

## Surface (story-scoper, 2026-11-09)

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** workspace dependency packaging of `serde_yaml_ng` — root `Cargo.toml:51` (registry dep) and `Cargo.toml:67-68` (`[patch.crates-io]` to `vendor/serde_yaml_ng`) — cited
- **Files:** `Cargo.toml`, `crates/ekr-core/Cargo.toml:15`, `crates/ekr-sdk/Cargo.toml:14`, `vendor/serde_yaml_ng/Cargo.toml` — cited
- **Symbols:** `serde_yaml_ng::observation` at `crates/ekr-core/src/decode.rs:108`, `crates/ekr-core/src/decode/yaml.rs:14`, `crates/ekr-kernel/src/yaml.rs:4`, `crates/ekr/src/cli/resolve.rs:149` — cited
- **Also likely:** `Cargo.lock` (any re-sourcing or rename of the package rewrites it) — inferred
- **Also likely:** the eight other manifests with `serde_yaml_ng.workspace = true` (ekr, ekr-graph, ekr-integrate, ekr-kernel, ekr-observe, ekr-ontology, ekr-store, ekr-views) and every `use serde_yaml_ng` site, only if the fix renames the package — inferred
- **Also likely:** a new external-workspace build check as a new test under `crates/ekr-sdk/tests/` or an `xtask` subcommand, wired into `Taskfile.yml` beside the vendor gates at lines 69-81 — inferred
- **Also likely:** `crates/ekr/tests/story_contract.rs` (pins dependency tables and mentions `patch.crates-io`), `crates/ekr-ontology/tests/yaml_ingress.rs` (token filter on `serde_yaml_ng`) — inferred
- **Documents:** `vendor/serde_yaml_ng/UPSTREAM.md`, `CHANGELOG.md`; consumer instructions in `README.md` or `docs/` if they describe the workaround — inferred
- **Confidence:** medium — the story names the manifests and the tree confirms the defect site, but the fix shape (git or path dependency, renamed package, or moving the facade in-tree) is undecided and sets how wide the edit goes
- **Would collide with:** any unit editing root `Cargo.toml` `[workspace.dependencies]` or `[patch.crates-io]`, `Cargo.lock`, `vendor/serde_yaml_ng/`, the `Taskfile.yml` check tasks, or (under a rename) any YAML-decoding source in the ten crates — inferred
- **Safety fact:** the bundled implementation and its tests (`vendor/serde_yaml_ng/src/observation.rs`, `tests/test_observation.rs`) need no behaviour change, since only how Cargo selects the package changes; no store or wire format is touched — step 2 (`Cargo.toml:67-68`), unproven, inferred
