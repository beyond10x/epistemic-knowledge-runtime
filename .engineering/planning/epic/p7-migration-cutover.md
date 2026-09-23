---
format: aep.planning-md/1
id: epic:p7-migration-cutover
kind: epic
status: draft
title: P7 — Migration and cut-over
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p4-operator-surface
- depends_on: epic:p5-frontier-schema-scheduler
- depends_on: epic:p6-maintenance-observability
- serves: vision:o5
revision: 1
---
## Context

`docs/predecessors.md` § 5–9: six v1 conflicts, thirteen v2 conflicts, one import policy. The v1→v2
import of 2026-09-03 skipped ≈10,600 v1 records and synthesised 427; this epic imports the skipped
kinds from v1 directly and marks the synthesised records as inherited without evidence.

## Outcome

- Curated v1 records (including the kinds v2 skipped), the v2 `store/`, and the v2 infrastructure
  mirror enter transient roots `v1-workday`, `v2-store` and `infrastructure` with `origin: inherited`;
  re-observation promotes; nothing inherited is `Accepted` without evidence.
- Identities are minted; predecessor ids survive as `canonical_name` or aliases; predecessor
  `provenance`, `revision` and `lifecycle_state` are renamed on import.
- v2 `source-cursor` records become checkpoints; v2 per-subject history becomes Provenance-class
  blobs.
- The v2 instance runs on this runtime; systemd units; release 0.1.0 through the bot route.

## Acceptance

Record counts reconcile per kind against v1 and against `org-brain/import-report.json`; every open
item on the v2 attention page has a counterpart on the v3 one; seven days unattended produce the
same evidence a CLI round produces.

## Depends on

P4, P5 and P6.
