# Kernel read projections and CLI integration

This is a dependent source slice within the operator-approved completion work,
for story:seed-and-explain and story:ekr-cli. It starts only after a coherent
committed writer checkpoint exposes the real Runtime and VerifiedRead. Remaining
writer crash/uncertainty controls continue independently; this slice cannot close
those stories, publish partial durability, or substitute an implementation.

Read AGENTS.md, COORDINATOR.md, both story bodies, DESIGN 88–94, active kernel/graph
ESS, and p1-cli-explain-contract-r2.md. That R2 contract resolves the prior scope
and contract reviews. Its declarations are already synchronized across active
trees. The adjacent old patch is historical and must not be applied.

## Ownership and execution

Reuse managed ekr-cli-host-input-20260922 after the coordinator integrates the
actual writer checkpoint and fast-forwards this tree. Keep its exclusive target,
private TMPDIR, two build jobs, no incremental/debug output and direct rustc.
Before builds require persistent free 4 GiB, temporary-memory free 8 GiB and
MemAvailable 8 GiB. Preserve evidence under the assigned private read-cli directory.
Acquire/heartbeat/release only your own implementor lease; never remove the tree.

One source owner implements the kernel reads first, then CLI consumers. Kernel
ownership is only new src/explain.rs and tests/explain.rs. In src/lib.rs it may
insert `mod explain;` and the corresponding `pub use explain::{...};` block;
no existing imports, exports or declarations are owned. The writer owns every
other kernel source and existing test. Return exact diff hunk headers for this
shared file; coordinator performs merge-tree review before integration.

CLI ownership: src/main.rs, new src/exit.rs and src/cli/ modules, the public module
exports in src/lib.rs, new tests/retraction_example.rs and dedicated new fixtures
beneath tests/fixtures/. Existing graph/assertion/serialization/contract tests
remain with the writer. Reuse reviewed host.rs and its tests unchanged unless a
concrete defect is reported first. Do not edit planning, DESIGN, ESS, manifests,
lockfile, vendor, frozen parser profile or any store implementation.

The coordinator alone writes planning and shared contracts. Request an exact
missing accessor/type if the real facade is insufficient; no raw-store dependency,
allow-all authority, fabricated receipt, model explanation or alternate writer.
No new dependency or public command beyond the declared verbs is authorized.

## Results and retained evidence

Implement the active SnapshotResult and ExplanationResult carriers in kernel
source, using the existing strict graph projection carriers or concrete typed
projections, not lossy serde_json::Value or invented persisted records. Methods
consume one VerifiedRead; do not reopen storage while assembling a result.
Use the existing shared graph valid_at query and preserve the full graph/root.

Follow the R2 ordering and evidence union exactly. A replacement accepted earlier
has its actual origin found in retained history. Preserve actual receipt addresses,
profile and evidence payload verification; missing support refuses the entire
result. HumanStatement terminates directly. An unknown ID is AssertionNotFound,
while missing/corrupt required retained support is an operational verification
failure. No successful partial explanation.

Thin CLI handlers use Runtime::file/sqlite with configured trusted host, real
lazy system time and actual public command results. Propose passes an opened
reader directly to propose_reader; no unlimited pre-read or reserialization.
Seed keeps its declared document parser. Every invocation can be a fresh process.
Named refusals exit 2, operational/configuration failures exit 1, all declared
outcomes including Rejected/Stale exit 0. Only the typed valid-but-different anchor
maps to Seed AlreadySeeded; corruption must not be hidden by that mapping.

## Acceptance and handback

Write decisive cases before their source and retain the first red. New tests
must exercise the real providers/kernel handlers, not shape-only substitutes.
Cover seeded and ordinary origin, withdrawal, distinct replacement evidence,
already-accepted replacement, required-support corruption and unknown assertion.
Hold one captured read while another valid commit advances the provider, then
explain that capture without leaking the later lifecycle transition.

Drive the DESIGN example through actual fresh binary processes on both providers:
seed, propose Alice, validate, commit, propose Bob plus explicit supersession,
validate, commit, selected valid times and earlier revision, then explain. Cover
original Seed/Commit retries after restart/head movement, named missing targets,
recorded rejection/staleness and malformed input. Keep actual byte/receipt/ID
checks; an unchanged seed-only root is insufficient. Exact seed/commit clock
silence is tested through the injected shared host seam, never a public fake clock.

Run focused new targets, strict scoped Clippy and formatting. Preserve the first
failure and distinguish newly observed defects from obsolete unrelated fixtures.
Report source commit, commands/status/counts from logs, any unexecuted acceptance,
exact changed paths/shared hunk headers and remaining ownership. Do not publish,
close a story, modify old assertions or claim the full gate. Independent adversary
and the complete integrated task check follow the finished source.
