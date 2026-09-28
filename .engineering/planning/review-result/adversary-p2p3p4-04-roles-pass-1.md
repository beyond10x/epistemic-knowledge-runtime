---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-04-roles-pass-1
kind: review-result
status: active
title: Adversary pass 1 on wave C unit R (view roles)
relations:
- reviews: story:data-free-graph-viewer
revision: 1
---
unit: story:data-free-graph-viewer (wave C unit R, role derivation), worktree ekr-wave-c-r at head 485f997d plus one untracked adversary test file
verdict: red
cases: executed 74→82, red 2
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 15 paths under <scratch>/ (listed in part 6), plus `cargo clean -p ekr` in <build-dir>
needs-coordinator: decide whether F1 (type hierarchy gets no roles) holds the unit or becomes its own story

## 1. Diff

`git --no-pager diff --stat`: empty. `git status --short`: `?? crates/ekr/tests/adversary_roles_rule.rs`. That is the only path touched, and it is a test file. No implementation file, fixture or doc was edited.

## 2. Cases added (`crates/ekr/tests/adversary_roles_rule.rs`)

Each case applies `docs/cli.md` § Roles by hand to a store the unit's tests do not use, and serves it through the real `ekr view`.

| case | asserts | now |
|---|---|---|
| `a_store_whose_edge_types_name_abstract_parents_places_its_concrete_timed_types` | edge types declared on abstract parents, with nodes and a timed assertion on concrete children (the seed accepts this, since endpoints are checked with `conforms_to`, `crates/ekr-kernel/src/validate/types.rs:508`). The concrete timed type should be `event` and the concrete reading type `observation` | **red** |
| `the_documented_invalid_query_refusal_covers_every_query_but_revision_n` | the doc line "400 `invalid-query` for any query but `revision=N`" holds for `?&` and `?revision=+0` | **red** |
| `an_observation_needs_one_advancing_target_not_every_one` | an observation with one advancing target and one non-advancing target | green; kills mutant M1 |
| `a_valid_time_bounded_by_to_alone_makes_its_type_timed` | a valid time with only `to` set counts as timed | green; kills M2 |
| `a_retracted_timed_assertion_still_makes_its_type_timed` | "whatever its lifecycle": after revision 2 retracts the timed assertion committed at revision 1, 1103 is still `event` | green; kills M3 |
| `a_cycle_follows_the_rule_by_hand` | a 1101↔1102 cycle gives two events | green |
| `relabelling_the_type_ids_moves_the_roles_with_them` | type ids swapped so they sort in reverse order; the roles move with the ids | green |
| `every_query_is_answered_by_roles_as_by_projection` | 12 queries give `/roles` and `/projection` the same status, headers and refusal body; HEAD gives 405 with `Allow: GET` | green |

The first run of the cases alone had 3 red. Two of those failed for the wrong reason, so they are not findings: the seed refuses an edge type with an empty `source_types` (`ekr.kernel.InvalidSeed … declares no source_types`), and it refuses a seeded `!Retracted` lifecycle (`assertion-states-its-own-verdict`). I dropped the empty-list half and rewrote the retraction case to go through propose, validate and commit. Red runs, captured verbatim from each case run alone:

```
---- a_store_whose_edge_types_name_abstract_parents_places_its_concrete_timed_types stdout ----
thread '…' panicked at crates/ekr/tests/adversary_roles_rule.rs:478:5:
assertion `left == right` failed: the concrete, timed incident type that points on: {"9103": "subject", "9105": "subject"}
  left: None
 right: Some("event")
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.45s
```
```
---- the_documented_invalid_query_refusal_covers_every_query_but_revision_n stdout ----
assertion `left == right` failed: GET /roles?&: {"format":"ekr.view-roles/1","revision":0,"node_types":[…1101 observation, 1102 event, 1103 subject, 1104 subject…]}
  left: 200
 right: 400
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.84s
```

Mutants were run in a scratch copy of the tree. The worktree file was never edited. Each mutant was run against the unit's `view_roles` and against the cases above:

| mutant | line | unit `view_roles` | killed by |
|---|---|---|---|
| M1 `any` → non-empty `all` | `crates/ekr/src/cli/view_roles.rs:131` | 3 passed (survives) | `an_observation_needs_one_advancing_target_not_every_one` |
| M2 `from \|\| to` → `from` only | `:95` | 3 passed (survives) | `a_valid_time_bounded_by_to_alone_makes_its_type_timed` |
| M3 skip non-`Active` lifecycles | after `:92` | 3 passed (survives) | `a_retracted_timed_assertion_still_makes_its_type_timed` |

## 3. Suite run (after the cases existed)

`nice -n 19 cargo test -p ekr --locked --no-fail-fast --lib --test view_roles --test view_cli --test docs_cli --test public_surface --test adversary_roles_rule`, CARGO_TARGET_DIR=<build-dir>:

```
unittests src/lib.rs      test result: ok. 35 passed; 0 failed
adversary_roles_rule.rs   test a_store_whose_edge_types_name_abstract_parents_places_its_concrete_timed_types ... FAILED
                          test the_documented_invalid_query_refusal_covers_every_query_but_revision_n ... FAILED
                          test result: FAILED. 6 passed; 2 failed
docs_cli.rs               test result: ok. 17 passed; 0 failed
public_surface.rs         test result: ok. 12 passed; 0 failed
view_cli.rs               test result: ok. 7 passed; 0 failed
view_roles.rs             test result: ok. 3 passed; 0 failed
EXIT=101
```

The before count, 74, is the implementor's `<scratch>/green.log` for the same five targets: 35+17+12+7+3.

A red suite is the result. `rustfmt --check` on my file is clean. `cargo clippy -p ekr --test adversary_roles_rule -- -D warnings` exits 0.

