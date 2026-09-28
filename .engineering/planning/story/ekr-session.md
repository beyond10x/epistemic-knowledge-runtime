---
format: aep.planning-md/3
id: story:ekr-session
kind: story
status: active
title: ekr session serves the existing verbs over one opened store
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: cited
  path: crates/ekr/tests/session.rs
- confidence: cited
  path: docs/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T09:14:09Z", actor: "agent:claude-coordinator", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-28T09:14:09Z", actor: "agent:claude-coordinator", revision: 7}
---
## Context

A host calls `ekr` once per resolve, mint, propose, validate and commit, and each call opens the
store from scratch (see `epic:ingestion-throughput`).

## Build

`ekr session`: reads one JSON request per line on standard input and writes one JSON response per
line on standard output, holding one opened runtime until end of input.

- Requests name an existing verb and its arguments: `resolve`, `mint`, `propose`, `validate`,
  `commit`, `head`, `snapshot`. Each response is the JSON the one-shot verb prints for the same
  store state, or its refusal with the same name.
- Writes go only through `propose`, `validate` and `commit`, as for the one-shot verbs
  (`AGENTS.md`, agents that use the binary).
- A line that is not a request is answered with a named refusal and the loop continues.
- The verb bodies in `crates/ekr/src/cli/` are called from the loop, not copied.
- `docs/cli.md` documents the verb, the request shape and the refusal.

## Acceptance

- For each verb, the session's response is byte-identical to the one-shot verb's output on the
  same store state, on the file and SQLite providers.
- A malformed line yields the named refusal and the next line is still served.
- A commit made in the session is visible to the next `head`/`resolve` in the same session.
- `crates/ekr/tests/docs_cli.rs` passes with the new verb documented.
- Measured: 1,000 resolves through one session on the 112 MB store of the epic, release build.
