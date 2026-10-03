---
format: aep.planning-md/3
id: review-result:hosted-http-final-2
kind: review-result
status: active
title: Hosted HTTP correction review and direct process tests
relations:
- reviews: story:hosted-read-serving
revision: 1
---
unit: story:hosted-read-serving — corrected working candidate against 37763e2ed24dcddcd1d944fe563d07736372f587
verdict: nothing found in the correction delta; original two findings addressed
cases: executed13→13, red0; direct replay of existing corrected target, no added cases
origin: introduced0 / pre-existing0 / undecided0 remaining
wrote-outside-worktree: final report, direct-run log and owned temporary fixture directory
needs-coordinator: final combined workspace gate remains required

Owners: 0 findings, 0 coordinator, 0 implementor.

Role assignments: implementation/corrections — pilot_runtime; this review/direct execution —
scope_hosted_ekr; integration and final gate — root.

1. Delta and identification

Reviewer source/test diff: none. Original http-review.md is retained unchanged, including both
introduced findings. This follow-up covers the correction, not a new blanket approval.
Frozen complete tracked-plus-new patch SHA-256:
42cfbc620034a237a20b33ffcc85e0d44d37b5f8c629e898804439406dab0e47
(serving/final-candidate.patch; all11file hashes retained in serving/final-files.sha256).

Inspected current source SHA-256:
```
crates/ekr/src/cli/mcp.rs
8eb827ad7e5b76ffb74028fe8e44e3ddc52a919a79dc09dda1b855e7c4799d1a
docs/cli.md
6cb69d6f7c9cf5e64510c6765145ac7660acce8ed06f9c98fb9512867b18ac4a
crates/ekr/tests/hosted_http.rs
d7b3b51e75f1b635c648082daacfe180dfbc77ac9d39371c8c75638c7498099b
crates/ekr/src/cli/http.rs
be606b3aafa16ea8378d427551a7de32b9c98793645c448bfd9a76300d0dd9d5
```
The final http.rs delta names the same bounded sender/receiver tuple Queue<T,R> at338 and uses
that alias as queue()'s return type; sync_channel(QUEUE_LIMIT) is unchanged. This is statically
inspected, not claimed recompiled by the direct test run below.

2. Original findings and correction evidence

- Original mcp.rs:982 admission-before-transport finding: corrected mcp.rs:982 calls
  http_request(&request) and returns its refusal before Held::open. That helper owns endpoint,
  method, readiness-body, content-type, Accept, version and blank-body checks. Existing Host,
  Origin and framing checks still run in the connection layer. Owner regression
  unavailable_store_does_not_override_transport_refusals exercises a missing store and expects
  GET/DELETE405, unknown404, POST readiness405, bad protocol400, readiness503 and health200.
  Retained serving/admission-order-red.log:0passed1failed0ignored12filtered; GET /mcp actual503
  versus expected405. Retained serving/admission-order-green.log:1passed0failed0ignored12filtered.
  These logs are owner-produced evidence and were read by this reviewer.
- Original docs/cli.md:1307 blanket503 finding: corrected1307-1310 explicitly distinguishes
  failed initial admission/readiness503 from accepted tool-call JSON-RPC/tool errors under200.
  That prose matches the retained Server::http dispatch and preserves stdio result parity.

3. Direct execution by this reviewer

No Cargo/build or PostgreSQL fixture operation. Ran the existing assigned compiled target from
the HTTP worktree with runtime CARGO_MANIFEST_DIR set to that tree's crates/ekr and own TMPDIR:
```
env TMPDIR=<assigned-search-scratch>/http-review-tmp \
    CARGO_MANIFEST_DIR=<http-worktree>/crates/ekr \
    <http-target>/debug/deps/hosted_http-52d5f9975019280a \
    --test-threads=1 --nocapture
```
Test binary SHA-256:38bcdd8b2b50842e3f3c9e88f60b99012bbdbed1786972497b03313eab520fe4
Child ekr binary SHA-256:8db753c5dddd1cfa1b3bb5ee4d75b2e09f37d5cf8d6f11442541580f29e8f062
Exact run log: search/http-final-tests.log. Runner summary, verbatim:
```
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.58s
```
Process exit0 observed. This replays the owner's13-case lane; count did not increase because
no new case was authored in this correction review. The direct run includes the new missing-store
status regression, incomplete-copy/checkpoint readiness, local replacement retry, external
commit/historical reads, read-only file preservation, nine-tool parity, and protocol/authority
and framing cases. It does not claim PostgreSQL serving or the full workspace gate.

4. Residue and artifacts

No unresolved finding in these two correction surfaces. Original general static review limits
remain; final package clippy and combined release gate belong to their assigned owners.

Outside-tree writes under the assigned search scratch: http-final-review.md,
http-final-tests.log, http-review-tmp/ (temporary synthetic stores created by the direct run).
No source/planning edits, new cases, Cargo outputs or live data. Own review lease released.

```findings
[]
```
