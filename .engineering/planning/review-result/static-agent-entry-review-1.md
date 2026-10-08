---
format: aep.planning-md/3
id: review-result:static-agent-entry-review-1
kind: review-result
status: active
title: Bounded adversarial review of static agent entry
relations:
- reviews: story:live-search-agent-entry
revision: 1
---
unit: static agent-guidance half at 798b7d5fc776143390da5e4f0a6114bef86cc800
verdict: nothing found
cases: executed 3→7, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 20 paths; exact private inventory in separate outside-paths.txt annex
needs-coordinator: remaining live-typing implementation and broader compatibility/lint/full gate; no release acceptance
 crates/ekr/tests/adversary_agent_guidance.rs | 221 +++++++++++++++++++++++++++
 1 file changed, 221 insertions(+)

## 1. Scope and diff

Only `crates/ekr/tests/adversary_agent_guidance.rs` was added. No implementation or AEP edit,
commit, push or Cargo build ran. Base: `7e71751e226c62d6f2247b03ffb5bcf492622989`.
The complete seven-file candidate delta, acceptance, written tests and touched callers were read.
The repository-local diff shown immediately after the header is test-only.

## 2. Cases written before execution

Four std-only process cases were added before any probe execution, then compiled with
`rustc --edition=2024 --test crates/ekr/tests/adversary_agent_guidance.rs -o <scratch>/probe`.
Compilation exit0. The process fixture resolves repository files at runtime and uses the assigned
scratch TMPDIR. Every child was stopped/reaped and each temporary fixture removed by its guard.

- `corrupt_store_never_changes_static_guidance_or_fabricates_readiness`: corrupt SQLite bytes;
  positive200 static guide/discovery before and after503readiness; unchanged guide and original
  corrupt bytes; configured response headers and503search still showing agent connection.
- `static_routes_reject_duplicate_authority_framing_and_queries_before_store_access`: both routes;
  absent/duplicate/foreignHost421,HEAD/DELETE405,duplicateContent-Length andTransfer-Encoding413,
  empty/nonemptyquery400; refusals omit configured endpoint, then zero-bodyGET200 advertises it.
- `maximum_url_is_preserved_and_one_extra_byte_is_refused`:4096-byte endpoint preserved literally
  in guide/discovery,4097refused withCLIexit2/noannouncement for both flags; HTML escaping and
  Markdown parenthesis encoding for operator guide metadata.
- `explicit_require_ready_still_refuses_before_url_despite_static_guides`: explicit ready startup
  fails without listener announcement/storecreation; ordinary lazy startup still serves guide200
  andready503 with the same endpoint configuration.

No case failed on its first execution. Exact individual output follows. Each command was
`PROBE_EKR=<candidate> CARGO_MANIFEST_DIR=<review>/crates/ekr TMPDIR=<scratch>/tmp <scratch>/probe --exact <case> --nocapture`;
each independently captured exit was0. Timeouts only bound transport waits; no elapsed-time
performance assertion was added.

```text

running 1 test
test corrupt_store_never_changes_static_guidance_or_fabricates_readiness ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

```text

running 1 test
test static_routes_reject_duplicate_authority_framing_and_queries_before_store_access ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.01s

```

```text

running 1 test
test maximum_url_is_preserved_and_one_extra_byte_is_refused ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.01s

```

```text

running 1 test
test explicit_require_ready_still_refuses_before_url_despite_static_guides ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.01s

```

## 3. Bounded suite after the new cases

The before count3 is the implementor-reported std-only process driver, not an inferred count
from source attributes and not a preemptive suite run. After the four new cases existed and ran
individually, that exact existing driver was rerun and the new four-case suite ran together:

```text
PROBE_EKR=<candidate> PROBE_REPO=<review> PROBE_SCRATCH=<scratch>/tmp <unit-scratch>/http-probe --test-threads=1 --nocapture
```

```text

running 3 tests
test explicit_mcp_and_guide_options_reach_the_listener ... ok
test find_links_to_connection_guidance_without_a_store ... ok
test guides_work_with_a_missing_store ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

```
Exit0.

```text
PROBE_EKR=<candidate> CARGO_MANIFEST_DIR=<review>/crates/ekr TMPDIR=<scratch>/tmp <scratch>/probe --test-threads=1 --nocapture
```

```text

running 4 tests
test corrupt_store_never_changes_static_guidance_or_fabricates_readiness ... ok
test explicit_require_ready_still_refuses_before_url_despite_static_guides ... ok
test maximum_url_is_preserved_and_one_extra_byte_is_refused ... ok
test static_routes_reject_duplicate_authority_framing_and_queries_before_store_access ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

```
Exit0.

Aggregate bounded run:7passed,0failed,0ignored,0filtered. Individual runs intentionally filtered
three other new cases apiece. The implementor's16-case Cargo search_page result was read, not
rerun here; this report does not claim20cases or a whole-package run. Candidate binary SHA256
`ff550d7740bbb60e4174321a00fbd280721759d5310ded48288a46aa4e6d44de` was checked before execution.
The supplied released base binary hash was also read; it was not executed in this pass because
there is no failing finding to classify. No origin is guessed from a different release binary.

## 4. Findings

Nothing found in the bounded static-guidance half.

Owners: 0 findings, 0 coordinator, 0 implementor.

## 5. Attack limits

- Corrupt/unavailable storage did not change static guidance or fabricate admitted readiness.
- Duplicate/absent/foreign authority, method, framing and query refusals held on both static routes.
- Maximum advertised URL length, one-byte overflow and HTML/Markdown delimiter boundaries held.
- Explicit require-ready still refused startup before an announcement.
- Static dispatch and URL configuration were reviewed; no endpoint is contacted or automatically
  discovered by these options, and no stored record or evidence payload enters the static renderer.
- No browser live-typing implementation, actual remote MCP connection, deployment authentication,
  load/queue saturation, broader viewer compatibility, package lint or full gate was verified.
  This is partial review, not story completion or an approval to merge/release.

## 6. Outside files

Only the assigned scratch was written outside the worktree. It contains probe, tmp/, four
individual .log/.exit pairs, baseline-driver.log/.exit, adversary-suite.log/.exit, diffstat.txt,
tests.patch, report.md, scan.log/.exit and outside-paths.txt. The exact personal absolute paths are
recorded in the separate private annex, not this publishable report. No Cargo target was created.
Existing unit binaries were read/executed without modification. Scratch and tests remain for the
coordinator; the review lease is released at handoff, with no worktree/cache deletion.

```findings
[]
```
