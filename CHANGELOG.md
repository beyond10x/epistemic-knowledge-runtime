# Changelog

Every change a user of the runtime sees, per release. Unreleased work sits at the top.

## [Unreleased]

### Added

- The six P1 crates — `ekr-core`, `ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store`, `ekr` — as
  empty workspace members in the acyclic order `docs/roadmap.md` § 3 gives, every P1 dependency
  declared, `rust-version` 1.91, and contract tests holding the manifests, the lockfile, README and
  the doc comments to the story (wave p1-01).
- Repository bootstrap: Cargo workspace with `xtask`, the shared source gate, the AEP planning
  store, and the roadmap and predecessor analysis under `docs/`.
- `systems/ekr/`: the runtime's own executable system specification (`ess/1`), four domains —
  kernel, ontology, graph, store — validated by `task spec-check`.
- Design amendments 81–87 to `docs/epistemic-knowledge-runtime-design.md`, carrying what v1
  (`company-brain`) and v2 (`org-brain`) proved necessary: operator surface, projections and views,
  the outward-write invariant, obligations and horizons, the interpretation session contract, blob
  evidence, and per-type lifecycles.
