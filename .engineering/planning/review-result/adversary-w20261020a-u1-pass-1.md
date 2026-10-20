---
format: aep.planning-md/3
id: review-result:adversary-w20261020a-u1-pass-1
kind: review-result
status: active
title: Adversary pass 1, wave 2026-10-20a U1 (review fixes)
relations:
- reviews: story:supersede-node-review-fixes
revision: 1
---
Adversary pass 1 (`aep:adversary`) on wave 2026-10-20a unit U1, `story:supersede-node-review-fixes`,
tree `ekr-w20a-u1` at `5127539c81`, recorded by the coordinator as it returned below.

Owners: 3 findings, 2 implementor (F1, F3), 1 coordinator (F2: the brief's decision 2 did not say
what `strict` does when the group re-check refuses).

---

unit: story:supersede-node-review-fixes (wave 2026-10-20a U1). Covers head 5127539c81 plus my 2 untracked test files.
verdict: NEEDS-CHANGE
cases: executed 1588→1591, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths under <home>/.cache/ekr-w20a-u1-scratch/adversary-1/ (list in part 6)
needs-coordinator: F2 needs a decision. Either `ApplyOptions::strict` refuses the whole document when a group of named things lands on a node of another type, or the SDK doc says strict does not cover that case.

**Kernel-verdict invariant: it holds.** In the kernel, the diff changes only the D9 lookups: `candidate.rs:207`, `reference.rs:475`, `structural.rs:841` and `:923`. Each one reads `(node, by)` or `is_some()` from the same snapshot's records as before. D7, D8, D10 and Q7 are absent: `transaction.rs` and `validate/supersession.rs` are untouched. A probe that runs the new index against the three 0.0.39 scans, over 2 000 generated record sets, agrees on every lookup. The sets include shapes no commit produces (an edge in several records, an edge both in the head and in a record, a record filed under another node's id, a mismatched edge key).

**1. `git --no-pager diff --stat`:** empty, so no tracked file changed. `git status --short`:
```
?? crates/ekr-graph/tests/adversary_w20261020a_retired_edges.rs
?? crates/ekr/tests/adversary_w20261020a_extraction_notes.rs
```

**2. Cases added (first run, each target alone)**

| Case | Asserts | Now |
|---|---|---|
| `crates/ekr/tests/adversary_w20261020a_extraction_notes.rs` `the_verb_names_no_supersession_where_its_end_connects` | Acme is superseded by Alice. The fact {Organization acme} CEO_OF {Person bob}, against one store, gets the same refusal from `ekr apply-extraction` and from the SDK | red |
| same file, `strict_refuses_the_document_for_a_fact_its_group_lands_on_another_type` | Under `strict`, a fact refused because its group lands on Alice (a Person, no `legal_name`) refuses the whole document | red |
| `crates/ekr-graph/tests/adversary_w20261020a_retired_edges.rs` `the_retired_edge_index_answers_as_every_scan_it_replaced` | The D9 index equals the three 0.0.39 scans, copied verbatim from the base, on 2 000 seeded record sets | green |

Red output, verbatim (`red-extraction.log`):
```
thread 'the_verb_names_no_supersession_where_its_end_connects' panicked at crates/ekr/tests/adversary_w20261020a_extraction_notes.rs:249:9:
  left: "extraction-relation-ends: facts[0]: Person CEO_OF Person; the Organization known as [\"acme\"] resolved through the supersession of node 00000000-0000-4000-8000-000000000303 (type Organization 00000000-0000-4000-8000-000000000202) by node 00000000-0000-4000-8000-000000000301 (type 00000000-0000-4000-8000-000000000201), which is of another type"
 right: "extraction-relation-ends: facts[0]: Person CEO_OF Person"
thread 'strict_refuses_the_document_for_a_fact_its_group_lands_on_another_type' panicked at crates/ekr/tests/adversary_w20261020a_extraction_notes.rs:279:9:
file: `strict` skipped a fact the SDK refused before writing and applied the rest: Ok(ExtractionReport { committed: [], rejected: [RejectedExtraction { item: "facts[0]", ..., refusal: Some("extraction-property-undeclared: Person.legal_name; the Organization known as [\"acme-holdings\", \"acme\"] resolved through the supersession of node ...0303 ... by node ...0301 ...") }], ... })
test result: FAILED. 0 passed; 2 failed
```
Both cases stop on the file provider, so the sqlite pass was not reached. Both files are rustfmt-formatted, and clippy with `-D warnings` gives exit 0 on each target.

