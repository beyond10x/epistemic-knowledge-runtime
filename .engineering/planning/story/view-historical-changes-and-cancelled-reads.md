---
format: aep.planning-md/3
id: story:view-historical-changes-and-cancelled-reads
kind: story
status: draft
title: ekr view answers /changes at a historical revision at the head's cost, and a disconnected read is cancelled
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#96
relations:
- decomposes: epic:zero-open-issues-2026-11
- serves: vision:o5
revision: 1
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
