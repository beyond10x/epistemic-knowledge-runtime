---
format: aep.planning-md/1
id: epic:p0-bootstrap
kind: epic
status: implemented
title: P0 — Bootstrap and design v3.1
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- serves: vision:o2
revision: 4
---
## Context

The repository must be able to accept governed work before the first runtime crate exists, and the
design must say what the predecessors proved necessary (`docs/predecessors.md` § 2) before P1
implements the ontology.

## Outcome

- Git repository; Cargo workspace with `xtask` only; `[workspace.lints]` forbidding `unsafe_code`
  and warning on `missing_docs`.
- `AGENTS.md` with a `## Serves` section (O2, O5, O6), `README.md`, `CHANGELOG.md`, LICENSE
  (Apache-2.0), `.github/workflows/shared-gates.yml`, `Taskfile.yml` with the gate.
- AEP planning store written from scratch: `.engineering/project.yaml`, three vision projections,
  this initiative, one epic per phase, three ADRs.
- Design amendments 81–87 appended to `docs/epistemic-knowledge-runtime-design.md`: operator surface
  (A1), projections and views (A2), invariant 6.11 on outward writes (A3), obligations and horizons
  (A4), interpretation session contract (A5), `ObservationContent::Blob` (A12), per-type lifecycles
  and operations (A13).

## Acceptance

`task check` exits zero on the empty workspace, `aep plan artifact validate` reports the store
valid, and amendments 81–87 are present in the design document.

## Not in this epic

`b10x.docs.yaml` is produced by `atlas docs reconcile`, not written by hand; Gates enrollment of
this repository is an administrator action. Both are recorded as pending in the roadmap.
