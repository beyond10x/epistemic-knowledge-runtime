---
format: aep.planning-md/3
id: story:search-live-typing-waits-on-the-page
kind: story
status: draft
title: The live-typing browser case waits on the page, not on elapsed time
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#104
relations:
- decomposes: epic:zero-open-issues-2026-11
- serves: vision:o5
revision: 1
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
