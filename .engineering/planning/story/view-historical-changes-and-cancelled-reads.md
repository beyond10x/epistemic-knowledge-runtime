---
format: aep.planning-md/3
id: story:view-historical-changes-and-cancelled-reads
kind: story
status: implemented
title: ekr view answers /changes at a historical revision at the head's cost, and a disconnected read is cancelled
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#96
relations:
- decomposes: epic:zero-open-issues-2026-11
- serves: vision:o5
scope:
- confidence: inferred
  path: crates/ekr-views/src/changes.rs
- confidence: cited
  path: crates/ekr-views/src/index.rs
- confidence: inferred
  path: crates/ekr-views/tests/changes.rs
- confidence: inferred
  path: crates/ekr/src/cli/http.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/view_cli.rs
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:54:58Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:54:58Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-10T02:23:27Z", actor: "agent:claude-integrate-20261110", revision: 12, decided_on: {"recorded":{"test_result":1}}}
---
## Context

GitHub issue #96 (opened against 0.0.38). On a served store at head 64 (about 12k nodes and 108k
assertions), `ekr view`:

- `GET /changes?since_revision=N&at=<head>` answers in 0.20–0.27 s;
- `GET /changes?since_revision=N&at=<revision 62 or older>` answers in 27.4–28.4 s per call.

When the client gives up, the reader keeps working on the abandoned read, so later reads queue
behind it.

Likely cause, inferred by the issue and not measured: `changes` in `crates/ekr/src/cli/view.rs`
obtains an index for revision `at` from `IndexCache::index` before answering; for a revision other
than the head this appears to build a whole index at that revision on every request. The reader
does not watch the connection while it works.

## Build

Diagnose first (a counted, red-capable loop), then, in the issue's order:

1. Spec first in `systems/ekr/domains/views.yaml`: a read whose client has disconnected is
   cancelled, and the next queued read starts.
2. A historical `at=` read answers in the same order of work as `at=<head>`.

## Acceptance

- A `/changes` read with a historical `at=` costs the same order of work as the same read with
  `at=<head>`: a test counts work (index builds, events read), not wall-clock time, and fails on
  `origin/main` `57244640`.
- A read whose client disconnected stops, and a read queued behind it is answered: a test holds it
  without a wall-clock assertion, and fails on `57244640`.
- `views.yaml` states the cancellation, and the views conformance suite still passes.

## Out of scope

Other routes' cost; the MCP hosts, unless they share the corrected code path.

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` view server (`ekr view`) — cited
- **Files:** `crates/ekr/src/cli/view.rs:1728` (`fn changes`, calls `indexes.index(runtime, at)` at 1759) — cited
- **Files:** `systems/ekr/domains/views.yaml` (reader text near 2891-2935, changes format near 1954) — cited
- **Files:** `crates/ekr-views/src/index.rs:1027` (`IndexCache`, `IndexCache::index` loads through `Index::load` on a miss) — cited
- **Symbols:** `IndexCache::index`, `IndexCache::DEFAULT_CAPACITY` (3), `Index::changes_as`, `ChangesRequest` — cited
- **Also likely:** `crates/ekr/src/cli/http.rs` — inferred, `super::http::queue(IN_FLIGHT_LIMIT)` and `WAIT_TIMEOUT` are the store-thread job queue where a cancellation check would sit
- **Also likely:** `crates/ekr-views/src/changes.rs` — inferred, a historical `/changes` may be answered from shared change lists without a whole index
- **Also likely:** `crates/ekr-views/tests/changes.rs` — inferred, already counts documents parsed per revision (line 432), the natural home for the work-count test
- **Also likely:** `crates/ekr/tests/view_cli.rs`, and `view.rs` `mod tests` (line 2049) — inferred, the disconnect and queued-read test
- **Also likely:** `crates/ekr/tests/conformance.rs` — inferred, the views conformance suite that must still pass; edited only if the new spec text gets a vector
- **Documents:** `systems/ekr/domains/views.yaml`; `view.rs` module doc (lines 7-23 describe queue and deadlines) — cited
- **Confidence:** medium — the story names the route function and the spec file, but the cause is unmeasured and the fix site (cache, changes path or queue) is open until the diagnosis
- **Would collide with:** any unit touching `crates/ekr/src/cli/view.rs`, `crates/ekr/src/cli/http.rs` (shared with `ekr mcp-http`), `crates/ekr-views/src/index.rs` or `views.yaml`
- **Safety fact:** `IndexCache::index` is also called by `crates/ekr/src/cli/mcp.rs:547,717` and `crates/ekr/src/cli/session/views.rs:319-418`, so a change inside it or in `http.rs`'s queue reaches the MCP hosts and session verbs, while a change confined to `view.rs` does not — step 2 (`git grep -n 'indexes.index'`), unproven
