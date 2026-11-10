---
format: aep.planning-md/3
id: story:search-live-typing-waits-on-the-page
kind: story
status: implemented
title: The live-typing browser case waits on the page, not on elapsed time
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#104
relations:
- decomposes: epic:zero-open-issues-2026-11
- serves: vision:o5
scope:
- confidence: inferred
  path: crates/ekr-search-web/src/browser.rs
- confidence: inferred
  path: crates/ekr/assets/search.wasm
- confidence: cited
  path: crates/ekr/tests/search_live.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-10T02:23:26Z", actor: "agent:claude-integrate-20261110", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Context

GitHub issue #104. Commit `57244640` ran the Correctness workflow twice with different outcomes:
run 40503144297 (pull_request, #98) succeeded; run 40510896648 (push to main) failed
`crates/ekr/tests/search_live.rs:456`,
`live_typing_preserves_focus_and_caret_without_navigation`, panic `DOM never contained No matches`.
The captured DOM still showed the initial empty state (`data-live-search="ready"`,
`aria-busy="false"`, "Start with a name."), so the typed query never produced a live response
within the test's wait. The other 10 tests in the file passed in both runs.

AGENTS.md forbids a test that passes or fails on wall-clock time.

## Build

Diagnose first: reproduce the red (load, a delayed response, a dropped input event) and name the
cause; the issue names two candidates and establishes neither. Then the test awaits something the
page does (an event or a deterministic hook), or the page is fixed if the typed input was lost.

## Acceptance

- The cause is named with the command that showed it.
- The case no longer depends on a response arriving within a fixed time: it waits on a page state
  that a lost or late response cannot satisfy early, and a hang still fails with the DOM in the
  message.
- The case passes 30 consecutive runs while the machine's load average is above 20, the count
  printed by the loop that ran it.

## Out of scope

The other wall-clock assertions listed in `task:no-test-asserts-wall-clock-cost`.

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` integration test `search_live` — cited
- **Files:** `crates/ekr/tests/search_live.rs:456` (the panic in `Browser::until`, lines 449-462) — cited
- **Files:** `crates/ekr/tests/search_live.rs:534` (`live_typing_preserves_focus_and_caret_without_navigation`) — cited
- **Symbols:** `Browser::until`, `Browser::type_text`, `Browser::focus`, `Browser::navigate` in `search_live.rs` — cited
- **Also likely:** `crates/ekr-search-web/src/browser.rs` (`input` listener at :54, `data-live-search="ready"` set at :85, `aria-busy` at :116/:201) — inferred, edited only if the page needs a new deterministic hook or dropped the input
- **Also likely:** `crates/ekr/assets/search.wasm` — inferred, the committed artifact that `xtask/src/search_web.rs:41` rebuilds whenever `browser.rs` changes
- **Documents:** `docs/epistemic-knowledge-runtime-design.md` — inferred, mentions the live-search hook; changes only if the hook contract changes
- **Confidence:** high — the story names the file, line and test; the page side is conditional on the diagnosis
- **Would collide with:** any unit touching the shared `Browser` helpers in `crates/ekr/tests/search_live.rs` (all 11 cases use them), and any unit rebuilding `crates/ekr/assets/search.wasm` (binary, unmergeable) — cited
- **Safety fact:** this case is the only browser case that types without first waiting for `data-live-search="ready"`; six other cases call `browser.until("data-live-search=\"ready\"")` after `navigate` (`search_live.rs:742, 797, 940, 985, 1016, 1045`), while :534-541 goes `navigate` -> `focus` -> `type_text`. A test-local wait added there changes no other case — step 2, unproven — inferred
