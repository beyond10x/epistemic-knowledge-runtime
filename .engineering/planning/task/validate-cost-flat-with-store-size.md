---
format: aep.planning-md/3
id: task:validate-cost-flat-with-store-size
kind: task
status: active
title: Validate and commit cost stays flat in a large delta
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
- derived_from: story:commit-cost-flat-with-store-size
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:08Z", actor: "human:timo", revision: 3}
---
## What is wrong

Wave extract-06 unit C made a held evidence object re-read only when its own stream moves. At full
size (20,000-fact base; 10,000- and 80,000-fact deltas; release; load 30–43;
`crates/ekr-sdk/tests/commit_scaling.rs`) SQLite still grows within the large delta: averaged over
transactions 1–15 against 106–120, validate 277 → 891 ms, commit 1,444 → 2,321 ms, propose
1,082 → 1,315 ms. Ratios: large/small 1.61× and last/first 3.28× over 3 transactions at each end;
1.40× and 1.43× over 15-transaction medians. The file provider: 1.18× and 0.76× (1.45× and 1.38×).
The story's acceptance bound is 1.5×.

Hypothesis, not verified: `crates/ekr-kernel/src/validate/lifecycle.rs:20–27` copies every claim in
the graph when a transaction holds a retract or supersede (found while scoping
`story:evidence-attaches-to-a-held-assertion`), and the consumer's transactions supersede.

## Build

A frame-pointer profile of `validate` and `commit` in the large SQLite delta (all threads of the
`ekr` child, inclusive), written here with the load average it ran at, then the change it points to.

## Acceptance

Revised by the coordinator on 2026-10-01 (`review-result:next-waves-1001-acceptance-r1`): this
acceptance first pointed at `story:commit-cost-flat-with-store-size`'s whole-transaction bound. That
story closed on it in 0.0.25 while validate alone still grew 3.2×, so the validate half of its
original title is bounded here on its own. The statistic matches the story's: medians of 15
transactions, load printed, no idle machine required.

- In the large SQLite delta of `crates/ekr-sdk/tests/commit_scaling.rs` at its default sizes, the
  median validate time of the last 15 transactions is at most 1.2× that of the first 15 (measured
  for 0.0.25: 277 → 891 ms averaged, 3.2×), and the same for commit; the load average is printed
  beside the numbers.
- Before the change, the profile result written into this task names one counted quantity per
  transaction (for example claims copied, or objects read); a counting test in the default gate
  asserts that quantity is equal in two stores whose assertion counts differ at least fourfold
- Refusals byte-identical (the kernel differentials and conformance suites pass).

## Recovered profile and resume boundary (2026-10-02)

The previous implementor's retained transcript reports frame-pointer samples for the
20,000-fact base and 80,000-fact large delta. Its small profiling lane was reduced to 2,700,
so this is diagnostic evidence, not the default-size acceptance measurement.
Per-transaction normalized samples from transactions 18–28 to 111–120 were reported as:
evidence loading 272.5 to 539.2 (validate 50.0 to 110.2; commit 100.3 to 150.4),
lifecycle checking 0 to 0. This contradicts the initial lifecycle-copy hypothesis for that run.
Other reported growth remained in checkpoint writes, root calculation and transaction-map cloning.
Source: the retained unit-W transcript and profile text, recovered by the read-only resume audit;
raw perf stacks had already been deleted by the prior session. The profile's load is retained in
its harness log and must be carried into the final measurement report, not guessed here.

WIP 39b37674 isolates repeated decoding of retained evidence on cached command replays.
Its counted quantity is retained objects placed into command histories, including memo hits; this is not a count of native provider reads. Its retained mutation result was
490 versus 196 reads with evidence skipping disabled, followed by an eight-case green
add_evidence lane with skipping restored. An earlier successive-transaction case still grew;
there is no claim that the full 1.2x acceptance passed. Complete the default-size measurement
and differential refusal tests before closing this task.

Reconcile with released main first: replay_history must use the current entered() guard,
and skipping evidence must not hide an appended withdrawal event or a replaced SQLite store.

## Default-size measurement and next correction (2026-10-02)

