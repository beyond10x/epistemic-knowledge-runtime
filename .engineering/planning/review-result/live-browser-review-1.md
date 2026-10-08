---
format: aep.planning-md/3
id: review-result:live-browser-review-1
kind: review-result
status: active
title: Live browser review catches document-lifecycle harness race
relations:
- reviews: story:live-search-agent-entry
revision: 1
---
unit: live search at 5ebde9dafdfde90cfad192c7abd16c38e670c49c; review tree 335673a9dba2d92704a366ec531d4ef957fa8fd4 has identical content
verdict: CONFIRMED
cases: executed 9→11, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned review scratch and sequentially reused idle unit target; private path inventory retained separately
needs-coordinator: repair the browser harness before the release gate

## Test-only change

```text
 crates/ekr/tests/search_live.rs | 75 +++++++++++++++++++++++++++++++++++++++++
 1 file changed, 75 insertions(+)
```

The additions are retained in bot commit 3e98343b2ad0808d7ef5221924e575c8590509da.
No implementation file or existing assertion changed. The coordinator performed this
tests-only role in the separate review checkout because a new worker thread was refused
at the thread limit. This report does not claim agent independence or approval.

## Cases written before execution

`adversary_clearing_an_inflight_read_keeps_the_empty_state_after_late_failure` clears
an intercepted active read, releases a late failure, checks the empty state and canceled
network request, then proves a subsequent successful query still renders.

`adversary_reserved_characters_round_trip_as_one_query_without_navigation` types Unicode
and reserved query characters through real browser input, checks the exact encoded request,
serves the real renderer response, and checks retained focus/value, one module, no result
nodes and no navigation.

First command, before the combined suite:
`cargo test --locked -p ekr --test search_live adversary_ -- --nocapture`.
Receipt: `additional-cases.log` and `additional-cases.exit`, exit 0.

```text
running 2 tests
test adversary_clearing_an_inflight_read_keeps_the_empty_state_after_late_failure ... ok
test adversary_reserved_characters_round_trip_as_one_query_without_navigation ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 3.76s
```

## Subsequent suite and finding

Command: `cargo test --locked -p ekr --test search_live --test adversary_agent_guidance -- --nocapture`.
The browser was required, with two test threads and two build jobs. Receipt: `suite.log`
and `suite.exit`, exit 101. The original complete log is retained without edits; its
runner summaries and failure are quoted below, excluding private compiler paths.

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

thread 'composition_suppresses_partial_queries_and_no_script_get_still_works' (2221710) panicked at crates/ekr/tests/search_live.rs:209:9:
DOM.querySelector: {"error":{"code":-32000,"message":"Could not find node with given id"},"id":87}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.15s
```

Measured: the new browser harness failed on a stale CDP node id. The implementation's
prior successful browser runs show that this is intermittent, not that the failure is
safe to ignore. Reachable caller: the ordinary scripting-disabled form submits on Enter
at `search_live.rs:509`, followed immediately by `until` reading the DOM at :510.
`node` reads the document id then queries it in a separate protocol command at :233–242.
A navigation between those commands is the inferred mechanism; the stale-id refusal
itself is directly observed. No user-visible product failure is established.

The finding is introduced with this candidate's new browser harness, and holds the
release gate until the harness observes document lifecycle without weakening its search,
focus, history or no-script assertions. A bounded lifecycle retry must discriminate
transient detached-node errors from unrelated protocol errors.

Owners: 1 finding, 0 coordinator, 1 implementor.

## Scope and retained paths

The two added browser cases passed; existing static guidance cases passed. The suite
also exercised typing, cancellation, historical links and asset admission, but its one
failure means this pass is not a green suite. The full workspace gate was not run.
The cancellation case does not force an aborted fetch to complete successfully; the
separate generation-state test remains distinct evidence for continuation authority.

Private operational inventory: assigned `ekr-search-live-review` scratch holds
`additional-cases.log`, `additional-cases.exit`, `suite.log`, `suite.exit` and the path
inventory. The idle `ekr-search-agents-unit` target was reused sequentially with the
implementor's explicit handoff; no concurrent build used it. Public source carries no
private paths or provider data.

```findings
- file: crates/ekr/tests/search_live.rs
  line: 233
  category: concurrency
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: The new browser harness can query a detached document node during the ordinary no-script Enter navigation and fail the acceptance suite with a stale CDP node id.
```
