---
format: aep.planning-md/3
id: review-result:adversary-extraction-parity-unit-a-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: EKR wave 20261005b unit A (extraction parity)'
relations:
- reviews: story:extraction-partial-apply
- reviews: story:extraction-valid-time
- reviews: story:extracted-relations-visible-to-graph-reads
- reviews: story:extraction-supersession
revision: 1
---
I found 6 defects in unit A, each with a test that fails, so the unit is held. Three are worth stopping the merge over: a refused replacement still creates its node, a valid replacement is refused because an earlier one failed, and a relation is stored with no edge.

```
unit: A of wave 20261005b (story:extraction-partial-apply, -valid-time, extracted-relations-visible-to-graph-reads, -supersession); uncommitted working tree on base 47f17ec94 in ekr-wx-a
verdict: NEEDS-CHANGE
cases: executed 2230→2236, red 6
origin: introduced 5 / pre-existing 0 / undecided 2
wrote-outside-worktree: 3 paths (listed in part 6)
needs-coordinator: whether a replacing fact that is held should still supersede the other active values (case 6), and whether the SDK path must refuse a relation with the wrong ends the way the CLI does (case 5)
```

**1. Diff stat.** `git --no-pager diff --stat` reads `19 files changed, 1721 insertions(+), 190 deletions(-)`. That is exactly what I was handed: I changed no tracked file. My only addition is one new, untracked test file, `crates/ekr/tests/adversary_wx_a_1.rs`. No non-test path is touched.

**2. Cases added.** All six are in `crates/ekr/tests/adversary_wx_a_1.rs` under the heading "ADVERSARY CASES". All are red now. Each was run alone first, on both storage backends (file, sqlite), and its log is `adversary-1/red-<name>.log`. The red output below is verbatim from those solo runs:

| Case | What it asserts | Red output (solo run) |
|---|---|---|
| `a_replacement_refused_for_nothing_to_replace_creates_no_node` | A `replaces` fact about a new organisation "Nova" is refused with `replacement-without-active-assertion`, and no Nova node is created (via the CLI and the SDK) | `file/verb: the node only the refused replacement names was created`, `left: Some("01a10cce-5de4-…") right: None` |
| `a_valid_replacement_is_not_refused_for_a_refused_one_before_it` | Store has Globex = "Globex Corporation". Fact 0 replaces it with older evidence (correctly refused, `invalid-supersession`). Fact 1 replaces it with newer evidence and should apply. | `left: ["facts[0]", "facts[1]"] right: ["facts[0]"]`. Fact 1 fails with `unresolved-assertion: assertion … is not in the graph`: it was made to supersede fact 0's assertion, which never committed. |
| `a_relation_read_twice_after_its_edge_was_refused_is_not_asserted_without_an_edge` | Rigel is already CEO_OF Betelgeuse (cardinality One). A document says Rigel CEO_OF Mintaka twice, from two sources. Both facts should be refused; any active relation of Rigel must have an edge. | `an active relation assertion of Rigel with no edge for graph reads` (the fact citing 403). The report rejects only `facts[0]` (`edge-cardinality`); fact 1 committed with no edge. |
| `a_fact_citing_part_of_an_earlier_claims_evidence_is_still_held` | Fact 0 says Lyra = "Lyra Group" citing items 401 (early) and 402 (late); fact 1 says the same citing 402 only. Before this unit, fact 1 was held as repeated. | `left: Array [] right: [{"item":"facts[1]","reason":"repeated"}]`. Fact 1 is asserted again, giving two active assertions of the same value. |
| `on_the_sdk_path_a_relation_refused_for_its_ends_creates_no_node` | An Organization CEO_OF Person relation (wrong ends) is the only fact naming "Dora". The CLI skips it and creates nothing; the SDK path should do the same. | The CLI part passes. SDK: `left: Some("01a10cce-7cad-…") right: None`. The kernel rejects the assertion and its edge (`edge-endpoint-type`) only after Dora was created. |
| `a_held_replacement_still_leaves_one_active_value` | Store holds two active legal names for Globex. Re-applying the known document with `replaces: true` should leave one. | `left: 2 right: 1`. The fact is held as `asserted` before replacement is considered, so nothing is superseded. |

**3. Suite runs (after the cases existed).**
- `task check > adversary-1/gate.log` ended `task: Failed to run task "test": exit status 101`, `EXIT=201`. Formatting and clippy passed before it; 55 `test result:` lines.
- `task test` stops at the first failing test binary, so the gate count is partial. For a full count I ran `cargo test --workspace --locked --no-fail-fast > adversary-1/suite-nofailfast.log`: `EXIT=101`, 361 `test result:` lines, 2236 run (2230 passed, 6 failed, 13 ignored). The only failing target is ``-p ekr --test adversary_wx_a_1``.
- The 2230 "before" figure is the same `cargo test --workspace` step in the implementor's `a/gate.log`. I compared it binary by binary; the difference is exactly my 6 cases.
- This run finished before the coordinator deleted the build directory, so it did not need a rebuild.

