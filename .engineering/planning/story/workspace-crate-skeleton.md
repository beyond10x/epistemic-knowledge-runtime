---
format: aep.planning-md/1
id: story:workspace-crate-skeleton
kind: story
status: active
title: 'P1 crate skeleton: five empty workspace members'
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/ekr-core/Cargo.toml
- confidence: inferred
  path: crates/ekr-core/src/lib.rs
- confidence: inferred
  path: crates/ekr-graph/Cargo.toml
- confidence: inferred
  path: crates/ekr-graph/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/Cargo.toml
- confidence: inferred
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-ontology/Cargo.toml
- confidence: inferred
  path: crates/ekr-ontology/src/lib.rs
- confidence: inferred
  path: crates/ekr-store/Cargo.toml
- confidence: inferred
  path: crates/ekr-store/src/lib.rs
- confidence: inferred
  path: crates/ekr/Cargo.toml
- confidence: inferred
  path: crates/ekr/src/main.rs
revision: 8
---
## Context

Every P1 story lands in its own crate. The two files they would all otherwise touch are the
workspace manifest and the lockfile. This story creates the six P1 crates empty, wires the
dependency edges between them, and declares every external dependency the P1 stories will use,
per crate, so that `Cargo.lock` is complete after this story and no later P1 story edits
`Cargo.toml` at the root or regenerates the lockfile.

The crate layering is acyclic by construction. The design's § 68 sketch is a module tree inside
one crate, where references run both ways; as separate crates the primitives every layer names —
identifiers, content hashes, canonical encoding — must sit below the graph, and the transaction
boundary that reads the graph must sit above it. Hence `ekr-core` at the bottom and `ekr-kernel`
above `ekr-graph` (review-result `p1-design-round-2`).

## Acceptance

`task check` exits zero with `ekr-core`, `ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store`
and `ekr` present as workspace members.

## Constraints the scope carries

- Each crate has a crate-level doc comment naming the ESS domain it implements
  (`systems/ekr/domains/<domain>.yaml`; `ekr-core` and `ekr-kernel` both name `kernel`);
  `missing_docs` under clippy `-D warnings` makes this part of the acceptance's exit code.
- No code beyond the doc comment and, for `ekr`, an empty clap `main`.
- Crate dependency edges, declared now:
  - `ekr-core`: none
  - `ekr-ontology`: `ekr-core`
  - `ekr-graph`: `ekr-core`, `ekr-ontology`
  - `ekr-kernel`: `ekr-core`, `ekr-ontology`, `ekr-graph`, `ekr-store`
  - `ekr-store`: `ekr-core`, `ekr-graph`
  - `ekr`: all five
- External dependencies, declared now per crate, even though unused until later stories:
  - `ekr-core`: `uuid` (v7), `sha2`, `hex`, `serde`, `serde_json`, `thiserror`; dev `proptest`
  - `ekr-ontology`: `serde`, `serde_json`, `serde_yaml`, `thiserror`; dev `proptest`
  - `ekr-graph`: `serde`, `thiserror`; dev `trybuild`
  - `ekr-kernel`: `serde`, `serde_json`, `serde_yaml`, `thiserror`; dev `proptest`, `trybuild`
  - `ekr-store`: `eventlog-core`, `eventlog-sqlite`, `eventlog-file` (git tag, read from
    `beyond10x/eventlog`'s `Cargo.toml`), `serde`, `serde_json`, `thiserror`; dev `tempfile`
  - `ekr`: `clap`, `serde_json`; dev `assert_cmd`, `tempfile`
- A later P1 story that needs a dependency not listed here adds `Cargo.lock` to its scope and
  says so, which makes the collision visible to `aep plan artifact waves`.

## Scope

- `Cargo.toml`, `Cargo.lock`
- `crates/ekr-core/{Cargo.toml,src/lib.rs}`
- `crates/ekr-kernel/{Cargo.toml,src/lib.rs}`
- `crates/ekr-ontology/{Cargo.toml,src/lib.rs}`
- `crates/ekr-graph/{Cargo.toml,src/lib.rs}`
- `crates/ekr-store/{Cargo.toml,src/lib.rs}`
- `crates/ekr/{Cargo.toml,src/main.rs}`

## Notes

Component map: `systems/ekr/components.yaml`; the `ekr-kernel` component is two crates,
`ekr-core` (types) and `ekr-kernel` (commands), for the layering reason above. Every crate opts
into `[lints] workspace = true`.
