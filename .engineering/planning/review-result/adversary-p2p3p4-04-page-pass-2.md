---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-04-page-pass-2
kind: review-result
status: active
title: Adversary pass 2 on wave C unit P (viewer page)
relations:
- reviews: story:data-free-graph-viewer
revision: 1
---
unit: story:data-free-graph-viewer (unit P, page half), pass 2, worktree ekr-wave-c-p at head fc634dd8 plus one untracked test file
verdict: red
cases: executed 12→15, red 3
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/adv2/ (red.log, suite.log, seed.out, view.out, view.err, repl.err, store/, prof/), <scratch>/adversary-pass-2.md
needs-coordinator: no

## 1. git diff --stat

`git --no-pager diff --stat` prints nothing, so no tracked file changed. `git status --short`:

```
?? crates/ekr/tests/adversary_p_page_2.rs
```

That is the only path touched, and it is a test file. I edited no implementation file and made no mutation in the tree.

## 2. Cases added (crates/ekr/tests/adversary_p_page_2.rs)

The correction's `exact()` (index.html:169-175) keeps every integer outside ±2^53 as a **string**. The valid-time code still accepts only `typeof === "number"`. A valid-time bound is an `i64` in milliseconds: ekr-core `Timestamp`, whose docs say "roughly ±292 million years". Each case seeds the unit's `sounding` fixture with assertion a510 set to valid from -2·10^17 to -10^17 ms. `ekr seed` accepts this (exit 0). The case then serves the store with `ekr view --port 0` and reads the DOM that headless Chromium builds (taskset -c 0-1, nice 19).

| case | asserts | now |
|---|---|---|
| `a_valid_time_end_beyond_two_to_the_fifty_three_is_shown_and_not_as_open` | a510's card shows `-200000000000000000 → -100000000000000000` | **red**: it shows `-200000000000000000 → …` (a closed interval shown as open) |
| `an_assertion_that_ended_beyond_two_to_the_fifty_three_before_the_epoch_is_outside_now` | with `?valid=1777593600000` (2026-05-01, confirmed chosen), a510's card has class `claim outside` | **red**: class `claim` (an interval that ended ~3.2 million years ago counts as valid now) |
| `the_valid_time_strip_spans_a_bound_beyond_two_to_the_fifty_three` | `?valid=-150000000000000000`, which lies inside a510's interval, stays chosen | **red**: it is clamped to 2026-03-01T00:00:00Z, because the strip ignores string bounds |

Red run of this file alone. It was written before any other run: `cargo test -p ekr --locked --test adversary_p_page_2`, EXIT=101.

```
running 3 tests
test a_valid_time_end_beyond_two_to_the_fifty_three_is_shown_and_not_as_open ... FAILED
test an_assertion_that_ended_beyond_two_to_the_fifty_three_before_the_epoch_is_outside_now ... FAILED
test the_valid_time_strip_spans_a_bound_beyond_two_to_the_fifty_three ... FAILED

---- a_valid_time_end_beyond_two_to_the_fifty_three_is_shown_and_not_as_open stdout ----
thread '...' panicked at crates/ekr/tests/adversary_p_page_2.rs:187:5:
a510 is valid -200000000000000000 → -100000000000000000; its card shows: <table class="claim">...<tr><td>valid</td><td>-200000000000000000 → …</td></tr>...

---- an_assertion_that_ended_beyond_two_to_the_fifty_three_before_the_epoch_is_outside_now stdout ----
thread '...' panicked at crates/ekr/tests/adversary_p_page_2.rs:210:5:
a510 ended at -100000000000000000 ms and is shown as valid at 2026-05-01: <table class="claim">...<tr><td>valid</td><td>-200000000000000000 → …</td></tr>...

---- the_valid_time_strip_spans_a_bound_beyond_two_to_the_fifty_three stdout ----
thread '...' panicked at crates/ekr/tests/adversary_p_page_2.rs:228:5:
a valid time inside a510's interval was moved off it; the strip shows:  class="muted">2026-03-01T00:00:00Z

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.79s
```

(The card HTML is elided with `...`. The full text is in `<scratch>/adv2/red.log`.) After this run I applied `rustfmt` to the file. Only formatting changed, and the asserts moved to :191, :214 and :232.

## 3. Suite run (after the cases existed)

`cargo test -p ekr --locked --no-fail-fast --test view_page --test adversary_p_page --test adversary_p_page_2`, EXIT=101:

```
Running tests/adversary_p_page.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.16s
Running tests/adversary_p_page_2.rs
test an_assertion_that_ended_beyond_two_to_the_fifty_three_before_the_epoch_is_outside_now ... FAILED
test a_valid_time_end_beyond_two_to_the_fifty_three_is_shown_and_not_as_open ... FAILED
test the_valid_time_strip_spans_a_bound_beyond_two_to_the_fifty_three ... FAILED
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.58s
Running tests/view_page.rs
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 68.54s
error: 1 target failed:
```