The resumed runtime at a8f087094 was measured with the default fact sizes on SQLite in release mode. The ignored benchmark exited 101. Verbatim per-verb medians and ratios from scaling.log:

```text
Sqlite first 15 medians            propose    645.5 ms  validate     76.0 ms  commit    648.5 ms  total   1370.0 ms
Sqlite last 15 medians             propose    923.1 ms  validate    182.7 ms  commit   1208.1 ms  total   2313.9 ms
Sqlite median ratios: validate 2.404x; commit 1.863x; load 14.50 13.53 14.69 8/6629 90184
```

These miss this task's bound. The counted retained-object test still passes, and two independent replay attacks pass; neither substitutes for measured acceptance. See review-result:adversary-extract-07b-w-pass-1. The task remains active.

The new frame-pointer capture is retained with its harness log. The implementor's profile attributes substantial validation work to copying retained transaction records through ReplayState::transactions_mut. A new counted test was written and failed before the correction:

```text
running 1 test
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... FAILED

failures:

---- replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers stdout ----

thread 'replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers' (143448) panicked at crates/ekr-kernel/src/replay.rs:1492:13:
assertion `left == right` failed: file=false: prior document buffers copied
  left: [4, 16]
 right: [0, 0]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.42s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

The next correction shares individual internal records while preserving the public full-transaction snapshot API and its unchanged-read sharing. Session settle must read only the states it needs, so constructing that public snapshot does not simply move the copying into another part of validate/commit. This expands the implementation surface to replay state, runtime state access and CLI session settle; O also edits session responses, in a separate region. No wire format or canonical root encoding changes. Re-measure before claiming completion.

## Second full-size measurement (2026-10-02)

The second full-size SQLite run covers f9d910a3b, using unchanged sizes 20000/10000/80000 and
unchanged first/last full-transaction windows. The run remains red. Exact measured lines:

```text
Sqlite first 15 medians            propose    676.6 ms  validate     57.9 ms  commit    673.6 ms  total   1408.2 ms
Sqlite last 15 medians             propose    753.5 ms  validate    115.1 ms  commit    994.9 ms  total   1863.5 ms
Sqlite median ratios: validate 1.986x; commit 1.477x; load 11.71 18.76 17.85 4/6321 874580
MEASUREMENT_EXIT=101
```

Raw scaling.log, scaling.perf.data and profile analysis remain under <cache>/ekr-extract-07b/w/records.
The implementor reports that record-copy samples vanished; validation still scales with candidate
and alias scans and old-graph destruction. Commit samples grow in knowledge/evidence root hashing,
checkpoint work and graph copying. These are profile-derived hypotheses for further counted probes,
not a green acceptance claim. Current SHA-256 canonical roots must remain byte-identical.
The coordinator continues correction within the user's full extract-07b scope; no bound is relaxed
and no release is claimed. Temporary Rust throughput probes may measure alternative hash execution
before any production dependency choice; no dependency change has been accepted on that basis yet.

## Review trend after two attacks (2026-10-02)

Both adversary passes found no new behavioral failure and both carry the same unresolved
performance acceptance. The second message records newer measured ratios, so its signature
is different: the CLI's new/resolved lists below do not mean the acceptance has been met.
The earlier and latest default-size runs both exceed the unchanged bound. W remains active.
The reviewer has finished two attacks; subsequent corrections are checked against retained cases
and reviewed by the coordinator without opening a third attack. Completion still requires a green
full-size measurement and the combined gate.

The earlier comparison accidentally selected the older acceptance reviews through the CLI's
default review selection. This corrected comparison names both extract-07b adversary records
explicitly; it changes no finding or outcome:

```json
{
  "artifact": "task:validate-cost-flat-with-store-size",
  "reviews": 4,
  "from": "review-result:adversary-extract-07b-w-pass-1",
  "from_reviewer": "unattributed",
  "to": "review-result:adversary-extract-07b-w-pass-2",
  "to_reviewer": "unattributed",
  "carried": [],
  "new": [
    {
      "file": "crates/ekr-sdk/tests/commit_scaling.rs",
      "line": 484,
      "category": "acceptance",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "undecided",
      "message": "Current f9 SQLite measurement still misses W's independent 1.2x bounds; validate medians are 57.9 to 115.1 ms (1.986x) and commit medians are 673.6 to 994.9 ms (1.477x). This carries the existing acceptance blocker forward; no base-commit benchmark established regression origin."
    }
  ],
  "resolved": [
    {
      "file": "crates/ekr-sdk/tests/commit_scaling.rs",
      "line": null,
      "category": "acceptance",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "undecided",
      "message": "Default-size SQLite validate and commit median ratios are 2.404x and 1.863x, exceeding the required 1.2x bound for each verb."
    }
  ]
}

