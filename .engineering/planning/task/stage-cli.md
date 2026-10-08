---
format: aep.planning-md/3
id: task:stage-cli
kind: task
status: implemented
title: Every store verb joins a stage by EKR_STAGE or --stage
relations:
- serves: vision:o5
- derived_from: story:a-run-is-staged-and-published-whole
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:53:48Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T08:53:48Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T03:23:30Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Build

Add `ekr stage begin|publish|abandon` and the `EKR_STAGE` / `--stage` option on every store verb; update `docs/cli.md` in the same change.

## Acceptance

The story's acceptance as named cases on SQLite and PostgreSQL, including a run whose gate fails leaving `ekr head` unchanged; `crates/ekr/tests/docs_cli.rs` passes.
