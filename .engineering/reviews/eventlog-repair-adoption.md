# Qualified Eventlog repair adoption

Selected immutable source: 28e578568846fc860e44a5f7c76b7e807abddc12

Eventlog PR #15 merged after required source checks passed:

https://github.com/beyond10x/eventlog/pull/15

https://github.com/beyond10x/eventlog/actions/runs/35720472752

The required CI completed PostgreSQL/SQLite conformance, formatting, lint, comparative and restart proof. The local Eventlog full-gate attempt lacked its PostgreSQL fixture; its refusal is retained, not counted as a pass.

EKR task check against the selected repair exited zero. Actual runner totals:

- Workspace: 531 passed, 0 failed, 0 ignored, 83 summaries.
- Standalone vendor: 182 passed, 0 failed, 0 ignored, 7 summaries.

A separate old-dependency process wrote synthetic evidence-backed seeds on both
backends. A fresh process using this exact repair reopened them through the real
kernel authority and compared head, graph document, replay and retained evidence
bytes against the original output. Both process commands exited zero. The prior
release qualification and original defective-release probe remain unchanged;
the new proof has its own source/lockfiles, states, logs and statuses.

The source-only SQLite repair validates the complete existing blob tuple before
fresh atomic reuse. Its independent review found no new discrepancy. Original
receipt-first retries remain callback-free after corruption or erasure.
The publication mechanism stays the portable AtomicBlobEventStore trait.

This selects a published, reviewed repair commit after the 0.3.0 tag, not a new
tagged release. Entity Runtime remains outside the product dependency graph.
No operator store, legacy codec or live migration was changed. The future
blob-backed writer still owes its own application, restart and corruption
acceptance; passing seed compatibility does not discharge those requirements.

Owners: the coordinator owns dependency selection, complete consumer gate and
cross-version compatibility proof. The upstream repair and independent test
findings are retained in Eventlog's corresponding review and task records.
