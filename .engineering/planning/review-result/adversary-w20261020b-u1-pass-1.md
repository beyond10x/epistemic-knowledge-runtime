---
format: aep.planning-md/3
id: review-result:adversary-w20261020b-u1-pass-1
kind: review-result
status: active
title: Adversary pass 1, wave 2026-10-20b U1 (MergeEntity)
relations:
- reviews: story:merge-entity-applies
revision: 1
---
Adversary pass 1 (`aep:adversary`) on wave 2026-10-20b unit U1, `story:merge-entity-applies`, tree
`ekr-w20b-u1` at `05bd70503`, recorded by the coordinator as it returned below.

Owners: 6 findings, 4 implementor (F1, F2, J3, J4), 2 coordinator (J1, J2: neither ADR 0018 nor the
coordinator's decisions said what explain shows for a rewritten assertion or what becomes of a
relation assertion joining absorbed and into).

---

unit: story:merge-entity-applies (wave 2026-10-20b U1). Tree ekr-w20b-u1 at 05bd70503 (base 2a843eb60), plus 2 untracked adversary test files
verdict: NEEDS-CHANGE
cases: executed 2573→2576, red 3
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3
needs-coordinator: J1 and J2. Should `ekr explain` link the merge that rewrote an assertion? Should a relation assertion joining absorbed and into be rewritten into a self-relation, retired, or refused?

**1. Diff stat**

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short`:
```
?? crates/ekr-views/tests/adversary_w20261020b_merge_changes.rs
?? crates/ekr/tests/adversary_w20261020b_merge_cli.rs
```
Both are test files; no implementation file was touched. A probe file, `crates/ekr-kernel/tests/adversary_w20261020b_merge_probe.rs`, was in the tree briefly. It passed, so I moved it to scratch. Its test binary is still in `target/`.

**2. Cases added. All three are red now; each red run below is the case run alone, before the suite**

| file | case | asserts |
|---|---|---|
| `crates/ekr-views/tests/adversary_w20261020b_merge_changes.rs` | `adv_a_change_list_held_from_before_a_merge_answers_as_one_derived_after_it` | An IndexCache that read the changes at revision 2 answers the same bytes at revision 3 as a fresh `Index::load` does |
| same | `adv_a_retraction_before_a_merge_is_listed_about_the_subject_it_retracted` | Revision 2's AssertionRetracted names the same subject whether it is read at revision 2 or at revision 3 |
| `crates/ekr/tests/adversary_w20261020b_merge_cli.rs` | `adv_a_merged_and_a_superseded_duplicate_do_not_compare_equal` | One store merges `dup` into Alice and the other supersedes it by her; their knowledge roots differ, so `ekr compare` must not answer `equal: true` |

Red run, `pinned.sh cargo test --locked -p ekr-views --test adversary_w20261020b_merge_changes` (the long JSON of `left`/`right` is cut at `[…]`):
```
panicked at crates/ekr-views/tests/adversary_w20261020b_merge_changes.rs:219:9:
assertion `left == right` failed: file: the same ChangesSince at revision 3 answers differently from a session that held revision 2's list (retraction about "00000000-0000-4000-8000-000000000302") and from a fresh read (about "00000000-0000-4000-8000-000000000301")
  left: "{…\"change\":\"AssertionRetracted\",…\"subject\":\"00000000-0000-4000-8000-000000000302\",…[…]"
 right: "{…\"change\":\"AssertionRetracted\",…\"subject\":\"00000000-0000-4000-8000-000000000301\",…[…]"
panicked at crates/ekr-views/tests/adversary_w20261020b_merge_changes.rs:251:9:
assertion `left == right` failed: file: revision 2's change, read at revision 3, names the node the merge rewrote the subject to
  left: String("00000000-0000-4000-8000-000000000301")
 right: String("00000000-0000-4000-8000-000000000302")