One deviation, which I caused and have fixed. The scratch mutant tree built into the unit's build dir, and cargo gives workspace crates path-independent hashes. As a result, an intermediate suite run (`<scratch>/adv-pass-1-suite-2.log`) ran the mutant-tree binaries: it shows 7 cases, not 8. **Discard that log.** I deleted the mutant tree and ran `cargo clean -p ekr` in <build-dir> (removed 2.9 GiB). Then I rebuilt from the worktree and reran everything. The run above comes after that clean. The build dir now holds the worktree's own `ekr` artifacts.

## 4. Findings

Coverage: head 485f997d plus the untracked adversary file.

- **F1** `crates/ekr/src/cli/view_roles.rs:81` (rule step 1, "`parents` are not consulted", also `docs/cli.md:333`). Verdict: confirmed. Origin: introduced. Severity: medium.
  - What was measured: take an ontology whose edge types name abstract parents, with nodes, edges and timed assertions on concrete children. It is placed as `{9103 abstract: subject, 9105: subject}`. The two concrete types get no role and nothing is an event.
  - What reaches it: `ekr seed` accepted this store. The kernel checks endpoints with `conforms_to`, so an ordinary inheritance-shaped ontology lands here. The story says the viewer should read "a store with any ontology".
  - The doc and the code agree. The defect is that the rule gives empty roles for a shape the runtime supports.
  - Fix, named and not applied: expand each edge type's `source_types`/`target_types` to every declared type that `conforms_to` them before drawing arcs. Update the doc line to match.
- **F2** `docs/cli.md:325`. Verdict: confirmed. Origin: introduced. Severity: low.
  - The unit's new sentence "400 `invalid-query` for any query but `revision=N`" is false for `?&` and `?revision=+0`: both are answered 200.
  - Cause: `revision()` skips empty pairs (`crates/ekr/src/cli/view.rs:436`) and accepts a leading `+` through `u64::from_str` (`:440`). That parser is pre-existing and shared with `/projection`. The false claim is this unit's.
  - What reaches it: a hand-typed URL. No caller found.
  - Fix: either narrow the doc sentence, or refuse empty pairs and non-digit values in the parser.
- **F3–F5** `crates/ekr/src/cli/view_roles.rs:131`, `:95`, `:92`. Verdict: confirmed. Origin: introduced. Severity: low.
  - The unit's suite does not pin three clauses of its own documented rule: "at least one target", "`from` or `to`", and "whatever its … lifecycle". Mutants M1–M3 each leave `view_roles` green.
  - The green adversary cases now pin all three.
  - What reaches it: any future edit to those lines.

## 5. Attacked and not broken

- No name or id is compared to a constant. A reverse-sorted id relabelling moves the roles with the ids, so there is no hidden id-order dependence (the unit's test 3 keeps ids and could not have shown this).
- Output is deterministic: BTreeMap/BTreeSet throughout, and entries are ordered by `type_id`.
- Symmetric edge types and self-loops match the doc (the `sessions`/`readings` fixtures cover them).
- A cycle gives two events.
- A timed type added at a later revision is placed per revision (revision 0 stays `subject`).
- A retracted assertion still counts as timed.
- An empty `source_types` cannot be reached: the seed refuses it.
- `/roles` refusals, headers and HEAD/POST 405 match `/projection` on 12 queries. Nothing in the roles derivation is wrong for those.

## 6. Paths written outside the worktree

All under <scratch>/:
- `adv-pass-1-cases.log`
- `adv-pass-1-cases-2.log`
- `adv-pass-1-case-query.log`
- `adv-pass-1-case-query-2.log`
- `adv-pass-1-suite.log` (an early run that stopped at the first red binary)
- `adv-pass-1-suite-2.log` (invalid, see part 3)
- `adv-pass-1-suite-3.log`
- `adv-clippy.log`
- `adv-mutate.sh`
- `adv-mutant-all.log`
- `adv-mutant-from.log`
- `adv-mutant-active.log`
- `adversary-pass-1.md`
- `adv-mutant-tree/` (already deleted)

Also:
- <build-dir>: `cargo clean -p ekr` and a rebuild.
- `tempfile` test directories under the shared TMPDIR, removed on drop.

## 7. Findings block


## Coordinator routing (2026-09-27)

- F1 → back to the implementor: before arcs are built, each edge type's source and target types widen to every type that conforms to them, as the kernel checks endpoints; the doc says so.
- F2 → back to the implementor: the shared query parser refuses what the doc says it refuses: an empty pair and a revision that is not plain ASCII digits get 400 `invalid-query`, on `/roles` and `/projection` alike.
- F3, F4, F5 → no-op: the adversary cases pin the three clauses and stay.
- Process note: the adversary edited its own test file with a one-off `python3 -c`; nothing Python was committed.

```findings
[
{"file":"crates/ekr/src/cli/view_roles.rs","line":81,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"edge types declared on abstract parents give their concrete, node-holding, timed subtypes no role, so a type-hierarchy store the kernel accepts shows no event"},
{"file":"docs/cli.md","line":325,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the new doc promises 400 invalid-query for any query but revision=N, yet ?& and ?revision=+0 are answered 200"},
{"file":"crates/ekr/src/cli/view_roles.rs","line":131,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"mutating any target advancing to every target advancing leaves the unit's view_roles suite green"},
{"file":"crates/ekr/src/cli/view_roles.rs","line":95,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"dropping the to bound from the timed test leaves the unit's view_roles suite green"},
{"file":"crates/ekr/src/cli/view_roles.rs","line":92,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"skipping non-Active assertions when deciding timed leaves the unit's view_roles suite green"}
]
```
