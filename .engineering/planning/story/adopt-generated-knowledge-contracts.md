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
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
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
- confidence: cited
  path: crates/ekr/tests/conformance.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
- confidence: inferred
  path: generated
- confidence: inferred
  path: systems/ekr
- confidence: inferred
  path: xtask
revision: 21
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

## Scope corrections from implementation

The worker inspected the actual files: .github/workflows/correctness.yml is the CI pin site; the earlier ci.yml path was inferred and incorrect. README.md and crates/ekr/tests/conformance.rs also bind the final ESS version/revision. crates/ekr/tests/story_contract.rs must explicitly admit generated-only dependencies alongside the SDK transitive closure guard; this preserves the no-kernel/store SDK boundary. Source comments in crates/ekr-integrate/src/lib.rs cite exact ESS line spans, and the existing test caught moved spans; the coordinator corrected the six citations without weakening the test.

The worker exhausted its execution allowance after implementing development adoption and starting touched-crate tests. The coordinator resumed the same dirty worktree and existing processes, preserving all generated artifacts and red-first evidence. No completed feature or release acceptance is inferred from this handoff. Publication still requires an actual verified generator release and refreshed conformance provenance.

Raw generator output includes a machine-local .ess-output/state.json ownership journal. It is excluded only as an explicitly recognized operational file, gitignored and checked against symlink/extra-entry substitution; all source/report/schema/manifest artifacts remain exact-byte checked. Rustfmt --all traversed nested generated path dependencies and changed their bytes; the drift gate caught it. The Rust xtask formatter now selects actual Cargo workspace members, preserving generated bytes without a manually maintained package list.

## Single release carrier coordination

The operator coordinated the existing ESS pull request #398 as the sole 0.52 release carrier and its integrator as the sole merge/tag owner. Our source branch fix/ekr-rust-generation is frozen at 26bef8f840032beb76b7cecb2eb939470011cf45, published with signed common evidence. Reviewed generator commits 09df87a6123a5ae50d752d2e3223fc19f596c9de and 126c2b3905d0f4279086b9d3030096147955dfec, sanitized adversary records, and final release tutorial corrections were handed off. ESS #403 is superseded only after all wanted source and evidence are published through #398. No duplicate merge, tag or full release gate will be started by this task. Adoption remains active until the sole carrier's released archives and exact checks are verified, then pinned and regenerated here.

The sole release integrator confirmed publication of the full frozen source history and sanitized evidence through ESS #398 at 5a5ac7f74d0ffdfcb9d91d395053620729b78254, with merge b0db254c2. ESS #403 was then closed by the bot as superseded. No release tag was reported at this handoff, so the verified-release pin remains pending. This task's completed ESS compiler outputs were reclaimed; its source worktree remains recoverable.

## Release handoff and bounded cleanup

The sole ESS integrator reported PR398 merged at 27b1ef507 with all 15 checks green and the entire frozen 26bef8f840032beb76b7cecb2eb939470011cf45 history preserved. PR403 is closed as superseded. The integrator retains release/tag ownership and reports publisher corrections in PR404 must land before tagging; no 0.52 release is claimed here.

The frozen ess-ekr-synthesis-publish-20261003 managed tree was finished and garbage-collected through the worktree CLI after confirming remote recovery and preserving raw evidence outside build outputs. Exact completed disposable directories accounted for 769302528 allocated bytes reclaimed (0.716 GiB); the earlier 18.3 GB inspection was stale. Live EKR caches and other owners' trees were untouched. Source changes remain recoverable on ESS main. EKR generator/release adoption remains active until the exact released artifact and final regeneration are verified.

## AEP report-reader compatibility pin

The actual ESS suite/13 reports exposed AEP 0.64's report ingestion limit. Published AEP 0.68.0 (tag commit 6d7a44d3607d2d9a6ffdf0a165993c546c43d0db, release published 2026-09-30) admits later ESS coverage-suite versions with original-byte and complete-parent checks; it leaves execution grammar to ESS. The x86_64 Linux archive was fetched from the official release and verified against both the release asset digest and SHA256SUMS: 02bf6a2c4bc9a3ffd717edfb365f665491005004f159774a8847d3cbd18c5e91. The extracted CLI reports aep 0.68.0 and validates the existing project/5 store without migration. Update the EKR local/CI tool pin and documentation together; retain the protocol source and artifact lifecycle. This closes a necessary evidence-reader prerequisite, not the held ESS publication decision.
