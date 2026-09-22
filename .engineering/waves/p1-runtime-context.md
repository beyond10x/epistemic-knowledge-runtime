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
