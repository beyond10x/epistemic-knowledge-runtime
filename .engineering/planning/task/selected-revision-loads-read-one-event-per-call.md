---
format: aep.planning-md/3
id: task:selected-revision-loads-read-one-event-per-call
kind: task
status: draft
title: Selected-revision history loads read the revision stream one event per call
relations:
- serves: vision:o2
- derived_from: story:eventlog-0-4-batched-reads
- decomposes: epic:read-and-storage-cost
revision: 3
---
## What is wrong

Measured by adversary pass 2 on wave p1-15 unit p1-15-eventlog (`review-result:p1-15-eventlog-adversary-r2`): `history_at` (`crates/ekr-store/src/eventlog.rs:582`) and `replay` (`:778`) load history with a page limit of 1, so a selected-revision read costs N+5 provider calls (7 at 2 proposals, 29 at 24, counted through a counting provider). On the file provider each call verifies the log. Reached by `Commit::read(Some(revision))` (`crates/ekr-kernel/src/read.rs:60`), i.e. `ekr snapshot --at`, and `Commit::replay` (`commit.rs:196`). The base had the same shape (`load_history(1, …)`).

## What closes this

Selected-revision loads read the revision stream in pages up to `MAX_READ_LIMIT` and stop at the requested revision, so their provider calls stay constant up to one page; a counting-provider case holds it.

## Refined scope and acceptance

Cited at released main: crates/ekr-store/src/eventlog.rs history_at calls load_history with a
single-event page, and replay calls admitted with the same limit. StreamRead::absorb stops at
the selected event. Inferred edit surface: those entry points, the page reader if required,
and existing counting-provider tests under crates/ekr-store/tests. No native dependency,
wire format, durability, checkpoint cadence or authority rule changes.

Use bounded pages up to MAX_READ_LIMIT while retaining the exact selected boundary. Count
revision-stream provider calls below and across page boundaries, separating unrelated stream
calls. Compare selected roots and refusal results on file and SQLite providers. Cover first
and last events of a page, missing revisions, stalled cursors and replacement of a held prefix.

Mandatory safety cases place invalid envelopes, malformed domain events and missing later
record blobs after a valid selected revision within the same provider page. Earlier selected
reads retain their baseline result; reads reaching damage refuse. Damage before the selected
boundary still refuses. Provider-level batch decoding must not accidentally widen the selected
read's validation boundary. Every case asserts the preserved contract after the fix.

## Provider-boundary refinement

The preparatory history-readiness.md report in <cache>/ekr-next-three/coordinator identifies an
important baseline boundary. The coordinator verified Cargo.lock's pinned Eventlog source and
read eventlog-sqlite/src/lib.rs::read_stream and build_event: SQLite queries limit + 1 rows,
decodes every returned row, then truncates. Even the old one-event read can therefore refuse
corruption immediately after a selected occurrence. Revised acceptance preserves that refusal;
it does not promise to read through every physically damaged provider. File storage may verify
its journal before slicing, whose existing refusal also remains unchanged.

A larger page must preserve earlier successful selections when native decoding damage is farther
into its suffix. Test real SQLite invalid timestamp and invalid JSON rows beyond the old lookahead,
on cold and advancing-held readers; compare exact outcomes with the old path. Also test the old
immediate-lookahead refusal. Returned-page invalid envelopes/domain events remain separate cases:
EKR must stop at selection before processing their suffix, and refuse when selection reaches damage.
Retain missing later-record tests on both providers, cursor-stall and held-history-replacement checks.

An inferred EKR-only design is a selected-read wide-provider-error fallback from the same cursor
using the old narrow semantics, including the held-prefix continuation path. It is unverified
until implemented and tested. Do not swallow errors from validating returned prefix events or
trust replaced cached history. Count each actual fallback provider call, not only the initial
request. Healthy selections below and across the native page boundary must show reduced actual
revision-stream calls on both history_at and replay. Native occurrence positions, not domain
revision numbers, define that boundary. Native dependency, format and durability changes remain
excluded. The readiness report is preparation only; this wave has not yet been dispatched.