The before count of 12 comes from the implementor's correction run (`<scratch>/c1-test.log`: view_page 9 + adversary_p_page 3).

## 4. Findings (tree: fc634dd8 plus crates/ekr/tests/adversary_p_page_2.rs)

Root cause for F1–F3: `exact()` at index.html:172 turns an out-of-range integer into a string, and three readers of valid time were not updated. Before the correction (04550f9a), the same value arrived as a rounded number and all three worked. The base page (6870b8f8) shows no valid time, so the origin is introduced.

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr/src/cli/viewer/index.html:790 | confirmed | introduced | `span()` shows `…` for any `to` that is not a number, so a closed valid interval ending beyond ±2^53 ms is shown as open-ended (case red). The same applies to `recorded_to`. | Any assertion with a valid-time end outside ±285,616 years. `ekr seed` accepts one, and `Timestamp` documents a range of ±292 million years. Nothing in the page's own workflow creates one; the store holds it. |
| F2 | crates/ekr/src/cli/viewer/index.html:305 | confirmed | introduced | `validNow()` maps a string bound to ±Infinity, so a claim that ended at -10^17 ms is drawn valid at 2026-05-01 (no `outside` class; case red). This is a wrong answer in the filter itself. | The same stores as F1, plus a chosen valid time (the slider or `?valid=`). |
| F3 | crates/ekr/src/cli/viewer/index.html:1169 | confirmed | introduced | `fillValidTime()` collects only number bounds, so the strip cannot reach a510's interval and clamps `?valid=-150000000000000000` to 2026-03-01 (case red). | The same stores as F1. |
| J1 | crates/ekr/src/cli/viewer/index.html:560 | plausible | introduced | Reading the code: `flat.camera()` writes the ratio with `toFixed(4)`, so a user who zooms below ratio 5·10^-5 gets `camera=x,y,0` written into the URL, and the corrected `readUrl` (:1289) refuses that on reload. The page writes a camera it will not read. Not measured: headless `--repl` gave no output, so I have no case. | Deep wheel zoom (about 19 notches from the default). No `minCameraRatio` is set. |

Fixes, named only and not applied:
- F1: test for an absent end (`to === undefined || to === null`), not for `typeof number`.
- F2/F3: read a bound that is a number or a string (for example `Number(bound)` for ordering, since rounding beyond 2^53 does not change a comparison against a safe `validAt` in practice), or have `exact()` hand valid-time fields back as numbers.
- J1: write the ratio with significant digits (`toPrecision`), not fixed decimals.

## 5. Attacked and not broken

- The `exact()` scan, by reading, not by running: digits inside strings, escaped quotes, strings ending in `\\`, negative numbers, fractions, exponents, keys that look like numbers and nested arrays all tokenise correctly for serde_json output. The one gap is that an exponent-form integer is not kept exactly, and serde_json never emits one.
- Fetch addresses: evidence is read only as `item.id` from `state.evidenceById` (projection ids), and an unknown `?evidence=` returns before any fetch. `revision` and `valid` are regex-gated, and `node`/`edge`/`assertion` are looked up in maps.
- Load sequencing: a stale success and a stale failure are both dropped (`mine !== loads`), and roles errors are caught inside `readRoles`.
- Evidence cache across a revision change: `/evidence/<id>` serves the head's bytes regardless of revision, `retained:false` returns before the cache, a failed read is evicted, and I found no CLI verb that removes retained bytes while a page is open.
- Palette and keyboard: an empty result list with ArrowDown/Enter runs nothing; Ctrl+K while open does not throw.

## 6. Paths written outside the worktree

- `<scratch>/adv2/`: red.log, suite.log, seed.out, view.out, view.err, repl.err, store/ (probe store), prof/ (probe browser profile)
- `<scratch>/adversary-pass-2.md` (this report)
- Temporary directories that tempfile created during test runs are removed on drop. The probe server and browser were stopped by PID.

## 7. Findings block


## Coordinator routing (2026-09-27)

- F1, F2, F3 → back to the implementor: every time bound is compared through one helper that accepts a number or the exact-text string, and an open end is tested as missing, not as not-a-number.
- J1 → back to the implementor: the zoom ratio is written with `toPrecision`.
- Second pass: the correction is verified by the coordinator, no third attack.

```findings
[
{"file":"crates/ekr/src/cli/viewer/index.html","line":790,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a closed valid interval whose end is beyond 2^53 ms is shown as open-ended because exact() made the end a string"},
{"file":"crates/ekr/src/cli/viewer/index.html","line":305,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"validNow treats a string valid-time bound as unbounded, so an assertion that ended at -10^17 ms is drawn valid today"},
{"file":"crates/ekr/src/cli/viewer/index.html","line":1169,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the valid-time strip ignores string bounds, so a valid time inside a deep-time interval is clamped off it"},
{"file":"crates/ekr/src/cli/viewer/index.html","line":560,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"flat.camera() writes the zoom ratio with toFixed(4), so a ratio below 5e-5 is written as 0, which the corrected readUrl refuses"}
]
```
