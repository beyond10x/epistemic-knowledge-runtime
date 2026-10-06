---
format: aep.planning-md/3
id: review-result:adversary-extraction-parity-unit-a-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: EKR wave 20261005b unit A'
relations:
- reviews: story:extraction-partial-apply
- reviews: story:extraction-valid-time
- reviews: story:extracted-relations-visible-to-graph-reads
- reviews: story:extraction-supersession
revision: 1
---
Unit A stays red after the correction: all 7 pass-1 findings are fixed with no regression, but I found 3 new defects in the corrected code, each with a failing test.

```
unit: A of wave 20261005b (the four extraction stories), correction pass 1, uncommitted on base 47f17ec94 in ekr-wx-a
verdict: NEEDS-CHANGE
cases: executed 2237→2240, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, ~/.cache/ekr-wave-8c9138d1/a/adversary-2/ (part 6); build dir reused, nothing deleted
needs-coordinator: disk is down to 11 GB free after the gate; my 200-fact case takes 127-162 s per run and can be cut to 20 facts (still red, 21 against 2)
```

**1. Diff stat.** `git --no-pager diff --stat` reads `20 files changed, 2654 insertions(+), 323 deletions(-)`. That is the implementor's correction, not touched by me. My only addition is one untracked test file, `crates/ekr/tests/adversary_wx_a_2.rs`. No non-test path changed.

**2. Cases** (headed "ADVERSARY CASES"; each run alone first, logs are `adversary-2/red-<name>.log`; all red now):

| Case | Asserts | Red output (solo run, verbatim) |
|---|---|---|
| `a_held_replacement_whose_supersession_is_rejected_is_reported_once` | The store holds two legal names for Globex, and the second starts later. Re-applying the known document with `replaces` must give `facts[0]` exactly one outcome. | `facts[0] has 2 outcomes in the report`: it is in `held` as `asserted` and in `rejected` with `invalid-supersession`. `left: 2 right: 1` |
| `corroborating_facts_of_one_claim_commit_in_one_transaction` | 200 facts make one claim (Polaris = "Polaris Ltd"), each citing its own evidence. They should commit as one node transaction plus one fact transaction. | `transactions committed for 200 corroborating facts`, `left: 201 right: 2`, 162.09 s |
| `strict_help_and_strict_behaviour_agree_on_a_replacement_with_nothing_to_replace` | Under `--strict`, a document with a replacement that has nothing to replace either exits 2 with nothing written, or `--help` mentions the exception. | `--strict exited 0 and wrote (Vela: Some("01a10e91-…")), but --help says: …` |

**3. Suite, run after the cases existed.**
- `task check > adversary-2/gate.log`: formatting and clippy passed, then `task: Failed to run task "test": exit status 101`, `EXIT=201`.
- `cargo test --workspace --locked --no-fail-fast > adversary-2/suite-nofailfast.log`: `EXIT=101`, 361 `test result:` lines, 2240 run (2237 passed, 3 failed, 13 ignored). The only failing target is ``-p ekr --test adversary_wx_a_2``.
- "Before" (2237) is the same step in the implementor's correction gate `fix-1/gate.log`, which exited 0.

**4. Findings.**
- **Pass-1 findings: all 7 fixed.**
  - My 6 cases were moved into `extraction_cli.rs` with their assertions unchanged. I compared each body after normalising helper names; only the helper renames differ.
  - The new test for finding 7 asserts real values and could fail.
  - All 7 pass in the correction gate and in my run.
- **Held replacement reported twice** (`crates/ekr-sdk/src/extraction.rs:1187`, introduced).
  - The fact is added to `held` before its supersede-only group is tried. When validation rejects that group, the fact is also in `rejected`.
  - Reached by re-applying a `replaces` document while the other active value starts later than the held one.
  - Fix: add to `held` only once that group commits.
- **Over-deferral of corroborating facts** (`extraction.rs:1145`, introduced).
  - Any fact with a claim already made in the round waits for the next round. That includes facts that cannot depend on each other: a different-evidence property corroboration is asserted whatever the earlier fact's outcome.
  - Rounds therefore equal the largest number of facts sharing one claim. Probe (`adversary-2/probe/probe.log`, file backend, counted and timed):

    | Facts | Distinct claims | Transactions | Seconds |
    |---|---|---|---|
    | 1,000 | 1,000 | 2 | 4 |
    | 1,000 | 500 | 3 | 4 |
    | 1,000 | 100 | 11 | 6 |
    | 200 | 1 | 201 | 162 |
    | 400 | 1 | 401 | 513 |

  - Time grows faster than linearly with repeats of one claim. 1,000 facts of one claim: not measured.
  - Before the rounds the 200-fact document would have been 1 fact transaction. That is reasoned from the batcher's 10,000-operation cap, not run.
  - Fix: defer a same-claim fact only when it could be held by the earlier fact (its evidence is a subset of the earlier fact's) or is a relation that needs the earlier fact's edge result.
- **`--strict` help text** (`crates/ekr/src/cli/mod.rs:231`, introduced).
  - It says "Refuse the whole document (exit 2, nothing written) on its first bad fact".
  - `docs/cli.md:380` and the SDK module doc say a replacement with nothing to replace is a per-fact row "with or without `--strict`", and the verb behaves that way: exit 0, the rest written.
  - So the answer to your strict question: yes, it is a per-fact row under `--strict`, consistent with `docs/cli.md`, and only `--help` disagrees.

**5. Attacked and could not break.**
- **Termination:** at line 1145 a fact is deferred only on state from the current round. The first pending fact of each round sees an empty round and is always planned, so the pending list shrinks every round. That holds for long chains and for A-then-B-then-A replacement sequences (reasoned, not run).
- **Early commits survive a later failure:** each round's commits go into the report before the next round. A fault stops the run with earlier commits kept, and a fault leaves untried groups unlisted, as at base (`batch.rs:316`). Read, not run.
- **Round atomicity:** groups are atomic and rounds are not. The docs claim only group atomicity, so this matches the documentation.
- **Plan-time "nothing to replace":** `holds` matches what `ekr resolve` answers (exact type, alias match; `ekr-integrate/src/lib.rs:209-217`).
- **Realistic 1,000-fact documents** commit in 2 to 11 transactions within 6 s.

**6. Paths written outside the worktree.** All under `~/.cache/ekr-wave-8c9138d1/a/adversary-2/`:
- `tmp/`, `build.log`, three `red-*.log`, `gate.log`, `suite-nofailfast.log`;
- `helpers.rs` (scratch extract, unused);
- `probe/probe.sh`, `probe/probe.log`, and four probe stores `probe/w1000x1000`, `probe/w1000x500`, `probe/w1000x100`, `probe/w400x1` (53 MB+).

Build dir `~/.cache/b10x-target/ekr-wx-a` was reused. My lease `ekr-wx-a-adversary-2` was acquired late (after the cases ran) and has been released.

**7. Findings block.**
```findings
[
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 1187, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A held replacement is listed under held before its supersede group is tried, so when validation rejects that group the same fact is also listed under rejected."},
  {"file": "crates/ekr-sdk/src/extraction.rs", "line": 1145, "category": "property", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Every fact repeating a claim already made in the round waits a round even when it cannot depend on the earlier fact, so N corroborations of one claim commit N transactions (200 facts: 201 transactions in 162 s; 400 facts: 401 in 513 s)."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 231, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The --strict help promises exit 2 with nothing written on any bad fact, but a replacement with nothing to replace is a per-fact row under --strict, as docs/cli.md says."}
]
```