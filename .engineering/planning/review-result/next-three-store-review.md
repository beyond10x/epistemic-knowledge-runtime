---
format: aep.planning-md/3
id: review-result:next-three-store-review
kind: review-result
status: active
title: next-three-store-review
relations:
- reviews: epic:p2-observation-layer
- reviews: task:seed-race-test-names-its-signal
- reviews: story:store-provider-migration
revision: 1
---
needs-revision

Owners: 3 findings, 3 coordinator, 0 implementor.

This review read 42 planning artifacts. Validation reported 423 artifacts, 23 existing reviews without recognized findings blocks, and `valid`. The coordinator retains the verbatim validation output separately.

- `epic:p2-observation-layer` — The epic still promises bundled third-party adapters and predecessor raw importers, contrary to accepted ADR 0012. Revise its outcome and acceptance to the consumer adapter contract while retaining observation, poll and privacy obligations. — `.engineering/planning/epic/p2-observation-layer.md:24`, `:32–40`; `.engineering/planning/architecture-decision-record/0012-source-adapters-are-a-contract.md:29–38`.
- `task:seed-race-test-names-its-signal` — The task says the race test prints no signal, but released source now prints seed and head signal numbers. Record this partial completion and retain the unverified 50-run, load-above-30 acceptance; diagnostics alone do not justify closing the task. — `.engineering/planning/task/seed-race-test-names-its-signal.md:18`, `:29–30`; `crates/ekr/tests/adversary2_p5_01_store_open.rs:209–224`.
- `story:store-provider-migration` — The story says no migrate verb exists, but the released CLI already provides preserving migration to the same provider. Revise its baseline and scope around the missing cross-provider capability and the existing migration guarantees. — `.engineering/planning/story/store-provider-migration.md:17`; `crates/ekr/src/cli/migrate.rs:1–2`.

```findings
- file: .engineering/planning/epic/p2-observation-layer.md
  line: 24
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "The epic still promises bundled third-party adapters and predecessor raw importers, contrary to accepted ADR 0012. Revise its outcome and acceptance to the consumer adapter contract while retaining observation, poll and privacy obligations."
- file: .engineering/planning/task/seed-race-test-names-its-signal.md
  line: 18
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "The task says the race test prints no signal, but released source now prints seed and head signal numbers. Record this partial completion and retain the unverified 50-run, load-above-30 acceptance; diagnostics alone do not justify closing the task."
- file: .engineering/planning/story/store-provider-migration.md
  line: 17
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "The story says no migrate verb exists, but the released CLI already provides preserving migration to the same provider. Revise its baseline and scope around the missing cross-provider capability and the existing migration guarantees."
```
