---
format: aep.planning-md/3
id: story:sdk-read-helpers
kind: story
status: implemented
title: The session serves the views reads and the SDK types them
relations:
- depends_on: story:sdk-session-transport
- depends_on: story:changes-since-read
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr-sdk/src/read
- confidence: inferred
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: docs/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T18:41:42Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-29T18:41:42Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-09-29T20:35:57Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

`ekr session` serves no `ekr.views` reads; they are served by `ekr view` over HTTP and by
`ekr mcp`. A consumer instance reads whole snapshots (8 MB on one trial store) to count things.

## Build

- The `ekr.views` reads (overview, search, describe, expand, timeline, and changes since once
  `story:changes-since-read` ships) added to `ekr session` as verbs, documented in `docs/cli.md`;
  no new noun, one process per store (decided 2026-09-29, `epic:consumer-sdk`).
- A typed `Reader` in the SDK over those verbs: `Overview`, `NodeMatches`, `NodeDetail`,
  `Timeline`, a paging iterator for `expand`, and typed `Head`, `Snapshot`, `Ontology`,
  `Transactions` and `Explanation`. The last five also run as one-shot reads with no session
  open, since a consumer's health and code-names checks read a store that way.

## Surface (inferred)

`crates/ekr/src/cli/session.rs`, `docs/cli.md`, `crates/ekr-sdk/src/read/{mod,views}.rs`, tests
reusing `crates/ekr-views/tests/fixtures/conformance`.

`story:reads-served-from-a-persisted-read-model` affects performance only.

## Acceptance

- Each session views verb returns the document the HTTP route returns for the same revision.
- Every conformance fixture document deserializes into its typed reader.
- A field added to a views format without an SDK update fails an engine test.
- The `expand` iterator pages a 5,000-node fixture with nothing lost or repeated.
- A reader in one session sees another session's commits on its next call.
- `head`, `ontology`, `snapshot`, `transactions` and `explain` return the same typed value through
  a session and as one-shot reads.
