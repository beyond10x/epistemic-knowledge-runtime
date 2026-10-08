---
format: aep.planning-md/3
id: task:postgres-source-copy
kind: task
status: implemented
title: A stage is begun by a preserving copy, from PostgreSQL as from SQLite
relations:
- serves: vision:o5
- derived_from: story:a-run-is-staged-and-published-whole
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:53:47Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T08:53:47Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T03:23:30Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Build

Begin a stage: a preserving copy of the store at its head into a new tenant of the same provider, with PostgreSQL admitted as a copy source under one consistent read.

## Acceptance

A stage begun from a PostgreSQL store and from a SQLite store replays to the source's roots at that head; an interrupted begin leaves a stage that refuses to be read as complete.
