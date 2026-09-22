# Epistemic Knowledge Runtime

A self-maintaining knowledge runtime. It continuously ingests observations, interprets them into
structured knowledge, validates and integrates what is trustworthy into a canonical core, parks
what does not yet fit in an incubation forest, evolves its ontology under controlled rules, and
forgets or compacts data over time.

It is the third generation of the organisation-memory line: `company-brain` (v1, Python over a
markdown store) and `org-brain` (v2, a Rust engine on the entity-runtime kernel) came before it.
What they proved necessary and how their data enters this runtime is written down in
[`docs/predecessors.md`](docs/predecessors.md).

## Status

P1 is in progress. Product crates: `ekr-core`, `ekr-kernel`, `ekr-ontology`,
`ekr-graph`, `ekr-store`, and the `ekr` binary.

Repository utility: `xtask`.

The workspace implements typed graphs and ontology, deterministic transaction validation,
kernel-validated seed initialization through both storage backends, and original-format
verification. Transaction application and replay, the completed CLI and conformance remain
in progress. The build order and phase exit evidence are in
[`docs/roadmap.md`](docs/roadmap.md). Work is planned in `.engineering/planning/`
through the AEP CLI.

## Documents

| document | what it is |
|---|---|
| [`docs/epistemic-knowledge-runtime-design.md`](docs/epistemic-knowledge-runtime-design.md) | the architecture: kernel, ontology, canonical core, incubation forest, observation layer, frontier, maintenance, with dated amendments |
| [`docs/roadmap.md`](docs/roadmap.md) | phases P0–P7, the crate map, exit evidence, decisions D1–D3 |
| [`docs/predecessors.md`](docs/predecessors.md) | what v1 and v2 hold that the design must not lose; schema conflicts and the import policy |
| [`AGENTS.md`](AGENTS.md) | the contract for changing this repository |

## Build and check

Rust 1.91 is the workspace's declared minimum. Builds and compile-fail checks use
the pinned Rust 1.98.1 toolchain, Cargo, [`task`](https://taskfile.dev),
ESS 0.29.0 and AEP 0.57.0. The correctness workflow pins both tools to their
reviewed source commits.

```console
cargo check --workspace
task check
```

`task check` is the gate: format, clippy with warnings as errors, tests, rustdoc,
vendored YAML compatibility tests and documentation, ESS specification validation,
and the planning store's own validation.

## Layout

```text
Cargo.toml              the workspace; every crate opts into [workspace.lints]
crates/                 the six P1 crates; ekr is the binary, the rest are libraries
xtask/                  repository tasks that are not the product
docs/                   design, roadmap, predecessor analysis
systems/                ESS domains, one per crate, added as the crates arrive
.engineering/           AEP project file and planning store
.github/workflows/      the shared source gate
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).