```

## Indexed validation and hash correction

The coordinator accepted the measured SHA-256 backend change after reading the temporary Rust
throughput probe, independent digest comparison and compatibility tests. Core uses ring internally;
sha2 remains a dev oracle. Domain labels, canonical bytes, digest width and streamed buffering
are unchanged. This supersedes the earlier pending dependency decision, without changing any
acceptance threshold or authorizing a dependency release.

The correction also borrows existing node types and keeps a private alias lookup bound to exact
revision identity and roots. A historical or divergent read rebuilds it. The coordinator reviewed
the minimum-holder semantics, revision checks, peer/historical differential and public snapshot
isolation. The counted tests retain their initial red results and now avoid copies or visits to
unchanged nodes. Store authority test scaffolding moved to a test fixture after the ownership
inventory refused its source location; the inventory remains intact.

Source and retained adversary cases are integrated. The merge preserves the attachment module,
adds AttachEvidence to the alias cache's exhaustive no-change arm, and initializes attachments
in the new candidate fixture. All previously generated scenarios and their floors are retained.
Pinned specification and synthesis freshness pass, and the entire workspace checks with every
target. Full combined correctness and default-size timing remain pending; this task stays active.

Evidence: unit report.md, core-sha-backend.log, store-kernel-indexed.log,
story-contract-indexed-final.log, clippy-indexed.log and hash-probe-interleaved.log under the
retained W scratch; coordinator w-merge-check.log and w-merge-spec.log.

## Third default-size measurement

The indexed validation and accelerated SHA-256 correction was measured at the unchanged default
SQLite sizes and windows. The older aggregate harness passed, but this task remains red: commit
meets its independent bound and validation does not. Exact measured output:

```text
Sqlite first 15 medians            propose   1260.5 ms  validate     96.6 ms  commit   1294.9 ms  total   2652.0 ms
Sqlite last 15 medians             propose   1261.4 ms  validate    167.1 ms  commit   1461.3 ms  total   2889.8 ms
Sqlite median ratios: validate 1.730x; commit 1.128x; load 35.31 41.29 45.89 6/6859 2416714
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 696.52s
MEASUREMENT_EXIT=0
```

The complete log and raw perf capture remain under <cache>/ekr-extract-07b/w/indexed.
Severe wall-clock outliers occurred under host contention and are retained in the log; no samples
or windows were removed or changed. The next correction follows the new first/last-window profile.
This is the existing acceptance finding after the completed adversary budget, not a third attack.
The combined correctness gate is also still running. No implemented status or release claim follows
from the aggregate harness exit code.

## Confirmed publication and unchanged edge index

The coordinator reviewed the correction in 033f519ae and its combined integration in 58f53523b.
After a confirmed publication, the replay cache retires only the predecessor whose covered
position and prefix digest match that exact occurrence. Held public readers keep their immutable
state. An unchanged assertion-edge index is shared across revisions; an edge assertion invalidates
it, including when validation alternates between historical and current bases. AttachEvidence is
explicitly classified as preserving assertion subjects.

The counted cases `confirmed_commit_releases_the_previous_head_before_the_next_command` and
`node_only_commits_share_the_unchanged_assertion_edge_index` retain their pre-correction failures.
`retirement_requires_the_confirmed_occurrence_and_matching_prefix` checks mismatched confirmation
and unrelated cached prefixes. The retained adversary cases, old-basis behavior and edge-index
invalidation checks remain in the default suite. No third adversary attack was opened.

The default-size SQLite benchmark now directly enforces this task's independent validate and
commit limits as well as the existing aggregate limits. Its sizes, transaction windows and bounds
are unchanged. A passing aggregate statistic alone can no longer be mistaken for this acceptance.
The combined correctness gate and full-size measurement remain required before implementation
status; unit test results do not discharge either obligation.

## Fourth default-size measurement

The combined source was measured with every consumer and attachment change present. Both
independent bounds remain unmet; the unchanged aggregate checks also fail. Exact raw output:

```text
Sqlite first 15 medians            propose    571.9 ms  validate     50.0 ms  commit    589.0 ms  total   1211.0 ms
Sqlite last 15 medians             propose   1020.0 ms  validate    107.9 ms  commit   1245.7 ms  total   2373.6 ms
Sqlite median ratios: validate 2.158x; commit 2.115x; load 18.92 17.12 19.36 2/6807 3299835
MEASUREMENT_EXIT=101
```

Source and raw evidence: combined commit 58f53523b, retained
<cache>/ekr-extract-07b/w/combined4/scaling.log and scaling.perf.data.
The new explicit per-verb assertions correctly fail this run. No samples were removed and no
threshold was changed. Profile analysis continues before selecting the next counted correction;
the task remains active and this result does not authorize a release.

## Bounded graph-reuse experiment

The coordinator reviewed the next prefix/checkpoint correction and requested position-only
mutation coverage beside event-content tampering. The implementor reports a green counted
prefix-hash case and obsolete-checkpoint lifetime case. These target validation; they do not
establish that commit timing meets acceptance.

The latest full-size profile still attributes material commit growth to graph cloning and root
hashing. A temporary Rust allocation probe reported no allocation reduction from BTreeMap
clone_from, so that approach is not accepted as a correction. Public CanonicalGraph owns its maps
and records; changing those public types or the canonical hash format is outside this correction.

The coordinator authorized a bounded private graph-reuse experiment in the kernel application
and replay-cache surface. A counted test must first expose copies of unchanged graph records.
Only an exclusively owned retired graph may be taken; an external reader or retained checkpoint
must prevent extraction. Reuse requires exact root/revision lineage and verified intervening
append-only operations, with the existing clone path for every unsupported operation, missing
buffer or mismatched history. Historical reads, retry/conflict, restart and failed publication
must preserve current behavior and immutable snapshots. No acceptance limit or release boundary
changes. The prefix/checkpoint correction is committed separately before this experiment.

Sources: retained combined4 profile and the implementor's allocation-probe report in W scratch;
coordinator source review and dispatch. Full combined correctness and the unchanged default-size
measurement remain required before the task can be implemented.

## Verified prefix and checkpoint correction integrated

The verified-prefix and superseded-checkpoint correction is integrated from 723cf92cc. The memo
compares every canonical event and stream position before reusing a digest; it does not replace
retained-object checks or admit replay state. Tests compare its result with the original hash
function after content and position changes at either end and in the middle, shortened and empty
histories, native-provider identity changes and later mutation while an old digest vector is held.
The checkpoint correction preserves explicit historical keep guards and public held snapshots.

The coordinator reviewed both changes and the position-only case added during that review.
The implementor reports a green full kernel/store suite and clippy; raw results are retained as
store-kernel-prefix.log and clippy-prefix.log in W scratch, beside the initial counted failures.
The ignored benchmark additionally records each actual proposal input length, after timing the
request. Its timing windows, thresholds and generated operations are unchanged.

Separately, the integration's SDK/store/views/tooling tests and format, all-target clippy,
documentation, vendor, pinned specification, freshness and planning checks passed before this
merge. Logs: coordinator remaining-consumer-store-check.log and combined-nontest-check.log.
This is accumulated correction evidence, not a completed final combined gate. Graph reuse and
unchanged default-size timing acceptance are still open.

## Confirmed graph reuse integrated

The bounded reuse correction was reviewed by the coordinator and passed the implementor's full
kernel/store regression, focused all-target clippy and formatting checks. Source: 0704be44b;
raw results: W store-kernel-recycle.log, clippy-recycle.log and recycle-unit-final.log.

The initial copied-assertion regression is retained in red-recycle.log. The green case measures
actual assertion totals at the same checkpoint phase, asserts their required size ratio, and
requires no unchanged assertions to be copied. Its independent oracle clears both reuse layers,
asserts that cloning actually occurred, and compares the complete graph and freshly hashed Root.
The existing candidate-application counter remains exact.

Coordinator review found that the first thread-local buffer owner could retain graph contents
after the runtime closed. Ownership now belongs to the replay cache; the thread retains only a
weak locator. reusable_graph_belongs_to_the_authority_not_the_thread proves the owner-drop
boundary while the thread is still alive. Additional cases retain external readers and checkpoint
graphs, preserve inherited attachments, reject an unconfirmed candidate as reuse provenance, and
fall back for an equal root reconstructed in another allocation. Confirmed capture requires
exclusive ownership, the sealed bridge and the exact parent/transaction proof; reuse additionally
binds target allocation, complete root, revision identity and commit time.

Design section 104 records these boundaries and the additional private buffer per authority.
The default-size timing acceptance and final combined repository gate remain pending. This
correction and its counted improvement do not close the task by themselves.

## Fifth full-size measurement (2026-10-02)

The frozen combined source at 4a3e384b0f12a56edbaea40858dde3aff4fba512 includes the
verified prefix, checkpoint retirement and exclusively owned graph reuse corrections.
The default SQLite release measurement remains red. The following lines are copied directly
from the retained `w/combined5/scaling.log`:

```text
Sqlite first 15 medians            propose    590.1 ms  validate     48.5 ms  commit    621.8 ms  total   1260.4 ms
Sqlite last 15 medians             propose    741.9 ms  validate     60.4 ms  commit    861.9 ms  total   1664.1 ms
Sqlite median ratios: validate 1.244x; commit 1.386x; load 17.74 17.26 16.89 7/6251 354886
Sqlite: large/small 1.74x, large last/first 1.96x
MEASUREMENT_EXIT=101
```

The raw harness log, frame-pointer recording and decoded stacks remain under
`<cache>/ekr-extract-07b/w/combined5/`. This measurement does not satisfy either
independent timing bound or the existing aggregate benchmark assertions. The task remains
active. No sample size, transaction window or acceptance bound was changed. Further work
must follow the profile and preserve the current canonical roots and refusal behavior.

## Combined correctness gate after graph reuse (2026-10-02)

The coordinator ran `task check` on the exact frozen measurement revision above, with the
repository-pinned tools and the ext4 temporary directory documented in the wave. The complete
log remains at `<cache>/ekr-extract-07b/coordinator/full-reuse-combined-check.log`.
Its directly captured terminal result is:

```text
valid
CHECK_EXIT=0
Fri Oct  2 15:25:55 UTC 2026
```

This includes workspace tests, benchmark-feature compilation, documentation, vendor compatibility,
specification validation, all generated-suite freshness comparisons and planning validation.
The planning validator retains its historical prose-only review warnings. This is correctness
evidence for the measured revision; it does not turn the failed timing measurement green.
The next bounded correction targets hash text formatting and borrowed checkpoint output,
preserving the current decoder, complete serialized bytes and canonical hashes. The native
encoding-inlining probe showed no reliable improvement and is not being applied. Final acceptance
still requires a green default-size measurement and a combined gate on the final source.

## Formatting and borrowed checkpoint correction integrated (2026-10-02)

Source correction 64fd489199956dad0b7dd0a626565c13624d80f8 and the combined wave evidence
are integrated at 79789de3b1a3ee654b5262d08c8904e88e275524. The coordinator reviewed the
source and retained tests. This is a bounded formatting/serialization correction, not a new
hash scheme or checkpoint cadence.

`ContentHash` formats a complete stack hex buffer with one writer call. The retained test
`formatting_a_hash_emits_one_complete_hex_string` failed on the prior implementation, and
`hash_display_preserves_text_flags_and_propagates_writer_errors` keeps the existing spelling,
formatter behavior and refusal behavior. The store's additive `GraphDocument::serialize_graph`
borrows the graph's record maps; canonical values retain their existing per-value conversion.
It reuses the existing graph format and explicitly handles every CanonicalGraph field.

The kernel's private checkpoint carrier borrows only while writing. Its default owned decoder
and admission remain unchanged. `borrowed_graph_matches_owned_bytes_for_every_value_and_retained_record_field`
compares complete owned and borrowed JSON/YAML, including attachments, lifecycle and temporal
fields; `borrowed_graph_returns_the_serializer_write_error` preserves failures. The coordinator
identified that serde struct names are observable even when JSON agrees. The new
`borrowed_graph_preserves_serde_envelope_and_graph_struct_names` reproduced that mismatch before
explicit renames corrected it. The real writer's
`checkpoint_writer_matches_the_complete_owned_document_bytes` compares complete checkpoint
bytes across the seed and successive commits, including identity retention.

The implementor's report section 13 and retained logs under `<cache>/ekr-extract-07b/w/`
record the native allocation probe, rejected inline-hint probe, formatting red/green,
serialization byte/name/error tests, focused checkpoint/replay/evidence tests, public-surface
and story guards, formatting and touched-crate clippy. Those checks passed. The scratch allocation
probe shows fewer record-copy allocations while preserving complete JSON bytes; it does not
claim that all allocation disappears or that timing acceptance is green. The next measurement
must use the exact combined revision and the unchanged default SQLite workload and bounds.

## Sixth measurement and storage diagnostic — acceptance remains red

Frozen source: `570cb34cf157e0703d3a48c7ce6d102933cb380d`. Default SQLite workload:
20,000 / 10,000 / 80,000 facts, unchanged first/last 15 windows, 1.2 per-verb bounds,
and existing 1.5 aggregate bounds. `combined6/scaling.log` reports validation 41.5→46.1 ms
(1.111×, pass), commit 520.2→650.9 ms (1.251×, fail), aggregate ratios 2.12× / 2.07×,
exit 101, and 274.85 seconds. Checkpoint transaction 119 took 5,070.8 ms to commit;
transactions 117/118 took 587.3/621.5 ms. Their mean is 2,093.2 ms, with checkpoint 119
accounting for 80.75% of those commit milliseconds. All measurements remain retained.

`combined6/functions-phases.txt` and `workers.txt` separate main-thread and asynchronous
provider work. Ordinary main+worker CPU sample counts grew 359→382 (1.064×); checkpoint
counts grew 69+94→136+221. Worker command assignment follows chronological main-command
samples, so it is an inference. Checkpoint 119 has 137 main+worker samples spanning 5.059
seconds, about 1.38 seconds of nominal sampling at 99 Hz. Cycles-only data cannot separate
storage waiting from descheduling. It does not justify another speculative CPU correction.

The original W transcript explicitly printed `TMPDIR=<cache>/claude-tmp` before profiling;
its script did not override that value. Both that path and all six resumed fixture paths are
on ext4. Selective original output is retained in `combined6/recovered-w-environment.txt`.
Current `/tmp` being tmpfs does not establish that the original run used tmpfs. A tmpfs run
would not replace ext4 acceptance.

An unchanged-source diagnostic tested three ranked explanations: full-checkpoint writes/syncs;
worker waiting/scheduling; and remaining encoding/copy CPU. It traced pwrite64, fsync,
fdatasync and futex without payload bytes. The executed CLI build ID,
`11689baad643c66b0fa0c5cf2c833a9a11d101d8`, matches the combined6 perf build-ID record.
`diagnostic-io/executed-artifact.txt` retains that identity and unlimited workload file-size limits.

Two incomplete attempts are preserved and excluded from acceptance. An inherited 64 MiB
file-size limit stopped the linker with SIGXFSZ before fixtures: invalid diagnostic, exit 101.
An external trace-size monitor then stopped raw tracing at 69,267,834 bytes before base
completion: exit 143. The final attempt streamed per-thread count/sum/max durations and
only calls lasting at least 20 ms, with a 600-second timeout and an external 64 MiB output
monitor. It completed in 380.26 seconds, exit 101, retaining 2,923,594 bytes of summaries
and slow calls. No bound or source changed.

`diagnostic-io/syscall-summary.log` records the following aggregate for trace IDs 1507000 and above, used as an approximation
of activity from the large-phase start onward:

- 1,816 fsync calls: 45.932106 seconds summed duration; maximum 1.138453 seconds.
- 2,775,803 pwrite64 calls: 13.397234 seconds; maximum 0.114338 seconds.
- No fdatasync calls observed.

The run did not retain clone events or a complete TID-to-process membership table. These are
phase aggregates, not proved large-child worker totals. In particular, membership of threads
1507494, 1520179 and 1520180 is not independently established. Thread 1507005 alone accounts
for 910 fsync calls and 22.318939 seconds; it is reported separately without relying on the
additional thread attribution.

The large main thread, independently identified as PID 1507000, recorded 367,607 futex waits,
totaling 154.338468 seconds, with a maximum of 3.608053 seconds. Large-phase elapsed time
was 289.9 seconds. Its final five-second process sample had 159.39 seconds cumulative CPU
(user 132.41 / system 26.98); this is a lower bound before exit, not a complete per-command
measurement. Main waits overlap worker CPU and storage calls. Worker idle waits also overlap;
neither can be added to CPU/storage totals as disjoint elapsed costs.

The final pre-close main wait spans approximately Unix time 1790957303.447759–1790957306.219604
(2.771845 seconds). Two slow fsync returns in that span total 0.353918 seconds. Their linkage
to the last full checkpoint is plausible, but no command markers were traced, so exact
checkpoint-119 attribution is not proved. The later close checkpoint has separate waits and
must not be charged to transaction 119. Ordinary commit medians over the 12 non-checkpoint
samples were 785.5→948.5 ms (about 1.208×); the all-15 medians were 785.5→965.1 ms (1.229×).
These traced timings are diagnostic only.

Storage waits are directly observed and materially contribute; their durations do not alone
explain the whole elapsed gap. Tracing adds substantial overhead: base 52.3 seconds versus
untraced 36.8 seconds. The coordinator's full gate concurrently exercised the same ext4
filesystem. No new EKR-owned repeated CPU work has been established. One same-revision
ext4 measurement after that known concurrent gate finishes is a justified controlled probe:
remove syscall tracing, retain the acceptance run's 99 Hz perf sampling, and change only the
known competing gate workload. Retain all earlier results and every bound, cadence,
durability setting and dependency pin. Do not repeat measurements merely until one passes.

## Combined source correctness gate

The complete `task check` passed on frozen source
`570cb34cf157e0703d3a48c7ce6d102933cb380d`. The retained coordinator log
`<cache>/ekr-extract-07b/coordinator/full-serialization-combined-check.log` ends with:

```text
CHECK_EXIT=0
Fri Oct  2 16:19:40 UTC 2026
```

This covers formatting, workspace clippy and tests, benchmark-feature compilation, rustdoc,
vendored YAML compatibility, pinned specification validation, generated-suite freshness and
planning validation. Historical prose-only planning review warnings remain. The previously
recorded ext4 temporary directory is used without changing the inode-reuse test. All feature
acceptance and retained review corrections are exercised on the combined source. Implementation
status does not claim publication: wave release remains held on the separate performance task.

## Controlled measurement after the combined gate

The coordinator's complete correctness gate ended before this one further measurement started.
The implementor retained the same source, CLI build identity, ext4 fixture filesystem, default
SQLite sizes, first/last full-transaction windows, bounds and perf sampling. No syscall tracer
ran. `w/controlled6/scaling.log` records the result:

```text
Sqlite first 15 medians            propose    518.7 ms  validate     28.8 ms  commit    468.1 ms  total   1015.7 ms
Sqlite last 15 medians             propose    545.8 ms  validate     28.2 ms  commit    618.5 ms  total   1192.5 ms
Sqlite median ratios: validate 0.979x; commit 1.321x; load 4.72 7.31 8.33 2/6388 1698381
Sqlite: large/small 1.65x, large last/first 1.57x
MEASUREMENT_EXIT=101
Fri Oct  2 16:24:50 UTC 2026
```

Validation passes its bound, while commit and the existing aggregate assertions still fail.
The final checkpoint is faster than in the earlier concurrent run, but removing the coordinator's
own heavy workload did not meet acceptance. This rejects contention as a sufficient explanation
for the remaining failure. All earlier measurements remain retained; no further repeated sampling
is planned. The task remains active and the release remains held. No acceptance threshold,
durability setting, checkpoint cadence, retained format or native dependency pin is relaxed.
