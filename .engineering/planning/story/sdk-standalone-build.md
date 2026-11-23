---
format: aep.planning-md/3
id: story:sdk-standalone-build
kind: story
status: active
title: The SDK builds in a standalone application
summary: ekr-core depends on a YAML crate that carries the API it uses, checked by a standalone build outside the test step (review 0.0.42 finding 34)
relations:
- serves: vision:o5
- delivers: task:sdk-consumer-yaml-dependency
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T20:20:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T20:20:13Z", actor: "human:timo", revision: 3}
---
## Outcome

A fresh Rust application builds the released EKR SDK with only its declared dependencies.

## Work

Review of 0.0.42, finding 34 (and task `sdk-consumer-yaml-dependency`): a standalone application's
`cargo check` exits 101 because the registry version of `serde_yaml_ng` lacks the observation
module two `ekr-core` imports use; EKR builds only with its bundled YAML crate.

- Give `ekr-core` a dependency that carries the API it uses (publish or vendor the patched YAML
  crate under its own name, or stop using the missing module).
- A check builds a standalone workspace that depends on the SDK the way an external user would
  (path or packaged crate, no workspace patch), offline from the vendored registry, outside the
  120 s test step (its own gate step or the `slow` lane).

## Acceptance

The standalone check is red on main and green after the change; the SDK's public API is unchanged.
