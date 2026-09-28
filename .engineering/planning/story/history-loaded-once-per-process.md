---
format: aep.planning-md/3
id: story:history-loaded-once-per-process
kind: story
status: active
title: The eventlog store verifies a retained blob once per process
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
scope:
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/tests/history_cache.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T09:14:09Z", actor: "agent:claude-coordinator", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-28T09:14:09Z", actor: "agent:claude-coordinator", revision: 5}
---
## Context

`load_history` (`crates/ekr-store/src/eventlog.rs`) reads every required blob and hashes it on each
read: 49% of one `ekr resolve` on the store of `epic:ingestion-throughput`. Within one process
(`story:ekr-session`) the same blobs would be read and hashed again on every call.

## Build

The eventlog store keeps, for the life of the handle, the retained objects it has verified and the
occurrences it has read. A later read loads only occurrences past the last one it holds and only
objects it has not verified. Blobs are content-addressed and immutable, so a blob verified once
stays verified for the process. Nothing is cached across processes.

## Acceptance

- A second read on one handle reads and hashes no blob the first read verified (a counting test).
- An occurrence appended through another handle is seen by the next read.
- A blob changed on disk before its first load is still refused (`required-object-integrity` or
  the provider's existing refusal, unchanged).
- The kernel conformance suite passes on the file and SQLite providers.
