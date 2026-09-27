---
format: aep.planning-md/1
id: story:workspace-crate-skeleton
kind: story
status: implemented
title: 'P1 crate skeleton: six empty workspace members'
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/ekr-core/Cargo.toml
- confidence: cited
  path: crates/ekr-core/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/Cargo.toml
- confidence: cited
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-kernel/Cargo.toml
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: cited
  path: crates/ekr-ontology/Cargo.toml
- confidence: cited
  path: crates/ekr-ontology/src/lib.rs
- confidence: cited
  path: crates/ekr-store/Cargo.toml
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: cited
  path: crates/ekr/src/main.rs
- confidence: cited
  path: crates/ekr/tests/adversary_docs_contract.rs
- confidence: cited
  path: crates/ekr/tests/msrv_contract.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
revision: 19
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

- Each library crate has a crate-level doc comment naming the ESS domain it implements
  (`systems/ekr/domains/<domain>.yaml`; `ekr-core` and `ekr-kernel` both name `kernel`); the
  binary `ekr` names `systems/ekr/components.yaml`, the composition it runs. `missing_docs`
  under clippy `-D warnings` makes the comment's presence part of the acceptance's exit code; a
  contract test makes its content part.
- No code beyond the doc comment and, for `ekr`, an empty clap `main`.
- Crate dependency edges, declared now:
  - `ekr-core`: none
  - `ekr-ontology`: `ekr-core`
  - `ekr-graph`: `ekr-core`, `ekr-ontology`
  - `ekr-kernel`: `ekr-core`, `ekr-ontology`, `ekr-graph`, `ekr-store`
  - `ekr-store`: `ekr-core`, `ekr-graph`, `ekr-ontology` (widened in wave p1-05, see below)
  - `ekr`: `ekr-core`, `ekr-graph`, `ekr-kernel`, `ekr-ontology` — **four, not five**; it lost its
    `ekr-store` edge in wave p1-06, see below
- External dependencies, declared now per crate, even though unused until later stories:
  - `ekr-core`: `uuid` (v7), `sha2`, `hex`, `serde`, `serde_json`, `thiserror`; dev `proptest`
  - `ekr-ontology`: `serde`, `serde_json`, `serde_yaml_ng`, `thiserror`; dev `proptest`
  - `ekr-graph`: `serde`, `thiserror`; dev `trybuild`
  - `ekr-kernel`: `serde`, `serde_json`, `serde_yaml_ng`, `thiserror`; dev `proptest`, `trybuild`,
    `tempfile` (added in wave p1-06, see below), `eventlog-core`, `eventlog-file`,
    `eventlog-sqlite`, `time`, `tokio` (added by `cb6dc41` for direct provider corruption
    witnesses; dev only, so the runtime gains no edge)
  - `ekr-store`: `eventlog-core`, `eventlog-sqlite`, `eventlog-file` (immutable Git revision
    `4ee3dc23f0d02a5726a0e41d097477791f09efe2`), `serde`,
    `serde_json`, `thiserror`, `time`, `tokio` (the last two widened in wave p1-05, see below); dev
    `tempfile`
  - `ekr`: `clap`, `serde`, `serde_json`, `time` (the last two added by `7444af5` for strict CLI
    host configuration and valid-time selectors, `task:strict-cli-host-input`); dev `assert_cmd`,
    `tempfile`
- `rust-version` is `1.91`, the minimum the pinned eventlog source requires (found by the adversary,
  pass 1; the story said nothing about the floor before).
- A later P1 story that needs a dependency not listed here adds `Cargo.lock` to its scope and
  says so, which makes the collision visible to `aep plan artifact waves`.

## Scope

As implemented and confirmed by the implementor's report and the adversary's two passes (wave
p1-01, merge `d5b163b`). Every line the draft inferred was confirmed; the lines marked *added*
were not in the draft.

