---
format: aep.planning-md/3
id: review-result:hosted-http-static-1
kind: review-result
status: active
title: Hosted HTTP transport static adversarial review
relations:
- reviews: story:hosted-read-serving
revision: 1
---
unit: story:hosted-read-serving — working candidate against 37763e2ed24dcddcd1d944fe563d07736372f587
verdict: NEEDS-CHANGE — two static contract findings sent to implementor
cases: executed 0→0, red not executed in this static pass
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: one report
needs-coordinator: retain implementor reproduction and correction evidence with this original review

Owners: 2 findings, 0 coordinator, 2 implementor.

Role assignments: implementation and targeted execution — pilot_runtime; static review — scope_hosted_ekr; integration, AEP and final gate — root.

1. Diff and scope

Reviewer changed no source or tests. The candidate already held these author changes:
```
crates/ekr/src/cli/mcp.rs           +102 -0
crates/ekr/src/cli/mod.rs           +50 -6
crates/ekr/src/cli/session.rs       +1 -0
crates/ekr/src/cli/view.rs          +146 -74
crates/ekr/src/main.rs              +4 -1
crates/ekr/tests/agent_cli.rs       +24 -1
crates/ekr/tests/read_only_store.rs +3 -2
crates/ekr/tests/view_cli.rs        +7 -8
docs/cli.md                       +75 -15
new crates/ekr/src/cli/http.rs
new crates/ekr/tests/hosted_http.rs
```
Candidate snapshot hashes at inspection:
http.rs 8dab71858b8d6f415f1ec50b882b44f2b23304c7d37fe786f757563aa53cd8b9
mcp.rs 8984c70a517c616ec6de2673ec08fa36c89104cd7d2b6d9e94b834358de07595
view.rs da6ed595ac73f795f86605aed6c15dfab75573c97228493b661e21d7558a2f13
hosted_http.rs 82648e8661a40785898678b1d83e4e091841d0d4305ab76995fdfdca08f1d3c3

2. Added cases and execution

None by this reviewer. No Cargo build, PostgreSQL fixture operation or socket probe was run.
The assigned pass was static while the implementation agent held the build lane. Findings below
are source-control-flow observations, not independently executed failures. The implementation
agent acknowledged the ordering issue and is adding a missing-store process regression before
changing it; its red/green output must be attached separately, not inferred from this report.

3. Findings

| Surface | Verdict / origin | Finding and reachable condition |
|---|---|---|
| crates/ekr/src/cli/mcp.rs:982 | NEEDS-CHANGE / introduced | Initial store admission precedes transport routing and validation, so an unavailable initial store turns documented method/path/protocol refusals into HTTP503. Reachable by starting mcp-http with a valid host configuration and a missing local store, then sending GET or DELETE /mcp, or a bad protocol request. The serve closure opens Held before Server::http can return405/404/400. Existing protocol/method tests used seeded stores only. |
| docs/cli.md:1307 | NEEDS-CHANGE / introduced | The broad claim that an unavailable or incomplete store returns503 does not distinguish initial admission/readiness from accepted MCP tool calls, whose errors are serialized with HTTP200. Reachable after the listener has admitted a store and a later tool read fails, or when a held provider is incomplete; mcp.rs:1063-1068 maps every dispatched JSON-RPC response to200. Readiness itself correctly returns503. Preserve MCP error parity and document this distinction. |

Severity is warning for both: these are externally observable contract inconsistencies, not a
demonstrated data-admission bypass. The implementation agent is correcting them before merge.
Introducedness is established by the base having no mcp-http transport or corresponding prose.

4. Attacked surfaces without a further static finding

- Authority: one exact admitted Host, no forwarded-header grant, non-loopback requires explicit authorities; IPv6 authorities validated; present duplicate/unapproved Origin rejected.
- Framing: duplicate/malformed Content-Length and any Transfer-Encoding refused; header/body byte caps; no keep-alive request reuse; total read and write deadlines, not per-chunk extensions.
- Admission:64connections,16queued jobs, nonblocking queue admission; job deadline checked before store work. A running operation can outlive the waiting client, but no unbounded queue accumulates behind it.
- Health: bypasses store queue and lazy store admission, so unavailable/incomplete stores do not by themselves prevent health responses. Connection saturation still returns busy as documented.
- Readiness: requires head Some including seed revision zero; missing/incomplete/read errors fail closed; local source replacement clears caches and retries; no new graph index is built merely for readiness.
- Parity/history: HTTP dispatch uses existing Server::message and the same nine read-only tools; explicit historical reads use existing IndexCache authority; no writer method introduced; existing stdio transport remains separate.
- Protocol policy: explicit supported revision required after initialization, with missing-header behavior documented; no invented newest-version fallback, sessions or SSE replay semantics.

5. Limits and artifacts

This is not a full protocol conformance certification or an execution verdict. No claim is made
about new runtime scenarios beyond the inspected control flow. Product fixes and dynamic proof
belong to the implementation agent; root owns the final combined gate.

Only outside-tree write: the assigned search/http-review.md report. No shared source edits,
planning writes, builds or fixture use. Reviewer lease is released after this report.

```findings
- file: crates/ekr/src/cli/mcp.rs
  line: 982
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Initial store admission precedes transport routing and validation, so an unavailable initial store turns documented method/path/protocol refusals into HTTP503.
- file: docs/cli.md
  line: 1307
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The broad claim that an unavailable or incomplete store returns503 does not distinguish initial admission/readiness from accepted MCP tool calls, whose errors are serialized with HTTP200.
```
