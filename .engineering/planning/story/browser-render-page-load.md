---
format: aep.planning-md/3
id: story:browser-render-page-load
kind: story
status: draft
title: Browser cases under 150 s summed
summary: One page load per browser case where it can be, measured on CI
relations:
- serves: vision:o5
revision: 1
---
## Outcome

The browser cases of EKR's test step take under 150 s summed on CI, down from 420-535 s, with the
same assertions. That is the largest share left in `story:ekr-test-step-budget`.

## Work

Measured after PRs 115 and 116 (CI run 45314673716): 82 browser cases, 444 s summed, 159 s from the
first start to the last end. `view_page` alone is 237 s.

- The cost is the page load per render. Each render opens a new browser context, loads the page and
  its libraries through Fetch interception, and draws with SwiftShader. The 8,000 ms virtual-time
  budget is not the cost. Measured locally on the instrumented `view_page` evidence case at budgets
  of 8,000, 2,000 and 500 ms, each render's settle took the same 3-11 s of wall time, and the case
  passed at each budget. The page's "settled" condition (`SETTLED`) did not hold in 38 of 39
  renders, so a settle that ends on that condition does not apply.
- The seeded stores are already made once per build (`support/seeded_store.rs`, PR 116).
- Cut the page loads: one page per case that moves between addresses by hash where the case allows
  it, or keep the libraries' compiled code between contexts. Measure each on CI before and after.

## Acceptance

On CI the step prints browser cases under 150 s summed. No browser assertion is removed or
weakened.
