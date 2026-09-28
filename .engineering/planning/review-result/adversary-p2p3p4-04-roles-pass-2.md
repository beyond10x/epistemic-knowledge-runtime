---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-04-roles-pass-2
kind: review-result
status: active
title: Adversary pass 2 on wave C unit R (view roles)
relations:
- reviews: story:data-free-graph-viewer
revision: 1
---
unit: story:data-free-graph-viewer (wave C unit R, role derivation), pass 2 on the correction 485f997d..f92c8d0f; worktree ekr-wave-c-r at head f92c8d0f plus one untracked adversary test file
verdict: red
cases: executed 291→293, red 1
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: 6 paths under <scratch>/ (part 6); two of them (a tree copy and its target dir) already deleted
needs-coordinator: no

## 1. Diff

`git --no-pager diff --stat`: empty. `git status --short`: `?? crates/ekr/tests/adversary_roles_rule_2.rs`. That is the only path touched, and it is a test file. I edited no implementation file, fixture or doc. I mutated a copy under <scratch>/, never the worktree.

## 2. Cases added (`crates/ekr/tests/adversary_roles_rule_2.rs`)

| case | asserts | now |
|---|---|---|
| `a_schema_change_under_a_self_loop_type_keeps_the_docs_closing_sentence_true` | seeds `readings` under a v2 host, then commits a schema change declaring 1106 with parent 1105 (1105 is on the self-loop `REFINES_MARKER` only). The case first checks that revision 0 still has revision 0's roles. It then checks that `/roles?revision=1` agrees with the closing sentence of `docs/cli.md` § Roles, "a type only on self-loops or on no edge type has no role", for as long as the doc contains that sentence | **red** |
| `widening_is_transitive_through_a_diamond_and_reverses_a_symmetric_edge_to_descendants` | a diamond two levels deep under an abstract root, an edge type on the root, one on a middle type, and a symmetric edge type with an abstract endpoint. The expected roles were worked out by hand from step 1 | green. It kills mutants MA and MB, which the rest of the suite does not |

Red output, from running that case alone (`cargo test -p ekr --locked --test adversary_roles_rule_2`, exit 101):

```
test widening_is_transitive_through_a_diamond_and_reverses_a_symmetric_edge_to_descendants ... ok
test a_schema_change_under_a_self_loop_type_keeps_the_docs_closing_sentence_true ... FAILED
thread 'a_schema_change_under_a_self_loop_type_keeps_the_docs_closing_sentence_true' (3001703) panicked at crates/ekr/tests/adversary_roles_rule_2.rs:281:5:
docs/cli.md says "a type only on self-loops or on no edge type has no role", yet at revision 1 the type only on a self-loop (1105) is Some("subject") and the type on no edge type (1106) is Some("subject"): {"1101": "observation", "1102": "event", "1103": "subject", "1104": "subject", "1105": "subject", "1106": "subject"}
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.79s
```

Mutants, applied to a copy of the tree under <scratch>/ and built into a separate target dir there (both deleted afterwards). Tests run: `--lib --test view_roles --test adversary_roles_rule --test adversary_roles_rule_2 --no-fail-fast`:

| mutant | lib (35) | view_roles (3) | adversary_roles_rule, pass 1 (8) | new diamond case |
|---|---|---|---|---|
| MA: widening goes one level (the type itself or a direct child), not transitive | ok | ok | ok | FAILED (9104 is not an event, 9106 is unplaced) |
| MB: a symmetric edge type's reverse arcs come from the declared lists, not the widened ones | ok | ok | ok | FAILED (`"9108": "subject"` where `"event"` is expected) |

## 3. Suite, run after the cases existed

`cargo test -p ekr --locked --no-fail-fast` (nice 19, 2 jobs, unit build dir): exit 101. 45 test binaries, 292 passed and 1 failed. The only failure:

