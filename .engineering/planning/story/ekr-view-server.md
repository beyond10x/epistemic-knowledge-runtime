---
format: aep.planning-md/3
id: story:ekr-view-server
kind: story
status: implemented
title: ekr view serves a read-only graph viewer for a store on 127.0.0.1
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:graph-projection-renderer
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/ekr/Cargo.toml
- confidence: inferred
  path: crates/ekr/src/cli
- confidence: inferred
  path: crates/ekr/tests
- confidence: inferred
  path: docs/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-27T12:43:26Z", actor: "human:timo", revision: 4, imported: true}
- {from: "proposed", to: "active", at: "2026-09-27T12:43:47Z", actor: "human:timo", revision: 5, imported: true}
- {from: "active", to: "implemented", at: "2026-09-27T21:29:39Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}, imported: true}
---
## Context

Operator request (2026-09-26): `ekr view`, a subcommand that serves a read-only graph viewer for a store on 127.0.0.1. A prototype exists outside this repository (a script server plus a single HTML page); this story describes its behaviour, not its code, and the language rule holds: the server is Rust, in the `ekr` binary, with clap derive.

Design sections served: § 82 (rendering is not delivery; a local read-only page is not an outward write under § 83), § 47 (canonical as of revision N), § 62 (each fact the page shows carries its assertion id), § 86 (evidence bytes by content hash). Invariant 6 holds: nothing here writes outward or to the store.

## Behaviour

- `ekr view` takes the existing global `--host`, `--store`, `--backend` (and their `EKR_*` variables) and a `--port` (0 picks a free one). It binds **127.0.0.1 only**; no option binds another address. On start it prints one JSON line with the URL on stdout, then serves until interrupted.
- It opens an **existing** store only, as every read verb does (`story:store-open-semantics`), and reaches no propose, validate or commit path.
- `GET /` serves the viewer page, embedded in the binary at build time (`include_str!`/`include_bytes!`); nothing is read from disk at run time. This story embeds a minimal page that fetches and shows the projection's `meta`; the full viewer is `story:data-free-graph-viewer`.
- `GET` of the projection endpoint returns the bytes `ekr-views` renders for `ekr.graph-projection/1` at the head, or at `?revision=N`; an absent revision is a 404 with a named refusal.
- `GET` of the evidence source endpoint for an evidence id returns that evidence's retained bytes through `Runtime::content`: `text/plain; charset=utf-8` when they are UTF-8, otherwise `application/octet-stream`, always with `X-Content-Type-Options: nosniff`. Unknown evidence id, or bytes not retained: 404. Record text is untrusted evidence (A14): it is never served as HTML.
- Every other method is 405; every other path 404. Responses carry no cookies and no CORS allowance.
- The store is read synchronously. `architecture-decision-record:0006-ekr-store-bridges-the-async-port` and `story:synchronous-runtime-refusal` mean a store call inside a Tokio context is refused, so either use a blocking HTTP server or keep store calls off any async runtime; say which in the change.
- `docs/cli.md` gains the verb in the same change (`crates/ekr/tests/docs_cli.rs` compares the page with the binary), and `ekr guide` names it.

## Domain relations

- `Evidence → retained bytes` (a `StoredObject` by `content_hash`), many-to-zero-or-one — inferable (**inferred** from `crates/ekr-graph/src/evidence.rs:260` and `crates/ekr-kernel/src/runtime.rs:253`, `Runtime::content`; no ess/1 relation declares it).
- Every relation of the projection itself: `story:graph-projection-specification` § Domain relations.

## Out of scope

The viewer's rendering and its data-free rule, authentication (loopback only), TLS, live reload on a new revision, any write.

## Acceptance

On both providers, an end-to-end test starts `ekr view --port 0` on a seeded fixture store, reads the printed URL, and gets 200 with the embedded page at `/`, the exact bytes `ekr-views` renders for the projection endpoint at head and at an earlier revision, the retained bytes of a seeded evidence item as `text/plain` from the source endpoint, 404 for an unknown evidence id, 405 for a `POST`, and afterwards the store's head revision and published event count are unchanged.