**3. Suite run (after the cases existed), `suite.sh`**
- Lane 3 (`cargo test --locked -p ekr-graph -p ekr-kernel -p ekr-store -p ekr-integrate -p ekr-views -p ekr-sdk --no-fail-fast`): 1446 passed, 0 failed, 11 ignored, `LANE3_EXIT=0`.
- Lane 4 (the brief's list plus `--test adversary_w20261020a_extraction_notes`): 143 passed, 2 failed, `error: 1 target failed: -p ekr --test adversary_w20261020a_extraction_notes`, `LANE4_EXIT=101`.
- The `before` count of 1588 comes from the implementor's `c1-lane3.log` (1445) plus `c1-lane4.log` (143).

**4. Findings**

| # | file:line | severity | verdict | origin | what was measured / what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr-sdk/src/extraction.rs:1195 | warning | NEEDS-CHANGE | introduced | **Measured:** the red case at :249. `named_by_cause` notes a supersession on an end where `!connects(own) \|\| !connects(landed)`. The engine reader now types the end by the landed node, which connects, so the end did not cause the refusal. The SDK's own `ends_connect` gives no note. **Reaches it:** `ekr apply-extraction` without `--strict`, for any relation with a superseded end that connects while the other end fails. **Fix:** note only on `!connects(&landed.node_type)`. **Origin:** from reading base 0.0.39 with `git show 01830917d0`, both surfaces typed by the key's own type and both noted, so they agreed. I did not run the base. |
| F2 | crates/ekr-sdk/src/extraction.rs:744 (doc :36) | note | CONFIRMED | introduced | **Measured:** the red case at :279. A refusal from the group check goes into `skipped` even under `strict`, while the same refusal on a single key refuses the document (`Plan::of`, strict branch). The new module doc exempts only `replacement-without-active-assertion` from `strict`. **Reaches it:** `strict` / `--strict` on a document where two named things share an alias and one alias belongs to a superseded node. **Nothing regressed:** 0.0.39 also applied the rest of this document, through a kernel rejection. The contradiction is between the new doc sentence and the code. |
| F3 | crates/ekr-graph/src/supersession.rs:96 | note | INFEASIBLE | introduced | **Measured:** by reading only. The D9 counter counts only reads through `retired_with` and `RetiredEdges::of`. A plain scan of `graph.superseded`, the 0.0.39 shape at `sample.rs:213`, is not counted. `a_sample_reads_each_supersession_record_once` stays red under a revert only because the pointer lookup goes through `retired_with`. **Why infeasible:** a mutated copy build needs a target the size of this tree's (27G), with 49–72G free and the watch stopping builds at 20G. |

**5. Attacked, could not break**
- **D1:** both node-detail formats read `Index.referencing`, and /2 also filters on `node_index`. The unit's test covers a node subject and an edge subject.
- **D2:** dropping cache entries by answer value covers a chain (records are repointed), cross-type supersession, and A→B plus B→C in one batch. An unobserved supersession is a foreign revision, cleared at the next flush (the existing contract). Checked by reading only.
- **D3:** roles and the timeline agree for node subjects.
- **D6, no orphan node:** the per-key check, the group check and the reader's refusals all happen before anything is resolved. A fact the kernel refuses after a supersession lands on a node that already exists. No orphan path found.
- **T1–T7:** reading each test against its named mutant, alone, each one kills it. For T6, the v3 lookup makes the one-record count able to fail. The implementor's `mutants.log` applied all 8 mutants at once, before D7/D8/D10/Q7 were taken out, so it does not isolate them.

**6. Paths written outside the worktree:** the directory `<home>/.cache/ekr-w20a-u1-scratch/adversary-1/` and its seven files: `suite.sh`, `red-extraction.log`, `d9-probe.log`, `clippy-graph.log`, `clippy-ekr.log`, `suite-lane3.log`, `suite-lane4.log`. sccache entries also went to the shared `<home>/.cache/sccache/epistemic-knowledge-runtime` through `pinned-task.sh`. The session lease is released.

**7.**
```findings
[
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 1195, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "named_by_cause notes a supersession on a relation end whose own type does not connect though its landed type does, so ekr apply-extraction names a supersession the SDK correctly omits for the same fact (red: crates/ekr/tests/adversary_w20261020a_extraction_notes.rs:249)"},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 744, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "under ApplyOptions::strict a kept-type refusal from the group re-check is skipped and the rest applied, while the per-key one refuses the document and the module doc at :36 exempts only replacements (red: crates/ekr/tests/adversary_w20261020a_extraction_notes.rs:279)"},
  {"file": "crates/ekr-graph/src/supersession.rs", "line": 96, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the D9 read counter sees only lookups through retired_with and RetiredEdges::of, so a plain scan of graph.superseded like 0.0.39's sample.rs subject_type passes the counted tests; not provable here without a mutated copy build"}
]
```
