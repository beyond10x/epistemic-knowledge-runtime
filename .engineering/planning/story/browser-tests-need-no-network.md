---
format: aep.planning-md/3
id: story:browser-tests-need-no-network
kind: story
status: implemented
title: Browser tests load the pinned graph libraries without the network
relations:
- serves: vision:o5
scope:
- confidence: cited
  path: crates/ekr/tests/adversary_p_page.rs
- confidence: cited
  path: crates/ekr/tests/adversary_page_stream_1.rs
- confidence: cited
  path: crates/ekr/tests/adversary_viewer_compact.rs
- confidence: cited
  path: crates/ekr/tests/adversary_viewer_follow_ups.rs
- confidence: cited
  path: crates/ekr/tests/search_live.rs
- confidence: cited
  path: crates/ekr/tests/view_page.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T06:22:52Z", actor: "agent:claude-ekr-controller", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-07T06:22:52Z", actor: "agent:claude-ekr-controller", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-07T07:33:54Z", actor: "agent:claude-ekr-controller", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
## What is wrong

Every browser test loads the viewer page, and the page loads its four pinned graph libraries from
`cdn.jsdelivr.net` (`crates/ekr/src/cli/viewer/index.html:7-10`). Each launch uses a fresh profile,
so each launch downloads about 1.74 MB from the network, and a failed download leaves the page at
"the viewer did not start: the graph libraries did not load". The case then times out on a wait
that can never come true.

Observed:

| when | where | what failed |
|---|---|---|
| 2026-10-06, `main` Correctness run 37529812043 | `view_page.rs:2964` | library request ended `net::ERR_CERT_VERIFIER_CHANGED` |
| 2026-10-07 00:53Z, local `task check` at load ~30 | `adversary_page_stream_1.rs:968`, 2 of 9 cases | the dumped DOM shows "the graph libraries did not load"; no network error was captured |

Pull request 84 stopped component installs and background requests in every launch, which removes
the certificate-verifier change; the inferred cause of the first row. It does not remove the
dependency on the network: the second row happened after it, and the same binary passed 9 of 9 in
the run before.

## Build

Serve the four pinned files to the browser from bytes held by the test process, checked against the
page's `integrity` attributes, so no browser test reaches the network. The page and its
Content-Security-Policy stay as they are; only the tests' harness changes.

## Acceptance

- With the library host unresolvable for the browser (for example
  `--host-resolver-rules=MAP cdn.jsdelivr.net ~NOTFOUND`), every browser test in `crates/ekr/tests`
  passes.
- A served file whose bytes do not match the page's `integrity` attribute makes the page refuse it,
  and a case shows that.

## Scope

- `crates/ekr/tests/view_page.rs` — cited (launch sites at the `"--headless"` argument lists)
- `crates/ekr/tests/adversary_viewer_compact.rs` — cited
- `crates/ekr/tests/adversary_viewer_follow_ups.rs` — cited
- `crates/ekr/tests/adversary_p_page.rs` — cited
- `crates/ekr/tests/adversary_page_stream_1.rs` — cited
- `crates/ekr/tests/search_live.rs` — cited
- where the pinned bytes come from (a test fixture or a fetch once per process) — not decided
