---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-02-views-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit R (graph projection renderer)
relations:
- reviews: story:graph-projection-renderer
revision: 1
---
unit: story:graph-projection-renderer, crates/ekr-views, worktree ekr-wave-a-r at head 86043d6c plus one untracked adversary test
verdict: confirmed (1 finding, red case)
cases: executed 14→15, red 1
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: 1 path (this report)
needs-coordinator: the fix may need a views.yaml change (the format has one property definition per id), and views.yaml is coordinator-held

## 1. Diff stat

`git --no-pager diff --stat`: empty. No tracked file changed.
`git status --short`: `?? crates/ekr-views/tests/adversary_property_redeclaration.rs` (93 lines, a test file). No non-test path touched.

## 2. Case added (written before anything ran)

`crates/ekr-views/tests/adversary_property_redeclaration.rs::a_property_redeclared_by_a_child_type_keeps_the_childs_definition_in_the_projection`

- **Setup:** seeds a store through `Runtime::seed` (file provider). Type `Base` declares property `measure` as String. Type `Refined` has parent `Base` and redeclares `measure` as Integer. `ekr-ontology` admits this shape on purpose (`crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs:40`).
- **Asserts:** some `ontology.properties` entry with `measure`'s id has `value_kind` Integer.
- **Status now:** red.

Red run of the case alone (`cargo test -p ekr-views --locked --test adversary_property_redeclaration`):

```
test a_property_redeclared_by_a_child_type_keeps_the_childs_definition_in_the_projection ... FAILED
thread '...' panicked at crates/ekr-views/tests/adversary_property_redeclaration.rs:88:5:
Refined redeclares `measure` as Integer, yet every definition the projection carries for that id says ["String"]: a reader of Refined is told the wrong value kind
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
```

## 3. Suite run (after the case existed)

Command: `CARGO_TARGET_DIR=<wave-root>/r/target CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 CARGO_INCREMENTAL=0 nice -n 19 cargo test -p ekr-views --locked --no-fail-fast`. The `bench` feature was off, so `render_time` did not run.

```
unittests src/lib.rs                     ok. 0 passed
adversary_property_redeclaration.rs      FAILED. 0 passed; 1 failed
conformance.rs                           ok. 3 passed
determinism.rs                           ok. 4 passed  (plus nested child-process runs: 1 passed; 3 filtered out, twice)
document.rs                              ok. 6 passed
reads_only.rs                            ok. 1 passed
doctests                                 ok. 0 passed
error: 1 target failed
EXIT=101
```

Top-level cases executed: 15. Without my binary (its 1 case deselected by subtraction from this same run): 14.

## 4. Findings

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr-views/src/document.rs:445-468 | CONFIRMED | introduced | `ontology.properties` holds one entry per property id. `definitions.entry(id).or_insert(first)` keeps the definition from the lowest-id node type and silently drops the rest. Red at `adversary_property_redeclaration.rs:88`: `["String"]` where `Refined` declares Integer. | `Runtime::seed` admits the ontology (the case seeds through the real kernel), and `ekr-ontology` supports redeclaration down a hierarchy by design. No shipped seed or example does it: the `crates/ekr/src/cli/examples/*.yaml` and `crates/ekr/tests/fixtures/*` seeds have `parents: []` and no redeclared ids. The same collapse also hits a node type and an edge type that share a property id. |

Why F1 matters: `ProjectedNodeType.properties` lists ids only, so `ontology.properties` is the only place a definition is written out. The document does not say it is lossy, and `render` returns `Ok`. views.yaml gives `ProjectedProperty` one global list keyed by id, so the format itself cannot hold two definitions for one id. Possible fixes (none applied):
- (a) views.yaml puts property definitions on each declaring type, or
- (b) the renderer refuses with `ProjectError::Inconsistent` when two definitions of one id differ, so it stops failing silently until (a) lands.

Tree covered: 86043d6c plus the untracked test.

## 5. Attacked and not broken

- Refusals: `at > head` gives RevisionNotFound with requested and head; `at == head` is projected; an unseeded store gives NotSeeded with `at` echoed.
- Determinism: nothing in `render` depends on HashMap order, the clock or the path. Every map is a BTreeMap over text, and the adjacent tag `kind` is written first.
- Scope to revision: `read(Some(revision))` and `replay(number)` per schema boundary. Version detection keys on `ontology_root`, a `ContentHash::of(ontology)` that includes the version record (`apply.rs:175`, `evolve.rs:81`), so no version bump is missed. Version-id reuse is refused (`validate/schema.rs:141`), so the `schemas.entry` dedup cannot merge two versions.
- The `Inconsistent` branch for an assertion on a deleted edge: unreachable. `DeleteEdge` of an edge that still carries assertions is refused by the reference validator (`validate/reference.rs:176-180`).
- Mutants:
  - removing the `evidence` sort: equivalent, because `Assertion.evidence` is a BTreeSet and UUID order is hex-text order.
  - removing `*first > graph.revision` or the `revisions` filter: equivalent for any input `load` builds.
  - `retained` always true: equivalent, because `retained: false` is unreachable (views.yaml UNMAPPED).
- Deviation: the brief asks for at least two mutations applied and reverted. My charter forbids mutating a file under attack, even briefly. I reasoned about the mutants above and changed no source file.

## 6. Paths written outside the worktree

- <wave-root>/r/scratch/adversary-pass-1.md (this report)
- Build output only in the assigned <wave-root>/r/target

## 7. Findings block


## Coordinator routing (2026-09-27)

- F1 → back to the implementor, fix (b): when two node or edge types carry different definitions for one property id, `render` refuses with `ProjectError::Inconsistent` naming the id, instead of projecting one silently. The adversary case is rewritten to assert that refusal. Fix (a), per-type property definitions in `ekr.graph-projection`, is filed as `task:projection-carries-per-type-property-definitions`. No shipped seed or example redeclares a property down a hierarchy (adversary, section 4).

```findings
[
{"file":"crates/ekr-views/src/document.rs","line":457,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"ontology.properties keeps one definition per property id, so a child type's redeclaration (admitted by ekr-ontology) is silently projected with its parent's value kind"}
]
```
