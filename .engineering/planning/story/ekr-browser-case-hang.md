---
format: aep.planning-md/3
id: story:ekr-browser-case-hang
kind: story
status: active
title: No browser case hangs
summary: Every browser case ends in seconds on every run or fails naming what it waited for
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T13:09:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T13:09:24Z", actor: "human:timo", revision: 3}
---
## Outcome

No browser case hangs: every case that passes in seconds on one CI run ends in seconds on every
run, or fails with a message naming what it waited for.

## Work

On CI run 45394513513 (release PR #134, 2026-11-24 12:47 UTC) the case
`adversary_page_stream_1::store_text_through_every_address_and_a_split_stream_never_becomes_html`
ran past nextest's 60 s bound and was killed. On the run of #130 at the same code base minus the
version bump it passed in 3.75 s. Earlier, on 2026-11-24, it took 10.6 s. #129 changed the
virtual-time harnesses (`pump` reads the DevTools socket between checks; `wait_for` runs inside the
page via `shared_browser::wait_in_page`); a wait that never sees its condition, or a pump that
blocks on a socket read with no message pending, would hang the case like this.

Find the hang: run the case in a loop under slot on the build server (and with the CPU loaded, as
on CI) until it hangs, then read where it waits. Give every wait in the harnesses a bound that
fails the case with a message naming the condition (well under 60 s), and fix the cause.

## Acceptance

The case runs 200 times in a row under load without exceeding 15 s; a forced stall (for example a
page that never signals) fails within the bound with a message naming the condition; no assertion
removed.