- `Cargo.toml`, `Cargo.lock` — confirmed
- `crates/ekr-core/{Cargo.toml,src/lib.rs}` — confirmed
- `crates/ekr-kernel/{Cargo.toml,src/lib.rs}` — confirmed
- `crates/ekr-ontology/{Cargo.toml,src/lib.rs}` — confirmed
- `crates/ekr-graph/{Cargo.toml,src/lib.rs}` — confirmed
- `crates/ekr-store/{Cargo.toml,src/lib.rs}` — confirmed
- `crates/ekr/{Cargo.toml,src/main.rs}` — confirmed
- `README.md` — *added*: the unit changed the toolchain floor, the member list and the layout,
  and README states all three (adversary pass 2, findings 1–2)
- `crates/ekr/tests/msrv_contract.rs` — *added* by the adversary: the lockfile's `rust_version`
  ceiling against the declared floor
- `crates/ekr/tests/story_contract.rs` — *added* by the adversary, hardened in two correction
  rounds: members, edges, external dependencies and their qualifiers, lints opt-in, no manifest
  comment, no Rust source reading the planning store
- `crates/ekr/tests/adversary_docs_contract.rs` — *added* by the adversary: README against the
  workspace, doc comments against `systems/ekr`

## Notes

Component map: `systems/ekr/components.yaml`; the `ekr-kernel` component is two crates,
`ekr-core` (types) and `ekr-kernel` (commands), for the layering reason above. Every crate opts
into `[lints] workspace = true`. Known weak case, filed as `task:readme-status-case`:
`the_readme_status_matches_the_workspace_members` asserts the absence of three stale phrases
rather than comparing README's Status to the member list.


## Amended on 2026-09-21, in wave p1-05

`ekr-store`'s two dependency lines above are wider than this story left them, and the tables in
`crates/ekr/tests/story_contract.rs` move with them in the same change, as that file's own doc
requires.

`architecture-decision-record:0006-ekr-store-bridges-the-async-port` and its amendment carry the
reasons. In short: `eventlog-core`'s `EventStore` trait is async and its providers need a tokio
runtime context, so `tokio` is the runtime `ekr-store` owns; `time` is
`eventlog_core::CommandMeta.occurred_at`, a `time::OffsetDateTime` on a struct with no constructor,
no `Default` and no builder, which no consumer can build without naming the crate; and
`ekr-ontology` is `CanonicalGraph.ontology`, which `RevisionLog::fold` returns.

Neither of the first two adds a package to `Cargo.lock` beyond `tokio` itself — `time 0.3.55` was
already resolved transitively. The crate edge is inside the acyclic order `docs/roadmap.md` § 3
gives and adds no cycle.

This story is `implemented` and stays so. Amending a closed story's constraint table is the right
move here rather than leaving it wrong: `story_contract.rs` reads these tables as ground truth, so
a stale table is a red gate for whoever touches the manifest next.


## Amended again on 2026-09-21, in wave p1-06

Two more of this story's constraint lines moved, and `crates/ekr/tests/story_contract.rs` moves with
them in the same change, as that file's own doc requires.

**`ekr` no longer declares `ekr-store`.** `architecture-decision-record:0007-the-commit-path-is-the-kernels`
repairs `AGENTS.md` invariant 1, whose second half — "only a `ValidatedTransaction` commits; no other
crate holds a writer to canonical state" — was carried by nothing. An independent review measured a
commit landing at revision 1 in a process that could not link `ekr-kernel`, on three hand-written
events through the public `RevisionLog::append`, with `ValidatedTransaction` holding zero consumers
in any `src/`.

The store sits **below** the kernel, so its writer cannot take a `ValidatedTransaction`, and a sealed
trait in the store would exclude the kernel along with everybody else. So the guarantee is stated
where it can hold: the binary reaches persistence only through `ekr-kernel`, and that is a property
of this edge table, read by a case rather than by the compiler.

**`ekr-kernel` gains `tempfile` as a dev-dependency.** `ekr_store::Fold` is `pub(crate)`, so an
in-memory store the kernel could hold cannot answer `head` or `fold`, and a case over one would
assert nothing. `tempfile` is already this workspace's temporary directory, in `ekr-store` and `ekr`.

This story is `implemented` and stays so. Amending a closed story's constraint table is right here
for the reason wave p1-05 gave: `story_contract.rs` reads these tables as ground truth, so a stale
table is a red gate for whoever touches a manifest next.
