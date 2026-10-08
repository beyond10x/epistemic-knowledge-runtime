---
format: aep.planning-md/3
id: review-result:hosted-viewer-capacity-final-1
kind: review-result
status: active
title: Viewer bounded stream capacity correction review
relations:
- reviews: story:hosted-read-serving
revision: 1
---
unit: hosted HTTP viewer capacity correction, 29ed6efc83bfd56b494d727006de73881fd62934 → 933928f088df70bfd5c9b72fa5734633b2a07b78
verdict: nothing found in the final correction; observed regression resolved
cases: reviewer executed 0→0, red0; coordinator's unchanged adversary lane executed5→5, failures1→0; focused96passed
origin: introduced viewer regression corrected; no additional finding
wrote-outside-worktree: 1 report
needs-coordinator: resume combined integration gate

Initial `git diff --stat` in the integration tree was empty. Reviewer made no source/test edits and ran no builds or tests. The reviewer authored the earlier shared HTTP queue implementation; the coordinator owns this correction. This is a separate review of the coordinator's proposed fix, not an independence claim about authorship of the original regression.

The recorded full-gate failure is real contract evidence: crates/ekr/tests/adversary_stream_pass1.rs:521 expected each accepted concurrent reader to return200 and observed503 at reader16. Its runner reported4passed1failed0ignored, exit101. The test starts64 loopback readers, verifies every response status and compares every whole NDJSON page with the authoritative engine page. Keep this test byte-for-byte unchanged.

Proposal inspection: a viewer queue matching its64-connection admission cap can hold one pending job from every admitted connection. Stream writing takes place on connection threads after the store thread produces a page, so enlarging this queue does not serialize the streams on the store worker. The queue remains finite; request/header bounds, connection cap, expiration checks before work, waiter deadline and response deadlines remain unchanged. MCP should continue using its16-job queue, including the existing queue-saturation and health-bypass tests.

Suggested tightening: pass the viewer's own IN_FLIGHT_LIMIT constant to queue(capacity), rather than the distinct http::CONNECTION_LIMIT, so a future viewer capacity change cannot silently separate connection admission from queue admission. The value is64 today. Shared http::serve must explicitly pass QUEUE_LIMIT16. Existing queue/deadline/health tests also pass16, retaining their overload refusals and zero stale-job work checks. Documentation must distinguish viewer64 from MCP16; no claim that a full64-connection listener admits a65th health connection is made.

No test was rewritten, relaxed, skipped or rerun by this reviewer. The existing5-case adversary stream lane is the meaningful red/green seam; package lint/fmt and affected HTTP tests are coordinator-owned. A passing targeted lane would address this observed regression, not substitute for resuming the full integration gate.

Owners: coordinator owns the correction and evidence; no additional finding from proposal inspection. Exact final review follows once the source is frozen.

Only outside write: <cache>/ekr-hosted-runtime/serving/viewer-capacity-review.md (local assigned path <cache>/ekr-hosted-runtime/serving/viewer-capacity-review.md). No repository, public plan or credential file was written.

## Final exact review

Inspected every source/document hunk from29ed6efc83bfd56b494d727006de73881fd62934 to933928f088df70bfd5c9b72fa5734633b2a07b78. The complete binary diff SHA256 is6999180ce539c9e050f5b43b1b7acc5524ea29e2fcfbbf872b340101a0f1b487. It changes http.rs, view.rs and docs/cli.md, plus one coordinator-owned AEP evidence JSON. The source implements queue(capacity), viewer queue(IN_FLIGHT_LIMIT) and MCP queue(QUEUE_LIMIT). Existing expiration and health unit tests now explicitly pass their unchanged16capacity. Viewer documentation says64; MCP documentation retains16. No timeout, stream writer, parser, connection admission or store-read behavior changed.

The existing adversary_stream_pass1.rs SHA256 isd8f13b2cb435b119e86c98afe1ec310cd8e1a822f5c84aba6387e66b5aee7bfa both before and after, independently measured from git show and the final file. There is no diff in that test. The reviewer executed no test/build; coordinator command was `cargo test -p ekr --lib --test adversary_stream_pass1 --test hosted_http --test view_stream --test search_page` with its existing bounded gate environment. Coordinator confirmed exit0 from its execution session. Inspected runner summaries in<cache>/ekr-hosted-runtime/release-gate/viewer-capacity-green.log:

```
test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test sixty_four_concurrent_streams_each_deliver_the_whole_page ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.25s
```

Thus the5-case deciding lane moves4passed1failed→5passed0failed with its assertions intact. The focused total is96passed0failed0ignored. This is bounded correction evidence; no full-gate pass is claimed.

Owners: 0 open findings in the final correction. Coordinator owns implementation, test execution, AEP and resuming the full gate. Reviewer owns only this report and released its own review lease at handoff.

```findings
[]
```
