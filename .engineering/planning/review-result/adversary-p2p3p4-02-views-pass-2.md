---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-02-views-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit R (graph projection renderer)
relations:
- reviews: story:graph-projection-renderer
revision: 1
---
unit: story:graph-projection-renderer, crates/ekr-views, worktree ekr-wave-a-r at head 68d669d3 plus one untracked adversary test
verdict: CONFIRMED (1 red case, 1 judgement note)
cases: executed 17→20, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (scratch logs and this report), plus build output in the assigned target dir
needs-coordinator: F3 needs a views.yaml change (coordinator-held); the fix for F2 is one line in document.rs, which I did not apply

## 1. Diff stat

`git --no-pager diff --stat`: empty. No tracked file changed.
`git status --short`: `?? crates/ekr-views/tests/adversary_pass2.rs`. That is a test file, and it is the only path I touched in the tree. No non-test path.

## 2. Cases added (`crates/ekr-views/tests/adversary_pass2.rs`)

| case | asserts | now |
|---|---|---|
| `a_redeclaration_that_differs_only_in_what_the_format_does_not_carry_is_projected` | Base declares `measure` (String, name "measure"). Refined, a child of Base, redeclares it identically except `required: true`. The render returns Ok with one `ontology.properties` entry: name "measure", value_kind String | **red** |
| `a_redeclaration_that_differs_only_in_name_is_refused` | same value type, different name: refused as Inconsistent. This catches the mutant that narrows the comparison to `value_type` | green |
| `a_superseded_assertion_projects_its_lifecycle_and_closed_valid_time_only_from_its_revision` | uses the real kernel (seed, then add, then add + SupersedeAssertion). At rev 1: `{"kind":"Active"}`, no valid_to or recorded_to. At rev 2: Superseded with at_revision 2, by, effective_from 2000, no reason; valid_to 2000; recorded_to present; raw bytes in declared key order | green |

The first runs of the supersession probe failed because of my own setup: the assertion cited no evidence, then serde_json re-sorted the keys. I fixed both before calling it green. Neither failure is a finding.

Red run of the file alone (`cargo test -p ekr-views --locked --test adversary_pass2`):

```
test a_redeclaration_that_differs_only_in_what_the_format_does_not_carry_is_projected ... FAILED
thread '...' panicked at crates/ekr-views/tests/adversary_pass2.rs:98:23:
Base and Refined both declare `measure` with name "measure" and value kind String, which is all ekr.views.ProjectedProperty carries; the format represents this revision exactly, yet the render refused: the projected revision is inconsistent: property 00000000-0000-4000-8000-000000000402 is declared by type 00000000-0000-4000-8000-000000000400 and by type 00000000-0000-4000-8000-000000000401 with different definitions, and ekr.graph-projection/1 carries one definition per property id (task:projection-carries-per-type-property-definitions)
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.51s
EXIT=101
```

## 3. Suite run (after the cases existed)

Command: `CARGO_TARGET_DIR=<wave-root>/r/target CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 CARGO_INCREMENTAL=0 nice -n 19 cargo test -p ekr-views --locked --no-fail-fast`. The `bench` feature was off, so `render_time` did not run.

```
unittests src/lib.rs                      ok. 0 passed
adversary_pass2.rs                        FAILED. 2 passed; 1 failed
adversary_property_redeclaration.rs       ok. 3 passed
conformance.rs                            ok. 3 passed
determinism.rs                            ok. 4 passed (plus nested child-process runs)
document.rs                               ok. 6 passed
reads_only.rs                             ok. 1 passed
doctests                                  ok. 0 passed
error: 1 target failed
EXIT=101
```

Top-level cases executed: 20. The before count is 17: I subtracted my 3 cases from this same run. `cargo fmt -p ekr-views -- --check` exits 0. `cargo clippy -p ekr-views --all-targets --locked -- -D warnings` reports no warnings.

