---
format: aep.planning-md/3
id: story:browser-tests-one-chromium
kind: story
status: active
title: Browser tests share one Chromium per test run
summary: The test step starts one Chromium; each browser case gets its own browser context; the browser group runs in parallel and the step reaches 120 s
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T11:40:38Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T11:40:39Z", actor: "human:timo", revision: 3}
---
## Outcome

The browser tests share one Chromium per test run, so their part of the test step falls from about
300 s to what their cases cost, and the step can reach its 120 s budget.

## Work

Measured on PR #109 (review finding 37): six browser test files (about 9,700 lines, 8 launch
sites, each with its own DevTools-protocol driver) launch one Chromium per case and run one at a
time in the nextest group `browser`, about 300 s of the step. nextest runs each test in its own
process, so a browser shared inside one test binary does not work.

- The test step starts one headless Chromium before the tests (swiftshader, `UNRESOLVABLE`
  host-resolver-rules, no component update, no background networking) and exports its DevTools
  websocket address; it stops it after the tests.
- One shared support driver replaces the six private ones. Each case opens its own browser context
  (`Target.createBrowserContext`) and gets the pinned libraries through `Fetch` interception, so
  cases do not see each other's state and make no network request.
- The `browser` test group runs its cases in parallel.
- With the step under 120 s, the interim `global-timeout` of 600 s (decision D1) becomes 120 s.

## Acceptance

The test step launches Chromium once (counted by the step's log); every browser case passes with
the shared browser; the browser files' share of the step is printed and is under 60 s; and the
nextest `global-timeout` is 120 s with the step green in CI.
