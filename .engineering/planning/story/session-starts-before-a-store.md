---
format: aep.planning-md/3
id: story:session-starts-before-a-store
kind: story
status: active
title: ekr session starts before a store exists and can seed it
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
- derived_from: story:ekr-session
scope:
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/src/main.rs
- confidence: inferred
  path: crates/ekr/tests/session.rs
- confidence: inferred
  path: docs/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T18:16:21Z", actor: "agent:claude-coordinator", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T18:56:02Z", actor: "agent:claude-coordinator", revision: 3}
---
## Context

Asked by a consumer instance on 0.0.12 (2026-09-28): building a store through `ekr session` takes
16 `ekr` processes on a small fixture where 2 would do. The seed needs minted ids and evidence
hashes first, and `ekr session` refuses to start without a store (`store-not-found`, exit 1), so
every `mint` and `hash` before the seed runs one-shot; on a corpus with one evidence entry per file
that is one process per file.

## Build

`ekr session` starts when the configured store does not exist yet and serves the verbs that open
no store (`mint`, `hash`, `schema`); `seed` is served in a session started with an explicit flag
(for example `--create`), after which the session holds the new store and serves every store verb.
Without the flag a store verb before the seed answers the one-shot `store-not-found` refusal as a
session response and the loop continues.

## Acceptance

- A session started on an absent store answers `mint`, `hash` and `schema` exactly as the one-shot
  verbs do, on both providers.
- In a session started with the create flag, `seed` followed by `propose`, `validate`, `commit`
  and `resolve` gives the same documents as the one-shot sequence.
- Without the flag, a store verb before any seed answers `store-not-found` (exit 1 in the response)
  and the next line is served.
