---
format: aep.planning-md/3
id: story:ekr-viewer-fixes
kind: story
status: active
title: Viewer keeps pinned links, opens offline, browser tests need Chromium
summary: The graph viewer keeps pinned links and the address in step with the page, opens without external script loads, and its browser tests fail when Chromium is missing on the forge
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:29:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T08:29:12Z", actor: "human:timo", revision: 3}
---
## Outcome

The graph viewer keeps pinned links and the address in step with the page, opens without external script loads, and its browser tests fail when Chromium is missing on the forge.

## Work

Review of EKR 0.0.42, findings 24 25 26 39. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

24. Opening a search result can turn a pinned graph link into a moving one. Chromium opened the revision-0 link returned by /find. The viewer removed the revision from its address because 0 was then head. Reloading after a commit showed revision 1. Reopening the original link still showed 0. Preserve a revision supplied in the URL even when it equals the current head.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/search_page.rs#L87
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/viewer/index.html#L2459
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/viewer/index.html#L3631


25. Submitting a search leaves the previous search in the address bar. From /find?q=Bob&revision=0, submitting Alice in Chromium showed Alice, but the URL still contained Bob. Reloading restored Bob. Update the submitted query in the URL and preserve the chosen revision. The native HTML GET form already does this.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-search-web/src/browser.rs#L76
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-search-web/src/browser.rs#L202
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/search_page.rs#L46


26. The local graph viewer needs four external script loads to open. With external requests blocked, local /find returned Bob but its graph link displayed a startup error. Serving the four requested libraries from their checked-in fixtures made the same browser render the graph. Bundle or locally serve the pinned libraries for local operation. The HTML search and JSON reads already work offline.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/viewer/index.html#L7
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/viewer/index.html#L742
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/support/viewer_libraries.rs#L29


39. Six browser acceptance tests can pass locally without starting Chromium. The browser helper allows tests to return early if it cannot find the browser. The forge requires it through its environment. The shared local test task does not make that requirement. Have the shared browser gate report the executed count and fail when its required browser cannot start.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/search_live.rs#L305
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/search_live.rs#L535
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/search_live.rs#L1011
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/.github/workflows/correctness.yml#L87
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/Taskfile.yml#L46


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
