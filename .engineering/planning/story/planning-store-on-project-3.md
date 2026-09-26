---
format: aep.planning-md/2
id: story:planning-store-on-project-3
kind: story
status: active
title: Move the EKR planning store to aep.project/3
relations:
- serves: vision:o2
scope:
- confidence: cited
  path: .engineering/planning
- confidence: cited
  path: .engineering/project.yaml
- confidence: cited
  path: .engineering/state
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: inferred
  path: .github/workflows/planning.yml
- confidence: inferred
  path: .gitignore
- confidence: inferred
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
revision: 13
---
## Context

The six `aep.project/2` stores moved to `aep.project/3` tree stores on 2026-09-24, and `planning validate` is required from AEP 0.59.2. EKR is still `aep.project/1` with AEP 0.57.0 in CI; its single `journal.jsonl` is rewritten by every planning write, which is also what pushed pull request 16 past the Gates scan limit.

## Acceptance

- The EKR planning store is `aep.project/3`, exported once with a fidelity check; no artifact, relation, evidence or history is lost (counts before and after).
- CI and `task plan-check` run AEP 0.59.3 or later.
- Two branches that write different artifacts merge without a conflict (one case or a recorded dry run).

## Scope

Derived 2026-09-25 by `story-scoper` (wave p1-15). Confidence: high; `planning.yml`, `AGENTS.md` and `.gitignore` are by analogy.

- AEP 0.59.3 `plan store export` accepts only an `aep.project/2` store (`crates/edge/aep-cli/src/store_command.rs:5330-5342`), so EKR goes /1 → /2 → /3, repeating eventlog `6f8186a2` then `8b9fdd63` — cited
- sequence: `plan store inspect`, `migrate dry-run`, `writer-control hold`, `migrate apply`, `store verify`, `plan store export --engineering .engineering --into <dir> --map <file>`, `artifact validate`, `store install-hooks` — cited (AEP 0.59.3 `website/docs/reference/cli.md:122-131`)
- `.engineering/project.yaml` (selector and `protocols` pin `#28abe09`), `.engineering/planning/` (journal removed, documents re-rendered), `.engineering/state/` (new tree authority) — cited
- `.github/workflows/correctness.yml:51,56` (AEP 0.57.0 cache key and `--rev ed0f60e7`), `Taskfile.yml:76-78`, `README.md:40` — cited
- `.github/workflows/planning.yml` (new required check), `AGENTS.md:45-46`, `.gitignore` (`.engineering/migrations/` untracked) — inferred
- collides with every planning-store writer; runs while no other store write is in flight
- not established: a /1 → /2 `migrate apply` on AEP 0.59.3 (eventlog did it on 0.55.0); fixups needed; the required-check ruleset and Gates baseline move (outside the repository)