## 4. Findings (tree: 68d669d3 + untracked test)

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F2 | crates/ekr-views/src/document.rs:470 | CONFIRMED | introduced | `held != property` compares the whole `PropertyDefinition`: required, cardinality, constraints and the full value_type. The format carries only id, name and value_kind (views.yaml `ekr.views.ProjectedProperty`). So the render refuses a revision it could represent exactly, which contradicts the `ProjectError::Inconsistent` doc at lib.rs:103 ("cannot represent without losing part of it"). Red at adversary_pass2.rs:98. The same refusal hits Enum variants, NodeRef targets and List element types, all of which leave value_kind unchanged | `Runtime::seed` admits the ontology (the case seeds through the real kernel). `ModifyProperty` evolution on one owner reaches the same state. No shipped seed or example has a hierarchy (pass 1). At 86043d6c this rendered Ok, so the correction commit introduced it. Once reached, every later revision of the store is refused too |
| F3 | systems/ekr/domains/views.yaml (`ProjectGraph` outcomes, :388) | INFEASIBLE here | introduced | `ProjectGraph` declares three outcomes: projected, not-found, not-seeded. Since 68d669d3, `render` has a fourth refusal (`Inconsistent`) on a state the kernel admits. The domain does not declare it, and no scenario can name it | Same reach as F2 and as the pass-1 state. views.yaml is coordinator-held |

Fix for F2 (named, not applied): compare `(name, value_type.kind())` instead of the whole definition.

## 5. Attacked and not broken

- Supersession lifecycle, valid_to and recorded_to, both at the revision before and at the revision after (green case above).
- Retraction at earlier revisions: `read(Some(r))` replays to r. Lifecycle `at_revision` is the new revision (apply.rs:122/141).
- Schema lineage: one version per transaction (`GraphTransaction.schema_version`), and a seed version with a parent is refused (seed.rs:171). So the parent-missing refusal at document.rs:545 is unreachable. `added` matches the spec for a redeclaration-only evolution (empty) and for a property id shared with the parent (not added).
- File vs SQLite: `both_providers_render_every_revision_of_one_fixture_to_the_same_bytes` covers every revision of the evolved fixture.
- Evidence and retained bytes: `retained` is keyed by content hash at the projected revision, and `retained: false` is unreachable (views.yaml UNMAPPED). Duplicate hashes are harmless.
- Edge endpoints: an assertion on an edge that the revision lacks is unreachable (reference validator). Endpoints are passed through as-is.
- Large stores: not measured. Per the dispatch, the bench-feature test was not run. `load` replays once per schema version, so cost grows with versions × revisions. That is a performance note, not a finding.
- Mutants on the correction: dropping the `edge_types` chain is caught by the implementor's node/edge case. Narrowing the comparison to `value_type` is caught by my name-only case. Neither mutant was applied to a file: the charter forbids it.

## 6. Paths written outside the worktree

- <wave-root>/r/scratch/pass2-red.log
- <wave-root>/r/scratch/pass2-suite.log
- <wave-root>/r/scratch/adversary-pass-2.md (this report)
- build output only in the assigned <wave-root>/r/target

## 7. Findings block


## Coordinator routing (2026-09-27)

- F2 → back to the implementor: the refusal compares only what `ekr.views.ProjectedProperty` carries (name and value kind); two declarations that agree on those project once.
- F3 → no-op for the unit: the interim `Inconsistent` refusal exists until `task:projection-carries-per-type-property-definitions` gives each type its own definition, which removes it. `views.yaml` stays unchanged in this wave; the task body names this gap.
- Second pass: the correction is verified by the coordinator, no third attack.

```findings
[
{"file":"crates/ekr-views/src/document.rs","line":470,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the redeclaration refusal compares whole PropertyDefinitions, so a revision whose two declarations agree on id, name and value_kind (all the format carries) is refused instead of projected"},
{"file":"systems/ekr/domains/views.yaml","line":388,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"ProjectGraph declares only projected, not-found and not-seeded, while render now refuses a kernel-admitted revision with Inconsistent, an outcome the domain does not declare"}
]
```