```
     Running tests/adversary_roles_rule_2.rs (…/adversary_roles_rule_2-a02f084f92fe0a6c)
test a_schema_change_under_a_self_loop_type_keeps_the_docs_closing_sentence_true ... FAILED
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
```

The same run passed `view_cli` 7, `view_roles` 3, `adversary_v_view` 4, `adversary_v_view_r2` 6, `adversary_roles_rule` 8, lib 35 and `docs_cli` 17. So the strict parser broke no existing caller or test. `executed <before>` is 293 minus the 2 cases in `adversary_roles_rule_2`, taken from this same run. I did not make a second run. `cargo fmt -p ekr -- --check`: exit 0. I did not run clippy on the new file.

## 4. Findings

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F6 | docs/cli.md:357 | NEEDS-CHANGE | introduced (the sentence was true at 485f997d; f92c8d0f made it false) | the case at adversary_roles_rule_2.rs:251 is red. After the correction, 1105 (only on a self-loop) and 1106 (on no edge type) are both `subject`. Step 1 of the same doc predicts that, and its closing sentence predicts no role | any child type added under a type whose edge types are all self-loops, by seed or by a v2 schema change (`DefineNodeType` with `parents`). The documented schema evolution workflow reaches it. Fix: rewrite the sentence in terms of widened arcs, e.g. "a type with no arc to or from another type after widening has no role" |
| F7 | crates/ekr/src/cli/view_roles.rs:93 | CONFIRMED | introduced | mutants MA and MB leave lib, view_roles and the pass-1 adversary cases green. No unit fixture has a `parents` entry, so the only widening coverage before this pass was one pass-1 case, one level deep and not symmetric | the new diamond case kills both. It is green now |

Everything above is for head f92c8d0f plus the untracked test file.

## 5. Attacked and not broken

- Diamonds and multiple inheritance: `conforms_to` walks every parent, and 9104 (two parents, two levels) was widened correctly.
- A type conforming to both endpoints: C→C is dropped as a self-arc, A→C and C→B are kept. This matches step 1, and the kernel accepts the same endpoints (`validate/types.rs:508`, same direction).
- Schema evolution: `/roles?revision=0` read revision 0's ontology after a later `DefineNodeType`, so the widening does not leak later types backwards.
- Symmetric edge types with abstract endpoints: the reverse arcs reach descendants.
- Strict parser against callers: no test or doc used an empty pair or a trailing `&`. The page builds `projection?revision=` + `encodeURIComponent(URLSearchParams.get("revision"))`, so the value is digits or already refused before and after the change. `ekr view --help` (`cli/mod.rs:231`) still says `[?revision=N]`.
- The module docs and `docs/cli.md` step 1 and step 3 state the widening and "own type_id, not ancestors" the same way the code does.

## 6. Paths written outside the worktree

- <scratch>/adv-pass-2-cases.log
- <scratch>/adv-pass-2-mutant-a.log
- <scratch>/adv-pass-2-mutant-a2.log
- <scratch>/adv-pass-2-mutant-b.log
- <scratch>/adv-pass-2-suite.log
- <scratch>/adversary-pass-2.md (this report)
- <scratch>/mutant-tree and <scratch>/mutant-target: deleted.
- The unit build dir <build-dir> was used for the unmutated case and suite runs only.


## Coordinator routing (2026-09-27)

- F6 → fixed by the coordinator (the correction a second pass allows): the closing sentence of `docs/cli.md` § Roles now reads "a type with no arc to or from another type after widening has no role"; the adversary case and docs_cli pass.
- F7 → no-op: the diamond case kills both mutants and stays.

```findings
[
{"file":"docs/cli.md","line":357,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"after widening, a type only on a self-loop and its new child are both subject, while the doc closing sentence still promises them no role"},
{"file":"crates/ekr/src/cli/view_roles.rs","line":93,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"non-transitive widening and declared-only symmetric reversal both left the unit suite and pass-1 cases green"}
]
```