**4. Findings, and what reaches each.** All findings are against the working tree on 47f17ec94 and are in `crates/ekr-sdk/src/extraction.rs`.
- **Cases 2 and 3, and finding 7 below, share one root cause.** `Run::go` updates its record of what this run has done (active values at line 927, edges at line 957, claims at line 940) when a fact is proposed, not when its transaction commits. When the kernel rejects a fact, later facts are still planned as if it had committed.
  - Fix: put a fact that depends on an earlier fact's change (superseding its assertion, or relying on its edge) in the same group as that fact, or rebuild the dependent groups from the batch's rejections.
- **Case 1, line 908.** The "nothing to replace" refusal is decided after names are resolved and new nodes created (line 810).
  - Reached by: `ekr apply-extraction` with a `replaces` fact about a new entity. The unit's own test does this with "Nova" but never checks the node.
  - Fix: decide the refusal while planning, from the snapshot, so the fact's subject is never created.
- **Case 2, line 927.** Reached by: two replacements of one property in one document where the first predates the active value, for example a registry history.
- **Case 3, line 957.** Reached by: one relation read from two sources where the edge breaks a One cardinality, through the CLI. This contradicts the coordinator decision ("an edge the kernel refuses refuses the fact, assertion and all") and `integrate.yaml` ("unless an edge of its type already joins the two nodes").
- **Case 4, lines 879-885.** Two facts are now the same claim only if their valid-time start matches, and the only fallback is an unbounded start. Reached by: overlapping batches that cite a subset of earlier evidence.
- **Case 5, line 357.** The SDK's own fact check does not look at relation ends or value types. The assertion itself is rejected by the kernel's type validator, so this mechanism probably existed at base too. I could not run it there, so its origin is undecided.
- **Case 6, lines 881-892.** Holding is checked before replacement. Reached only after a non-replacing fact has left two active values.
- **Finding 7 (no test), line 940.** A fact with the same evidence as an earlier fact the kernel rejected is reported as `held: repeated`, though nothing was asserted. Through the CLI this is newly reachable via an edge refusal. The same mechanism is at base (claims pushed before the group commits), but I did not run it there, so the origin is undecided.

**5. Attacked and could not break.**
- Baseline digests: final `4c22023c…` matches `suite.json`. Applying `story-3.diff` to a scratch copy reproduces the implementor's `8a571c68…` for the story 1-3 commits.
- Conformance passes `--strict` only for the two suite cases that set `strict: true`.
- Strict versus default on the CLI: the reader's codes are used per fact, and a whole-document defect still exits 2 (covered by the implementor's tests).
- Re-applying a 0.0.30 document with an unbounded claim adds nothing (the implementor's test passes).
- By reading only, not run:
  - Replacing a value set earlier in the same document passes the kernel's lifecycle rules (`lifecycle.rs`, a Proposed assertion in the same transaction counts as accepted).
  - With equal timestamps, bisection still converges.
  - Evidence dated in the future is accepted, because no kernel validator checks the start of valid time.

**6. Paths written outside the worktree.**
- `~/.cache/ekr-wave-8c9138d1/a/adversary-1/`: `tmp/`, `build.log`, six `red-*.log`, `fmt.diff`, `gate.log`, `suite-nofailfast.log`, `suite-nofailfast.log.counts`, `digest-check/systems/ekr/conformance/suite.json`.
- `~/.cache/ekr-wave-8c9138d1/a/gate.log.counts`. This one sits in the implementor's directory by mistake; it is a derived count file and can be deleted.
- `~/.cache/b10x-target/ekr-wx-a`, the build directory, already deleted by you.

My worktree session lease (`ekr-wx-a-adversary-1`) was acquired and has been released.

**7. Findings block.**
```findings
[
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 908, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A replaces fact refused for having nothing to replace is refused after names are resolved, so the new node only it names is created, against partial-apply decision 4."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 927, "category": "boundary", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The active-value record is replaced when a replacement is proposed, so a later valid replacement supersedes a rejected fact's uncommitted assertion and fails with unresolved-assertion."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 957, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A refused edge still counts as joining its two nodes, so a second fact of the same relation commits as an assertion with no edge, invisible to graph reads, against the coordinator decision and integrate.yaml."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 879, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Claims are now matched on their own earliest observed time, so a fact citing a subset of an earlier claim's evidence is no longer held and adds a second active assertion of the same value."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 357, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "undecided", "message": "On the SDK path a relation with the wrong endpoint types is not refused before writing, so the node only it names is created before the kernel rejects it, unlike the CLI path."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 881, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A replaces fact whose claim is already active is held before replacement is considered, so other active values of that subject and property are not superseded."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 940, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "undecided", "message": "A claim is recorded as repeated before its group commits, so a same-evidence fact after a kernel-rejected one is reported held as repeated although nothing was asserted."}
]
```