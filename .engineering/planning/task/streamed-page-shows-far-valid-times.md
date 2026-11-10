---
format: aep.planning-md/3
id: task:streamed-page-shows-far-valid-times
kind: task
status: active
title: The streamed page shows valid times beyond 2^53 ms exactly; unread routes decided
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
- decomposes: epic:zero-open-issues-2026-11
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:55:00Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:00Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
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

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` — the embedded viewer page and its headless page tests — cited
- **Files:** `crates/ekr/tests/view_page.rs` (the DevTools-driven `rendered()` at :940 the story says to drive as) — cited
- **Files:** `crates/ekr/src/cli/viewer/index.html` (the streamed page; "fixing the page if it misdisplays") — cited
- **Files:** `crates/ekr/src/cli/view.rs:880-881` (`"/projection"`, `"/roles"` in `route()`), dispatch at :474-475 and :1095-1096, module route table at :68-70 — cited
- **Symbols:** `exact` (`index.html:388`, keeps an unsafe integer as a string), `Data.ms` (`:402`, returns null for anything but a safe integer), `num` (`:1516`), `iso` (`:662`), `span2` (`:1578`) — cited
- **Also likely:** the ported cases land in a new `crates/ekr/tests/adversary_p_page_2.rs` or inside `view_page.rs`; the old file is recoverable with `git show d19820f17^:crates/ekr/tests/adversary_p_page_2.rs` — inferred
- **Also likely:** defect site is `index.html:1653-1654` (`valid_from: num(c.valid_from)`), where a bound beyond 2^53 arrives as a string, becomes null and would show as "open"; the time filter `validAt` (`:858`) compares the same values — inferred, read not run
- **Also likely:** fixture `crates/ekr/tests/fixtures/view-page/sounding/` is reused unchanged (the old cases rewrote assertion a510's valid time in a copy of the seed) — inferred
- **Also likely:** if the routes go: `crates/ekr/tests/view_roles.rs`, `adversary_roles_rule.rs`, `adversary_roles_rule_2.rs`, `adversary_x6_e.rs`, `adversary_v_view.rs`, `adversary_v_view_r2.rs`, `view_cli.rs`, `adversary_p_page.rs:478` and `crates/ekr-views/src/roles.rs` doc comments all request them — inferred
- **Documents:** `docs/cli.md` (:1253-1254 route table, :1524-1561 § Roles, :1842), `CHANGELOG.md`; `systems/ekr/domains/views.yaml` (:887-898, :2892) only if the routes are removed — inferred
- **Confidence:** medium — the story names the test file to imitate and both routes, and the tree confirms them; which test file receives the port, the defect site and the keep-or-remove decision are open
- **Would collide with:** any unit editing `crates/ekr/src/cli/viewer/index.html`, `crates/ekr/tests/view_page.rs`, the route table or `route()` in `crates/ekr/src/cli/view.rs`, or `docs/cli.md` § `ekr view` — inferred
- **Safety fact:** no embedded page reads `/projection` or `/roles` (`index.html` has no match for either; `docs/cli.md:1543` says "never `/projection`"), so removing or keeping them changes nothing the page shows; the tests listed above and any outside HTTP client still depend on them — step 2, unproven
