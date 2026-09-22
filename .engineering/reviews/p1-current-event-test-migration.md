# Current event test migration

Coordinator-owned integration work over the frozen format/seed checkpoint.
Only crates/ekr-graph/tests/revision_events.rs changes; product encoders and
the frozen legacy family remain unchanged.

The existing cases now construct RevisionEvent's versioned envelope around
RevisionPayload. They still check payload kind indices, domain names, variant
separation, deterministic encoding and exhaustive declaration/index/fixture
agreement. The source guard locates the payload's own implementation before
reading variant_index, so the envelope's delegating method cannot make it vacuous.

A new independently transcribed byte vector holds the current envelope's field
order and all fields' contribution to its address. A separate case distinguishes
absent and present proposal operations hashes; it is representation evidence,
not a claim that ordinary Propose/Validate already handles noncanonical values.

The initial target refused compilation on the old bare enum constructors.
The first corrected test run passed. Strict Clippy then required as_chunks
instead of constant-sized chunks_exact; the iterator correction preserves the
same byte vector. Final test, strict target Clippy and package format exit zero.
Exact commands and all earlier failures remain in private graph-event-migration
evidence. The final runner summary is quoted below.

Deleting record_hash from the real current encoder makes the new vector case
fail. The exact original source was restored and compared, then the target
passed again. This mutation changes no retained historical vector. It is a
bounded current-format regression witness, not complete provider durability or
the final repository gate.

> test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
