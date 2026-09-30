---
format: aep.planning-md/3
id: epic:ingestion-throughput
kind: epic
status: implemented
title: A long ingest is bound by its work, not by ekr process start-up
relations:
- serves: vision:o5
- decomposes: initiative:epistemic-knowledge-runtime
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T09:14:10Z", actor: "agent:claude-coordinator", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T09:14:10Z", actor: "agent:claude-coordinator", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:21:59Z", actor: "human:timo", revision: 4}
---
## Context

A host that ingests through the `ekr` binary pays process start-up on every call. On a store
seeded with 11 MB of evidence (112 MB on disk, 44 blobs, one revision), one `ekr resolve` takes
2.94 s and `ekr head` 0.57 s (release build of `0cc35c9e`, file provider). A consumer applying
67,558 facts with 4,127 distinct references needed about 3 h, almost all of it start-up.

Profile of one `ekr resolve` (frame-pointer release build, `perf --call-graph fp`; inclusive
shares, so they overlap):

| share | cause | code |
|---|---|---|
| 49% | `load_history` reads every required blob and SHA-256s it | `crates/ekr-store/src/eventlog.rs` |
| 41% | the 39 MB seed envelope (evidence payloads as JSON integer arrays) decoded twice, as `serde_json::Value` and then typed | `crates/ekr-kernel/src/seed.rs` `envelope` |
| 29% | checkpoint restore, mostly the same envelope decode | `crates/ekr-kernel/src/checkpoint.rs` `restore_checkpoint` |

## Outcome

A long ingest is bound by the work it does, not by process start-up: many calls share one opened
store, and one process verifies a blob and decodes the seed envelope once.

## Stories

- `story:ekr-session`: a long-lived stdio session serving the existing verbs.
- `story:history-loaded-once-per-process`: the eventlog store verifies a retained blob once per
  process and reads only new events after that.
- `story:seed-envelope-decoded-once`: one decode of the seed envelope per process, shared by every
  reader, skipping payload bytes where only their keys are needed.

No persisted format changes in these three. A seed envelope that names payloads by hash instead of
embedding them is a format change with a migration, recorded as its own draft story.
