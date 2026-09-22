unit: story:kernel-validated-seed; uncommitted ekr-p1-08-seed on 4032d002ecd2b08519910398ae6ed3b9ec8ea0f6
verdict: NEEDS-CHANGE
cases: executed 217→221, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: 16 paths; listed below
needs-coordinator: shared ontology correction scope; named loader refusal checks; AGENTS named-case citation

`git --no-pager diff --stat 4032d00` — inherited tracked implementation, test migration and coordinator ESS changes:
```
AGENTS.md                                          |   8 +-
 crates/ekr-graph/src/assertion.rs                  |   4 +
 crates/ekr-graph/src/canonical.rs                  |  20 +-
 crates/ekr-graph/src/edge.rs                       |   1 +
 crates/ekr-graph/src/evidence.rs                   |   2 +
 crates/ekr-graph/src/node.rs                       |   1 +
 crates/ekr-graph/src/root.rs                       |   1 +
 crates/ekr-kernel/src/commit.rs                    | 101 ++++++-
 crates/ekr-kernel/src/lib.rs                       |   2 +
 crates/ekr-kernel/src/validate/mod.rs              |  18 ++
 crates/ekr-kernel/tests/commit_path.rs             |  66 ++---
 crates/ekr-store/src/eventlog.rs                   | 175 +++++++++--
 crates/ekr-store/src/lib.rs                        |  20 +-
 crates/ekr-store/src/log.rs                        |  26 ++
 crates/ekr-store/src/snapshot.rs                   | 314 +-------------------
 .../ekr-store/tests/adversary2_event_vocabulary.rs |   7 +
 .../ekr-store/tests/adversary2_membrane_bounds.rs  |  41 ---
 .../tests/adversary_membrane_and_schema.rs         | 143 ---------
 .../tests/adversary_objects_and_append.rs          |   1 +
 .../tests/adversary_p1_06_reference_from_bytes.rs  | 120 ++------
 crates/ekr-store/tests/fixture/mod.rs              |  51 +++-
 crates/ekr-store/tests/fold_rules.rs               |  24 +-
 crates/ekr-store/tests/lineage/mod.rs              |   7 +
 crates/ekr-store/tests/membrane_boundary.rs        | 327 +--------------------
 .../tests/review_p1_invariant_one_at_the_store.rs  |  11 +-
 crates/ekr/tests/story_contract.rs                 |  22 ++
 systems/ekr/domains/kernel.yaml                    |   5 +-
 27 files changed, 495 insertions(+), 1023 deletions(-)
```

