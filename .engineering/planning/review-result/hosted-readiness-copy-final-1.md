---
format: aep.planning-md/3
id: review-result:hosted-readiness-copy-final-1
kind: review-result
status: active
title: Read-only viewer cleanup correction review
relations:
- reviews: story:hosted-read-serving
revision: 1
---
unit: story:private-network-http-serving — read-only viewer cleanup regression correction
verdict: pass, bounded static review
Owners: 0 findings, 0 coordinator, 0 implementor.
findings: []

Reviewed commits 0b3520d2fbb37385939bc1616e4cc41e130fbb82 through
29ed6efc83bfd56b494d727006de73881fd62934, specifically
crates/ekr/tests/adversary_read_only_store.rs:375–430 and the listener/admission implementation.
Combined test-file diff SHA256:
eb9528bd555cedd95a8718c54bd300d86b7681323db8e85c66d7bda920403217.
No repository edits, builds, test execution or fixture use by this reviewer.

The original assertion raced a changed lifecycle: view.rs:190 announces a bound socket before
the first queued store request opens Held at :204. Merely reading the announcement cannot prove
that the read-only file-store copy has been created. The new GET /readyz supplies the exact
announced authority as Host and requires HTTP 200 before inspecting the copy directory.
The immediate health/page bypass does not include /readyz; the ready route at view.rs:846
requires an admitted runtime with a seeded head. This is synchronization on actual store
admission, not a sleep or weakened copy-count expectation.

The final JSON parsing matches http.rs:102–115: announce emits a flushed JSON object with a
url string, including http:// and trailing slash. Stripping only that known framing yields the
authority needed by TcpStream and Host. Invalid output remains a hard test failure.
The original exact-one-copy assertion is retained before TERM, and exact-zero-copies remains
after kill succeeds and child.wait observes termination. Read-only permissions and the private
TMPDIR remain unchanged. No skip, retry-to-hide-refusal, looser count or product behavior was added.

Evidence reviewed, not executed here:
- Coordinator reports original CI run 37092103530 failed the immediate count (0 versus 1).
  This review independently confirms the ordering cause in source; it did not fetch that CI log.
- release-gate/readiness-fix-announcement-error.log records the intermediate parser attempt:
  9 passed, 1 failed, no ignored cases; Option::unwrap failed while interpreting announcement text.
- release-gate/readiness-fix-green.log records the corrected full target: 10 passed, 0 failed,
  0 ignored, 0 filtered, including the cleanup case, finished in 0.27 seconds.
The focused log is not evidence of the full combined gate or rerun remote CI; coordinator owns
those gates and publication. The private instance adoption patch was not touched during review.
