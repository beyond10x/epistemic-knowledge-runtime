---
format: aep.planning-md/1
id: story:synchronous-runtime-refusal
kind: story
status: implemented
title: Return a named refusal for synchronous store use inside Tokio
relations:
- serves: vision:o2
- decomposes: epic:p1-kernel-ontology-core
scope:
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary_runtime_context.rs
- confidence: cited
  path: crates/ekr-store/tests/runtime_context.rs
revision: 10
---
## Outcome

Close task:ekr-store-block-on-cannot-nest using its already approved named-refusal
option. Synchronous provider construction and I/O detect an entered Tokio runtime
and return StoreError::RuntimeContext before opening files or invoking callbacks.
Normal synchronous callers retain their existing behavior.

A store opened synchronously may be moved into an async caller by ordinary Rust
ownership. Its destruction must not cause a second panic after a named refusal.
Use explicit owned-runtime shutdown that is safe in that context; do not spawn
detached writes or hide a still-running synchronous call. This closes the bounded
panic behavior, not the later design question of a fully async runtime API.

## Acceptance and scope

Both File and SQLite constructors refuse in an entered runtime before creating
their source path. A pre-opened store refuses reads, writes, initialization and
lineage operations without unwinding, calling supplied authority, or changing
stored data; normal operations still succeed afterward. Destruction in that
context does not unwind. Cover a plain entered Handle as well as a running
current-thread executor where relevant.

Source: crates/ekr-store/src/eventlog.rs and crates/ekr-store/src/lib.rs.
Cases: crates/ekr-store/tests/runtime_context.rs (new). Existing store/kernel
seed acceptance remains required. Root implements in a separate managed tree;
independent adversarial review and full integrated gate precede publication.
This source is disjoint from the active source-guard unit's tests. Persisted
writer activation is later and must inherit this runtime boundary.
No manifest, dependency, ESS, persistence format or live-store change is needed.

## Implementation and review

# Synchronous runtime refusal, 2026-09-22

Source 51c7511, integration 594efa8, story:synchronous-runtime-refusal.
The existing task's named-refusal option keeps the synchronous store boundary.
Both constructors and all public persistence methods now return RuntimeContext
on an entered Tokio thread before provider work, authority or I/O.
Dropping an owned store inside an entered runtime shuts down without blocking
that runtime. Completed synchronous writes remain readable after reopening.

The initial running-runtime read test panicked at nested Runtime::block_on,
exit101. A separate idle entered-handle test returned success before the fix;
that case establishes the deliberately conservative new refusal contract,
not a reproduced nested-runtime panic. An initial test compile mistakenly
named a nonexistent storage class and was corrected to Ephemeral; it is not
counted as a behavioral failure.

The coordinator's ekr-store suite passed66, including six new tests. Independent
review added five attacks, each green on first execution, then ran the same
suite at71 passed, zero failed and zero ignored. Existing source/test hashes
were unchanged. Formatting and strict package Clippy exited0.
The full report is .engineering/reviews/p1-runtime-context-adversary.md.

The coordinator then removed initialize's entry guard. The independent populated
File lineage case failed its named-refusal assertion, exit101 with one executed
failure. Exact restoration matched all three original SHA-256 values and that
case passed again. Full workspace integration remains the closing gate.

These tests use substitute provider authority only to observe callbacks; they
do not claim canonical seed validation. No current async product caller was
invented: the current CLI is synchronous, and the repair covers the exposed
store API used from a Tokio context.

The reviewer disclosed one read-only AEP show command before reading its complete
charter, which prohibited all AEP calls. It made no planning mutation; the
coordinator retained this deviation and explicitly prohibited it in the next
dispatch. It does not change the reported empty implementation findings.

Owners: coordinator, production fix and six cases; reviewer, five cases and
independent report; coordinator, restored mutation and integration. This host
did not expose per-agent token/tool totals; none is estimated.