This diff was already present at dispatch. I authored no production, ESS, AEP or existing-test changes.
New implementor files seed.rs, tests/seed.rs, its fixtures and store/tests/seed_object_integrity.rs were
also inherited. My entire authored tree change is the new file below (first fixture copied from
the implementor's seed.rs; four adversarial cases and their helpers are mine):
```
.../ekr-kernel/tests/adversary_p1_08_seed.rs       | 461 +++++++++++++++++++++
 1 file changed, 461 insertions(+)
```
The no-index stat exits 1 because the new file differs from /dev/null; this is expected.

## Cases, first executions, then suite

All commands ran from the assigned unit tree with
`CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed/adversary`.

The new case file existed before any tests ran. Initial compilation failed because I used a struct
form for tuple variant ValueType::Record; property-filing-first.log preserves it. This typo is not
a finding. After fixing only my new test, the first executed case was:

`cargo test -p ekr-kernel --test adversary_p1_08_seed seed_ontology_property_definitions_must_be_filed_under_their_own_ids -- --exact`

Exit 101, one executed case, all four backend/property-map combinations accepted malformed schema.
The final regression additionally preserves a post-correction path: if the loader refuses, require
both conflicting IDs in that error, open valid compatibility configuration, submit the malformed
SeedDocument anyway, and require seed-ontology refusal with no seed event, bytes or content object.
A mere compatibility-ontology mismatch does not satisfy it. There is no skip or constructor-panic
acceptance. This final witness remains red; the post-correction branch is unexecuted until repair.
The implementor should separately test the loader's exact named error variant.

First execution and final witness output (only private path prefixes are elided in the public copy):
```
Compiling ekr-kernel v0.0.0 (<worktrees>/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.63s
     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 1 test
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... FAILED

failures:

---- seed_ontology_property_definitions_must_be_filed_under_their_own_ids stdout ----

thread 'seed_ontology_property_definitions_must_be_filed_under_their_own_ids' (222641) panicked at crates/ekr-kernel/tests/adversary_p1_08_seed.rs:221:5:
canonical seed admitted misfiled property definitions (backend, edge_property): [(Sqlite, false), (Sqlite, true), (File, false), (File, true)]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    seed_ontology_property_definitions_must_be_filed_under_their_own_ids

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.24s

error: test failed, to rerun pass `-p ekr-kernel --test adversary_p1_08_seed`
   Compiling ekr-kernel v0.0.0 (<worktrees>/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.39s
     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 1 test
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... FAILED

failures:

---- seed_ontology_property_definitions_must_be_filed_under_their_own_ids stdout ----

thread 'seed_ontology_property_definitions_must_be_filed_under_their_own_ids' (236448) panicked at crates/ekr-kernel/tests/adversary_p1_08_seed.rs:249:5:
canonical seed admitted misfiled property definitions (backend, edge_property): [(Sqlite, false), (Sqlite, true), (File, false), (File, true)]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    seed_ontology_property_definitions_must_be_filed_under_their_own_ids

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.26s

error: test failed, to rerun pass `-p ekr-kernel --test adversary_p1_08_seed`
```

Each following case then ran alone with the same cargo command prefix and `-- --exact`:

| Case | Log | Exit | Executed |
| --- | --- | --- | --- |
| losing_cached_seed_keeps_its_original_retention_class | cached-race-first.log | 0 | 1 |
| persisted_seed_format_and_context_fields_refuse_before_admission | persisted-format-first.log | 0 | 1 |
| inherited_record_properties_remain_typed_and_constraints_cannot_disappear | inherited-record-first.log | 0 | 1 |

The cached case races separately opened handles on both providers with both envelopes pre-retained as
Cache; exactly one initializes, the winner becomes Canonical and the loser remains Cache.
The format case injects unsupported input/envelope versions and unknown context/envelope fields through
raw test handles, then exercises real kernel replay; all refuse while retaining original bytes.
The inheritance case roundtrips and reopens an inherited required Record property whose user key is
"validation", and rejects an inherited opaque constraint.

After these individual runs:
`cargo test -p ekr-graph -p ekr-store -p ekr-kernel --no-fail-fast`
exited 101. Initial suite.log and final suite-final.log both executed 221: 220 passed, one failed.
The before count 217 is quoted from the implementor report, not collected by a pre-probe suite run.
Final suite result lines:
```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)
thread 'seed_ontology_property_definitions_must_be_filed_under_their_own_ids' (237455) panicked at crates/ekr-kernel/tests/adversary_p1_08_seed.rs:249:5:
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
error: test failed, to rerun pass `-p ekr-kernel --test adversary_p1_08_seed`
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: 1 target failed:
    `-p ekr-kernel --test adversary_p1_08_seed`
```
Full raw output is retained in suite-final.log; no tests are ignored or skipped.

`cargo clippy -p ekr-kernel --test adversary_p1_08_seed -- -D warnings` exited 0 both before and
after regression adaptation (clippy.log, clippy-final.log).
`cargo fmt --all -- --check` exited 0 both times (fmt.log, fmt-final.log).
`git diff --check` exited 0. No full task check is claimed.

## Finding

| Location | Severity | Verdict | Origin | Finding |
| --- | --- | --- | --- | --- |
| crates/ekr-kernel/src/seed.rs:153; delegated check crates/ekr-ontology/src/schema.rs:179 | blocker | NEEDS-CHANGE | undecided | Seed admission and restart accept ontology node and edge property definitions whose map key differs from their declared PropertyId. |

What was measured: the new case's final assertion at
crates/ekr-kernel/tests/adversary_p1_08_seed.rs:249 fails on SQLite/node, SQLite/edge, File/node and
File/edge. All four seeds initialize and reopen under real kernel authority. No forged event or
substitute authority is needed.

What reaches it: callers construct or decode SeedDocument, use the public compatibility constructor
with Ontology::load on that document's ontology, and call Commit::seed. The loader checks values only;
property lookup uses the map key while the stored definition names a different identity. Kernel
admission retains this contradictory ontology and replay repeats it. This violates the story's
coherent full-ontology/map-identity requirement and design §11.2's stable PropertyDefinition identity.

Correction point: the shared ontology declaration loader, for node and edge property maps, with a
named error identifying key and declared ID. Coordinator verified this route and will add the needed
ontology scope. Origin remains undecided: source inspection locates the unchecked loader in existing
code, but I did not execute the witness against opening commit 4032d00, and will not guess pre-existing.

Owners: 1 finding routed to implementor correction; coordinator owns scope expansion and final
normative citations. Historical defect origin is undecided.

## Bound of review

Initialization and replay both reach kernel seed admission; no raw Commit store accessor remains.
The old-to-new case map preserves semantic seed refusals at the kernel, while transaction forgery
cases retain seed authority so their transaction checks remain reachable.
Existing cases cover full ontology/context mismatch, genesis checks, evidence bytes, dangling
references, filing, lifecycle, property types, missing authority, repeated/concurrent initialization
and object metadata. The three new passing cases extend the boundaries listed above.
Event occurrence/version envelopes, post-seed writer application, full roots, durable transaction
receipts, historical reconstruction, explain and the already-recorded future seed-response race
remain excluded; no new current failure is claimed for them.
The AGENTS statement still needs the coordinator's final named-case citations.

## External writes and handback

Raw logs retain exact original output; the public-ready report replaces only private path prefixes.
The raw report is private scratch and lists every path in full:
- <cache>/ekr-completion-20260922/seed/adversary/
- <cache>/b10x-target/ekr-p1-08-seed/
- <cache>/ekr-completion-20260922/seed/adversary/property-filing-first.log
- <cache>/ekr-completion-20260922/seed/adversary/property-filing-executed.log
- <cache>/ekr-completion-20260922/seed/adversary/cached-race-first.log
- <cache>/ekr-completion-20260922/seed/adversary/persisted-format-first.log
- <cache>/ekr-completion-20260922/seed/adversary/inherited-record-first.log
- <cache>/ekr-completion-20260922/seed/adversary/suite.log
- <cache>/ekr-completion-20260922/seed/adversary/clippy.log
- <cache>/ekr-completion-20260922/seed/adversary/fmt.log
- <cache>/ekr-completion-20260922/seed/adversary/property-filing-final.log
- <cache>/ekr-completion-20260922/seed/adversary/suite-final.log
- <cache>/ekr-completion-20260922/seed/adversary/fmt-final.log
- <cache>/ekr-completion-20260922/seed/adversary/clippy-final.log
- <cache>/ekr-completion-20260922/seed/adversary/report.md
- <cache>/ekr-completion-20260922/seed/adversary/raw-report.md

Temporary provider data was created under assigned TMPDIR and removed by TempDir; persistent build
artifacts are under the assigned target above. Lease metadata changed only through managed hooks.
No process started by this review remains running. No source edits, commits, publication or cleanup.

```findings
- file: crates/ekr-kernel/src/seed.rs
  line: 153
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: Seed admission and restart accept ontology node and edge property definitions whose map key differs from their declared PropertyId.
```

