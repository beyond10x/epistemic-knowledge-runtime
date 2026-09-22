# P1-08 — kernel seed admission

Apply the installed aep-drive:implementor charter. Read AGENTS.md and COORDINATOR.md, then
story:kernel-validated-seed and its implementation-decisions section. The completion plan is
approved. Coordinator owns every planning and ESS mutation; report any required contract edit.

Managed tree: ekr-p1-08-seed; branch codex/ekr-p1-08-seed. Dedicated build directory:
<cache>/b10x-target/ekr-p1-08-seed. Scratch and TMPDIR: <cache>/ekr-completion-20260922/seed.
Acquire/renew your own lease and release it at handback. Two compiler jobs; inspect disk before
building and pause new compilation below 10 GB. Do not touch other sessions' caches or processes.

## Implementation boundary

Kernel owns initialization and replay admission. Remove the unchecked GraphDocument conversion
from store source and remove Commit::store. Expose read methods rather than a writer accessor.
Extend the existing guarded CommitAuthority port for seed replay; missing authority refuses.
Do not use an in-memory accepted-seed hash set. Keep the seed's full ontology and compare full
content on reopen, not only its schema ID.

Use a versioned seed input and persisted envelope with strict decoding. Caller assertions carry
Proposed only. Bootstrap context supplies actual operator/validator identity independently of
payload. Refuse self-validation and attribution mismatch. Preserve all graph fields while
narrowing; NodeDraft is not a lossless stored-node representation.

Bootstrap has no previous committed revision. Reuse deterministic validator helpers under an
explicit bootstrap mode; allow empty graph seeds without weakening ordinary empty-transaction
refusal. A private kernel capability gates initialization. The operator-approved story states
which bootstrap differences are legitimate and which checks remain mandatory.

Publish the seed object and Seeded event atomically through the existing pinned eventlog
AtomicEventStore port. NoStream guards the lineage; typed conflicts are not flattened before
classification. Content-addressed reuse verifies exact bytes. No unconditional-append fallback.
Verify payload address and metadata before replay admission. Legacy raw seeds remain untouched
and get an explicit migration-required refusal when required ontology/attribution is unavailable.

## Acceptance and old cases

The archived three invalid seed witnesses become executable kernel refusal cases through both
real providers. The store test demonstrating successful dangling conversion must be replaced by
that boundary regression. Existing low-level provider tests remain explicitly substitute-authority
coverage; do not add a kernel dependency to the store merely to preserve test placement.

Write behavioral red witnesses before repair. New API staging may be needed, but do not count a
missing-method compile error as behavioral red. Cases for valid empty/evidence seeds,
repeat/concurrent initialization, deterministic roots, replay after restart, unknown type,
dangling references, caller verdicts, root filing/schema/space, unsupported constraints,
initial lifecycle, wrong full ontology, missing authority and tampered payload all have named
expected outcomes. The no-write refusal is measured against both object and revision streams.

Use separate independently opened backend handles for concurrency and tampering tests; no
production writer accessor. Preserve the old semantic assertions when moving tests to the kernel.
Confirm all touched scope rows before implementation; request additional paths from coordinator
instead of silently editing outside the story.

Run tests for touched packages, fmt and package clippy. Coordinator runs the integrated gate and
adversary. Return exact commands/exits, executed counts, red/green evidence, scope confirmation,
changed paths and every outside path. Do not commit, publish, or write AEP.

## Retained evidence and decoder scope

The story's Retained bootstrap evidence and decoding section is binding: include exact
HumanStatement payload bytes, verify every evidence content hash before publish and on replay,
refuse unsupported source kinds explicitly, and test uncited invalid evidence too. This keeps
P1 bounded without treating a metadata hash as retained support. A read-only content lookup
must support later explain. Strict graph-record decoding is scoped; user Record keys remain data.
The graph files are also scoped for updating obsolete comments about the unvalidated seed path.
