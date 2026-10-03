---
format: aep.planning-md/3
id: story:adopt-generated-knowledge-contracts
kind: story
status: active
title: Adopt verified generated knowledge contracts and gate drift
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- derived_from: release-plan:knowledge-inbox-schema-learning
scope:
- confidence: inferred
  path: .github/workflows/ci.yml
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr-core
- confidence: inferred
  path: crates/ekr-integrate
- confidence: inferred
  path: crates/ekr-sdk
- confidence: inferred
  path: crates/ekr-store
- confidence: inferred
  path: crates/ekr-views
- confidence: inferred
  path: generated
- confidence: inferred
  path: systems/ekr
- confidence: inferred
  path: xtask
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T00:11:28Z", actor: "agent:codex-ekr-knowledge", revision: 13, decided_on: {"recorded":{"approval":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T00:11:28Z", actor: "agent:codex-ekr-knowledge", revision: 14, decided_on: {"recorded":{"approval":1}}}
---
## Outcome

Adopt generated EKR runtime contracts from a verified released ESS build after the optional-recursion/adopter-compilation prerequisite. Pin that exact release and make regeneration drift a Rust gate. This is infrastructure for A–F, not feature acceptance.

## Contract source

Contract commit 07604bf1a5e6c19ef86b78846dc99a603f3d2bfb validated before this story. Its final spec_digest is 29e2f5d0af0bb99248bb46d0dc1f538dedab7d5d209ca07c0fce5d0ac60a1a4e. Preserve all existing serialized labels via explicit enum name/wire declarations.

## Design

Use ESS synthesis workspace output under generated/ekr-contracts and depend on its ekr-types crate under alias ekr-contracts; no generated manifest edits. Use the existing generate types --target rust --all-types --package ekr-contract-data for serde named documents under generated/ekr-contract-data. Exclude nested workspaces. Do not invent HTTP bindings to obtain codecs; no handwritten duplicate models. Named persisted document projections missing from the specification must first be added and validated there.

The published 0.51 data-library probe emitted 409 types, compiled and round-tripped receipt/recursive data. This does not discharge the unreleased synthesis prerequisite. Keep types-report obligations: serde decoding alone does not check UUID/hash/base64, integral Number or exclusive unions. Checked adapters enforce them and runtime semantics.

## Scope

Cited by author_contracts adoption review: Cargo.toml, xtask/src/main.rs, Taskfile.yml, .github/workflows/ci.yml and systems/ekr pins; structured-enum readers in crates/ekr-integrate/tests/extraction.rs:347, crates/ekr-store/tests/domain_projection.rs:579, crates/ekr-views/tests/code_names.rs:76 and src/code_names.rs:109. Identity guard crates/ekr-core/tests/identity_serde.rs:293 requires exact legacy-plus-generated ID coverage, retaining legacy mint/serialization checks. SDK guard crates/ekr-sdk/tests/document_drift.rs:610 explicitly admits only generated data alongside core and proves no transitive kernel/store/runtime. Inferred generated directories, adapter helpers and Rust drift tests. Existing conformance provenance is regenerated without inflating actual execution counts.

## Acceptance

Rust/clap xtask contracts-check checks the exact ESS version, regenerates both complete trees into task-owned temporary directories, compares exact generator-owned paths and bytes, and compiles synthesis. Missing, changed and extra generated files fail. No hand-edited generated manifest or source. Pins/checksums match a verified published release and downloaded artifact. Existing serialized labels and data roundtrips remain unchanged. SDK architecture guard and generated ID coverage pass. Record generator plan/report obligations; full task check passes before adoption is complete.

## Authorization

The operator explicitly requested the generator prerequisite, verified release pin, regeneration gate and generated runtime contracts in the execution plan on 2026-10-03.
