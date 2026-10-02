---
format: aep.planning-md/3
id: task:sdk-consumer-yaml-dependency
kind: task
status: draft
title: The released SDK builds in an external consumer without a workspace patch
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 1
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
