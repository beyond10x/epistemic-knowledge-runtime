---
format: aep.planning-md/3
id: task:streamed-page-shows-far-valid-times
kind: task
status: draft
title: The streamed page shows valid times beyond 2^53 ms exactly; unread routes decided
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

Retiring `/alt` (wave sdk-01) removed `crates/ekr/tests/adversary_p_page_2.rs`, whose three headless
cases checked that a valid-time bound beyond ±2^53 ms displays correctly: they read the old page's
markup (`#valid-at`, `table.claim outside`). The streamed page has no such check. The routes
`/projection` and `/roles` are also no longer read by any embedded page.

## Build

Port the three cases to the streamed page's markup (driving it as `view_page.rs` does), fixing the
page if it misdisplays such bounds; decide whether `/projection` and `/roles` stay as API routes
(documented as such) or go.

## Acceptance

- A valid-time bound of ±(2^53 + 1) ms displays exactly on the streamed page (a failing case first).
- `/projection` and `/roles` are either documented as API routes with a test, or removed with a 404
  case.