test result: FAILED. 0 passed; 2 failed
```
Red run, `pinned.sh cargo test --locked -p ekr --test adversary_w20261020b_merge_cli`:
```
panicked at crates/ekr/tests/adversary_w20261020b_merge_cli.rs:197:9:
… their roots differ ({…"knowledge_root":"ae20a05303b3be7d264cf2ca080cedb55e632837a0101292e33a6528ff52e932"…} / {…"knowledge_root":"3fa1bdb5343a9458de6e243a5eaec39007faa668638c35251113a90355bdcc87"…}), and ekr compare answers that they hold the same knowledge: {…{"category":"superseded","differences":0,…"equal":true…}],"equal":true,…}
  left: Bool(true)
 right: false
```

**3. Suite run (after the cases existed)**

- Command: `pinned.sh cargo test --locked --workspace --no-fail-fast`. Log: `<home>/.cache/ekr-w20b-scratch/adversary-1/suite-after.log`.
- Result: EXIT=101. Passed 2573, failed 3, ignored 13, executed 2576.
- Only two targets failed: `-p ekr --test adversary_w20261020b_merge_cli` and `-p ekr-views --test adversary_w20261020b_merge_changes`. That run included `ekr-ontology`.
- How I got "before" = 2573: it is 2576 minus my 3 cases, which sit in 2 new test targets of their own. That is arithmetic, not a second run, and not one of the two sources the charter names.
- Also run: `cargo fmt --all -- --check` exit 0, and `cargo clippy -p ekr-views -p ekr --tests -D warnings` exit 0.

Rule 1 (a bare merge keeps its 0.0.40 verdict) and rule 2 (a store that never merges keeps its bytes) both hold in this run:
- `base_era_v1_replay`: 4 ok. The file and its sqlite fixture are not in the diff.
- `merge_entity a_bare_merge_keeps_its_retained_verdict`: ok.
- `current_root_sensitivity every_supersession_record_field_reaches_only_the_knowledge_root_and_its_root_is_pinned`: ok. Its pinned roots `a743d661…` and `2b0554cd…` are byte-identical to the base (`git show 2a843eb60:…:465-466`).

**4. Findings (cover 05bd70503 plus the two test files)**

| # | file:line | severity | verdict | origin | finding | what reaches it |
|---|---|---|---|---|---|---|
| F1 | `crates/ekr-views/src/changes.rs:539` (also `:526`) | blocker | NEEDS-CHANGE | introduced | A retracted or superseded assertion's change reads its subject from the graph at the read revision. A merge rewrites that subject, so revision 2's change names the node kept when read after the merge. Change lists are held per revision, so the same request at the same revision returns different bytes and `changes_hash`, depending on cache history. This breaks the module's own claim (`:33-37`) and views.yaml:1958 ("read off the transaction it committed"). Fix: list the subject the assertion held at the change's own revision. The record lineage holds that prior subject. | The IndexCache held by `ekr mcp` (`crates/ekr/src/cli/mcp.rs:192-201`, `:546`) and by sessions (`crates/ekr/src/cli/session.rs:141`). Any assertion about a duplicate that is retracted or superseded before the duplicate is merged. |
| F2 | `crates/ekr/src/cli/compare.rs:213` | warning | CONFIRMED | introduced | `superseded()` still compares only the six § 110 fields of a record, not `merge`. A merged store and a superseded store with different knowledge roots compare `equal: true`. Fix: add the record's `merge` to the compared entry. | `ekr compare --other` ("whether two stores hold the same knowledge", docs/cli.md:170) |
| J1 | `crates/ekr-kernel/src/explain.rs:302` | note | CONFIRMED (by reading, not run) | introduced | Explaining a merge-rewritten assertion gives: the assertion link naming `into`, the origin proposal's AddAssertion naming `absorbed`, and no link to the merge. The merge's evidence is not in the explanation's support. | `ekr explain`, MCP explain |
| J2 | `crates/ekr-kernel/src/merge.rs:342` | note | CONFIRMED (by reading) | introduced | An edge joining absorbed and into is retired (`:201`), but a relation assertion `absorbed R into` is rewritten to `into R into`. ADR 0018 open question 4 decided this for edges only. | Any relation assertion between the two nodes |
| J3 | `crates/ekr-kernel/src/validate/merge.rs:874` | note | CONFIRMED | introduced | No test checks `property-cardinality` for a moved property. `wrong-type` is checked for neither moved values (`:685`) nor edge values (`:669`); its only case is a node value (`merge_entity.rs:1390`). Both refusals are documented. My probe of `property-cardinality` passed against the real code (copy in scratch). | Child types that redeclare a property with another cardinality or value type |
| J4 | `CHANGELOG.md:27` | note | CONFIRMED | introduced | `EntityMerge::new` in the SDK now takes four arguments, and both `EntityMerge` types lost `Copy` (`crates/ekr-sdk/src/document/transaction.rs:487`, `crates/ekr-kernel/src/transaction.rs:145`). These are breaking API changes, but they are listed under "Added". The file's convention is "**Breaking:**" under "### Changed" (`CHANGELOG.md:982`). | SDK consumers |

**5. Attacked and could not break**

- **Rule 1:** every new path skips the bare shape (`structural.rs:1145`, `supersession.rs:117`, `reference.rs:197`, `merge::shaped`). The `cite` message changes only when a shaped merge is present.
- **Rule 2:** the pinned record roots pass, and `merge` is left out of serde and `Canonical` output when absent.
- **Encoding:** the bare and shaped merge encodings cannot collide. The `Some`/`None` tags (0x0c/0x0b) differ from the variant tag (0x0f).
- **Validation and commit agree:** both use the same `fates` function. CreateNode/CreateEdge with empty values, UpdateProperty, DeleteEdge and SupersedeNode in the merge's transaction are handled the same way by both (by reading).
- **Probe, both providers (green):** lineage with node values, edge values, a nested Record NodeRef and a collapsed edge's subject survives a checkpoint reopen, a full replay and a migration.
- **Probe, both providers (green):** after a collapse, deleting the kept edge, which now carries the moved assertion, is refused as `unresolved-edge`. This holds in the committing process and after migration.
- **Chains:** `supersede()` repoints merge records (`apply.rs:366-370`), and a merge repoints supersession records.
- **Cardinality** is counted after the collapse, including edges created from `into` in the same transaction.
- **Reads:** describe /1 and /2, expand and timeline refuse the absorbed node. The resolver cache drops answers naming it (`resolve.rs:220`). Alias resolution goes through the record.

**6. Paths written outside the worktree**
- `<home>/.cache/ekr-w20b-scratch/adversary-1/src.diff`
- `<home>/.cache/ekr-w20b-scratch/adversary-1/suite-after.log`
- `<home>/.cache/ekr-w20b-scratch/adversary-1/adversary_w20261020b_merge_probe.rs.green`

The worktree CLI also recorded my session lease (`adversary-w20b-u1-pass1`). I released it.

**7. Findings block**
```findings
[
  {"file": "crates/ekr-views/src/changes.rs", "line": 539, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A retracted or superseded assertion's change takes its subject from the read revision, which a later merge rewrites, so ChangesSince answers different bytes for one request at one revision depending on whether a cached list is held."},
  {"file": "crates/ekr/src/cli/compare.rs", "line": 213, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "ekr compare ignores a supersession record's merge lineage, so a merged store and a superseded store with different knowledge roots compare equal."},
  {"file": "crates/ekr-kernel/src/explain.rs", "line": 302, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The explanation of a merge-rewritten assertion has no link to the merge, so its assertion names into while its origin's AddAssertion names absorbed, and the merge's evidence is not in its support."},
  {"file": "crates/ekr-kernel/src/merge.rs", "line": 342, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A relation assertion joining absorbed and into is rewritten to a self-relation on into, while the joining edge is retired; ADR 0018 decided this for edges only."},
  {"file": "crates/ekr-kernel/src/validate/merge.rs", "line": 874, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No test checks property-cardinality for a moved property, or wrong-type for moved or edge values, though both refusals are documented."},
  {"file": "CHANGELOG.md", "line": 27, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The SDK EntityMerge::new arity change and the loss of Copy on both EntityMerge types are breaking changes listed under Added, not marked Breaking under Changed."}
]
```
