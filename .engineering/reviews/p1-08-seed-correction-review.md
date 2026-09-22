unit: story:kernel-validated-seed correction; uncommitted ekr-p1-08-seed on 4032d00
verdict: nothing found
cases: executed 3→3, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths, listed below
needs-coordinator: record prior blocker corrected; integrated gate and normative citation ownership remain coordinator's

`git --no-pager diff --stat 4032d00`, followed by the independent case SHA-256:
```
AGENTS.md                                          |  13 +-
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
 crates/ekr-ontology/src/schema.rs                  |  18 +-
 crates/ekr-ontology/tests/ontology_load.rs         |  43 +++
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
 29 files changed, 559 insertions(+), 1025 deletions(-)
a3152f04d6a02c28be6dc3b4327a7f8dec3fcf1ec1177c98a7d4fba78c4920ce  crates/ekr-kernel/tests/adversary_p1_08_seed.rs
```

Every tracked diff above is inherited implementation/correction or coordinator work.
I made no tree changes in this correction review. Since the original review, the implementor
changed only ontology/schema.rs and ontology/tests/ontology_load.rs; the coordinator additionally
reported an AGENTS named-case citation edit during this review. Source/tests remained stable.
The original independent case hash is unchanged:
a3152f04d6a02c28be6dc3b4327a7f8dec3fcf1ec1177c98a7d4fba78c4920ce.

## Prior finding disposition

The blocker at crates/ekr-kernel/src/seed.rs:153 is **corrected** in the reviewed working tree.
Shared Ontology::check_properties compares each map key against definition.id and returns typed
OntologyError::MisfiledProperty { key, declared }. Both node and edge property declaration maps
call this method; inherited properties are declarations on those same checked node types.

The two loader cases assert the exact variant and IDs through in-memory and YAML inputs, then
accept matching-ID controls. The unchanged kernel case now executes its corrected branch: it
opens valid compatibility configuration, submits malformed SeedDocument, requires seed-ontology
refusal (not a mere seed-ontology-mismatch), and checks no head, seed bytes or retained object.
This runs for SQLite/node, SQLite/edge, File/node and File/edge. No constructor panic or skip
counts as acceptance.

Owners: 1 prior implementor correction verified; 0 new findings. Original historical origin
remains undecided; this pass did not run a base-commit probe.

## Commands and runner output

All test commands ran in the assigned seed tree with:
`CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed/adversary`.

1. `cargo test -p ekr-ontology --test ontology_load property_definition_filed` — exit 0, 2 cases.
2. `cargo test -p ekr-kernel --test adversary_p1_08_seed seed_ontology_property_definitions_must_be_filed_under_their_own_ids -- --exact` — exit 0, 1 case.

```
Compiling ekr-ontology v0.0.0 (<worktrees>/ekr-p1-08-seed/crates/ekr-ontology)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ontology_load-772bbf00ea3f3baa)

running 2 tests
test a_node_property_definition_filed_under_another_id_is_refused_at_load ... ok
test an_edge_property_definition_filed_under_another_id_is_refused_at_load ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 1 test
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.13s
```

This is targeted correction verification: 3 executed, 3 passed, 0 failed. Before counts are the
two executed behavioral reds in correction-report.md plus this review's preserved original
kernel red; no pre-probe baseline suite was run. No new cases were authored this pass.
The implementor reports a four-package 290-pass suite and clippy/fmt success; this pass does not
claim to have rerun that suite, the full gate or unrelated writer/inspection work.

## External writes and handback

- <cache>/b10x-target/ekr-p1-08-seed/
- <cache>/ekr-completion-20260922/seed/adversary/
- <cache>/ekr-completion-20260922/seed/adversary/correction-loader.log
- <cache>/ekr-completion-20260922/seed/adversary/correction-kernel.log
- <cache>/ekr-completion-20260922/seed/adversary/correction-review.md
- <cache>/ekr-completion-20260922/seed/adversary/correction-review-raw.md

The public copy elides only private path prefixes; raw logs/report retain exact originals.
Temporary provider files stayed under assigned TMPDIR and were removed by TempDir.
Lease metadata changes use managed hooks only. Own correction-review lease released at handback;
the coordinator's separate lease is untouched. No process started by this review remains running.
No source, tests, docs, ESS or planning edits; no commit/publication/cleanup.

```findings
[]
```

