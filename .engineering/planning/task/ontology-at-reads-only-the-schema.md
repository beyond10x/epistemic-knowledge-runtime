---
format: aep.planning-md/3
id: task:ontology-at-reads-only-the-schema
kind: task
status: draft
title: ekr ontology --at reads only the schema history
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
revision: 3
---
## What is wrong

A consumer reported on 2026-10-01 that `ekr ontology --at 405` on a SQLite store of about 148,000
assertions takes 486 s with ekr 0.0.25, while `ekr ontology` at the head and `ekr head` answer
quickly. A caller that needs only a past revision's `schema_version` and `schema_version_number`
pays a full replay of the graph up to that revision. Not blocking the consumer now (it keeps its
own copy record), but older runs and its check still pay it.

Related: the perf audit's "an overview at an old revision loads the full history"
(`task:perf-audit-2026-09-29-remaining`, Views bullet; `crates/ekr-kernel/src/read.rs`
`schema_history`) and `task:rebuilt-revision-reuses-retained-verdicts` (rebuilding a released
revision replays from the seed).

## Build

A probe first: where the 486 s goes (replaying data operations, revalidation, or loading history).
Then a schema-only read: the schema at a revision is a function of the seed and the schema
transactions up to it, so `ontology --at` replays only those, or reads a revision → schema-version
index kept with the history. The answer stays byte-identical.

## Acceptance

- `ekr ontology --at <revision>` answers in time that does not grow with the number of data
  transactions before that revision: a counting test shows no data operation is replayed for it.
- Its output is byte-identical to today's for every revision of the views and kernel fixtures, on
  both providers.

## Refined scope and acceptance

Cited baseline: crates/ekr/src/cli/ontology.rs routes a selected revision through runtime.read;
crates/ekr-kernel/src/read.rs requires the graph; replay/checkpoint state already retains
revision-to-ontology mappings. Reuse those verified mappings instead of introducing another
persisted read model. Inferred surfaces: kernel read/replay/checkpoint interfaces, CLI ontology,
their existing tests and the current kernel ESS contract if its read projection needs clarification.
No new persistent format or dependency release is authorized.

First measure current cold open, warm held-handle read, checkpoint plus tail and forced full replay.
Separate opening integrity work from the selected ontology projection. The scoped claim is that
selecting historical ontology after the same verified open executes no data operation replay;
cold open retains the current full integrity obligations. Preserve byte-identical ontology output
at every fixture revision on both providers, absent-revision and corruption refusals, host
authority/profile checks, changed history detection and full-replay behavior. A counter must
distinguish schema work, data replay and open verification. If the current authority cannot safely
expose its verified mapping, report the measured blocker rather than trusting serialized receipts.
The previous broad elapsed-time claim is replaced by this deterministic scoped work-count claim;
the reported consumer timing remains historical motivation, not a new performance measurement.

## Admission-boundary refinement from source scoping

Runtime construction does not itself verify a store. The selected path deliberately reads only
through its requested revision, so later corruption can be outside its result. An optimization
must not replace that contract with unconditional head admission.

The scoper identified a candidate for the original one-shot CLI benefit: attempt the existing
complete history admission and materialize its verified checkpoint mapping, select from that
mapping only after successful authority admission, and fall back to the unchanged selected-only
path on any speculative failure. This is an unverified implementation hypothesis, not permission
to trust a checkpoint or mutate retained state. A root-only checkpoint fast path does not prove
the ontology mapping was admitted. Existing verified mappings can also benefit a warm handle.

Before selecting this candidate, compare same-handle fallback with fresh selected-only controls
for malformed native envelopes, malformed domain events and missing record objects after the
selected revision, corruption before it, invalid checkpoints, repeat failures and replacement.
A failed speculative head read must not poison occurrence/object/authority caches or change the
selected result/refusal. No retained write is allowed on this read path.

Measure cold checkpoint plus tail as well as warm reads. A warm-only improvement is insufficient
to claim the original one-shot CLI problem solved. If safe checkpoint reuse cannot improve the
cold selected read, record the unmet acceptance and hold the unit rather than closing it under
a convenient warm benchmark. No-checkpoint and forced full replay retain their integrity work;
report those separately. Exact source seams are history_at, history, reconstruct, ReplayState
revision ontology entries, and Runtime/Commit read methods; do not infer admission from a
revision number or the mutable prefix-hash memo alone.
