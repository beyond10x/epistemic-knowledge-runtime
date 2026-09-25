# Epistemic Knowledge Runtime

A knowledge store that keeps what it knows typed, evidence-backed and revisable. You declare a
schema — node types, edge types, typed properties, lifecycles — and record knowledge against it as
assertions, each citing the evidence it rests on and the time it is true. Every change is a
transaction that an agent proposes, a deterministic validator checks and only then is committed as
a new immutable revision, so nothing reaches the canonical state unchecked and every state can be
replayed and explained.

The `ekr` binary is the way in. It is built for agents as much as for people: every verb prints
JSON, every refusal has a name, and the binary documents its own workflow (`ekr guide`).

It is the third generation of the organisation-memory line: `company-brain` (v1, Python over a
markdown store) and `org-brain` (v2, a Rust engine on the entity-runtime kernel) came before it.
What they proved necessary and how their data enters this runtime is written down in
[`docs/predecessors.md`](docs/predecessors.md).

## Status

P1 is released as 0.0.2: the kernel, the typed graph and ontology, the store and the `ekr` binary.
Product crates: `ekr-core`, `ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store`, and
the `ekr` binary.

Repository utility: `xtask`.

| works in 0.0.2 | not yet |
|---|---|
| a schema of node types, edge types, typed properties, lifecycles and named operations, declared in the seed | changing the schema after seeding: `DefineNodeType`, `DefineEdgeType`, `ModifyProperty` and `MergeEntity` are refused as `unsupported-operation` |
| propose → validate → commit of nodes, edges, property updates, assertions, retractions, supersessions and named operations | adding evidence after seeding: in P1 evidence enters only through the seed |
| reads: snapshots at any revision, what is believed at a valid time, and the full explanation of an assertion | property constraints, operation preconditions and emitted events: declared, but writes touching them are refused |
| file and SQLite storage, with the kernel's executable specification passing on both | ingestion, the incubation forest, schema evolution and maintenance: later phases of [`docs/roadmap.md`](docs/roadmap.md) |

On the file provider every read re-hashes the whole event log, so commands slow down as revisions
accumulate; SQLite grows about linearly. [`CHANGELOG.md`](CHANGELOG.md) has the details.

## Install

```console
cargo build --release -p ekr
```

The binary is `target/release/ekr`; put it on your `PATH`. [`docs/cli.md`](docs/cli.md#install)
covers configuration.

## First run

The binary prints example documents that fit together: a host document naming the operator and the
validator, a seed with a small schema and graph, and a transaction adding one assertion.

```console
ekr example ekr.cli-host/1 > host.json
ekr example ekr-seed/2 > seed.yaml
ekr example ekr.transaction-document/1 > change.yaml
export EKR_HOST=host.json EKR_STORE=./store EKR_BACKEND=file
ekr seed seed.yaml                                          # revision 0
ekr propose change.yaml                                     # records the transaction
ekr validate 00000000-0000-4000-8000-000000000601           # "kind": "Validated"
ekr commit 00000000-0000-4000-8000-000000000601             # "kind": "Committed", revision 1
ekr explain 00000000-0000-4000-8000-000000000501            # the assertion, its evidence and history
```

## Where to read next

| read | for |
|---|---|
| [`docs/cli.md`](docs/cli.md) | the CLI reference: configuration, every verb, the seed and transaction formats, how to design a schema, a worked example from schema to committed assertion, and the common refusals |
| `ekr guide`, `ekr operations`, `ekr example <format>` | the same workflow from the binary itself |
| [`docs/epistemic-knowledge-runtime-design.md`](docs/epistemic-knowledge-runtime-design.md) | the architecture: kernel, ontology, canonical core, incubation forest, observation layer, frontier, maintenance, with dated amendments |
| [`docs/roadmap.md`](docs/roadmap.md) | phases P0–P7, the crate map, exit evidence, decisions D1–D3 |
| [`docs/predecessors.md`](docs/predecessors.md) | what v1 and v2 hold that the design must not lose; schema conflicts and the import policy |
| [`CHANGELOG.md`](CHANGELOG.md) | what each release changed |

## Build and check

For contributors.
Rust 1.91 is the workspace's declared minimum; builds use the toolchain pinned in
`rust-toolchain.toml`. The gate also needs [`task`](https://taskfile.dev), ESS 0.32.0 and AEP 0.57.0.

```console
task check
```

[`AGENTS.md`](AGENTS.md) is the contract for changing this repository: its invariants, the gate and
what must not happen here.

## Layout

```text
Cargo.toml              the workspace; every crate opts into [workspace.lints]
crates/                 the six P1 crates; ekr is the binary, the rest are libraries
xtask/                  repository tasks that are not the product
docs/                   the CLI reference, design, roadmap, predecessor analysis
systems/                ESS domains, one per crate
.engineering/           AEP project file and planning store
.github/workflows/      the shared source gate
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).
