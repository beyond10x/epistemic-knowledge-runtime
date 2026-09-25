---
format: aep.planning-md/1
id: task:no-store-at-a-mistyped-path
kind: task
status: draft
title: A read verb or a refused seed leaves a new store at the path it was given
relations:
- serves: vision:o5
- derived_from: story:agent-discoverable-cli
revision: 1
---
## What is wrong

Measured in wave p4-01 (adversary pass 2 on unit agent-cli, finding 6; blind trial 2, gap 2): a read verb given a store path that holds no store, and a seed that is refused, both leave a new provider store at that path. `ekr head` with a mistyped `EKR_STORE` created a `nostore/` directory with a `manifest.json` and exited 1 with "no seed"; a seed refused with `seed-evidence-payload-mismatch` left a 110 KB SQLite file.

## What closes this

Read verbs open an existing provider only and refuse a path with no store, without creating one; a refused seed leaves no store behind. Cases on both providers.
