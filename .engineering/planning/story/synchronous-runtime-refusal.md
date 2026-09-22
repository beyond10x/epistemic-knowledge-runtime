---
format: aep.planning-md/1
id: story:synchronous-runtime-refusal
kind: story
status: active
title: Return a named refusal for synchronous store use inside Tokio
relations:
- serves: vision:o2
- decomposes: epic:p1-kernel-ontology-core
scope:
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: inferred
  path: crates/ekr-store/tests/runtime_context.rs
revision: 6
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
