---
format: aep.planning-md/3
id: task:readers-reopen-a-replaced-store
kind: task
status: draft
title: Long-running readers reopen a store replaced at their path
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

`ekr mcp`, `ekr view` and `ekr session` open the store once when they start (`crates/ekr/src/cli/mcp.rs:4`,
`crates/ekr/src/cli/mod.rs:11-14`, `docs/cli.md:496`, `:581`). A consumer instance replaces its store
file by rename when it promotes a new one, keeping the old file under another path. Measured by
that instance on 0.0.14 (2026-09-29): a running `ekr mcp` kept answering from the replaced store
until it was restarted. For `ekr view` this is inferred, not measured. An MCP client such as Claude
Code keeps the server alive for a whole session, so its agents read the replaced store without any
sign of it.

## Build

Before each request, a long-running reader (`ekr mcp`, `ekr view`, `ekr session`) compares the
identity of the store at its configured path (device and inode of the file store root or the SQLite
file) with the one it opened. When they differ, it reopens the store at that path and answers from
it. If the reopen fails, it answers the refusal naming the replacement and never the old store's
data. A session with uncommitted proposals open refuses the reopen by name rather than dropping them.

## Acceptance

- A running `ekr mcp` answers `head` from the new store after the store file is replaced by rename,
  on both providers, with no restart.
- `ekr view` answers `GET /head` from the new store after the same replacement.
- A replacement by a file that is not a store gives a named refusal on the next request, not the
  previous store's answer.
- Without a replacement, the check adds no store read to a request (the counting test pattern of
  `story:history-loaded-once-per-process`).
